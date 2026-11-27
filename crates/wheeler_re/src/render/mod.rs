//! Port of `Rendering/RenderManager.cpp`: hooks the renderer init and the
//! DXGI present call, owns the ImGui context, and drives the per-frame draw.

use core::sync::atomic::{AtomicBool, AtomicIsize, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use imgui::{FontConfig, FontGlyphRanges, FontSource};
use libskyrim::re::Renderer;
use libskyrim::relocation::{RelocationID, VariantOffset};
use libskyrim::rex::com_call;
use libskyrim::rex::{DXGI_SWAP_CHAIN_DESC, succeeded};
use libskyrim::sdk::plugin;
use libskyrim::{skse_error, skse_message};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallWindowProcA, GWLP_WNDPROC, GetClientRect, SetWindowLongPtrA, WM_KILLFOCUS, WNDPROC,
};

use crate::hooks::write_thunk_call;

const FONT_DIR: &str = "Data\\SKSE\\Plugins\\wheeler\\resources\\fonts";
const FONT_SETTING_PATH: &str = "Data\\SKSE\\Plugins\\wheeler\\resources\\fonts\\FontConfig.ini";

/// Everything that lives on the render thread.
struct RenderState {
    imgui: imgui::Context,
    renderer: wheeler_dx11::Renderer,
    hwnd: HWND,
    last_frame: Instant,
}

// Only ever touched from the render thread (D3D init and Present hooks).
unsafe impl Send for RenderState {}

static STATE: Mutex<Option<RenderState>> = Mutex::new(None);
static INITIALIZED: AtomicBool = AtomicBool::new(false);

static D3D_INIT_ORIGINAL: AtomicUsize = AtomicUsize::new(0);
static PRESENT_ORIGINAL: AtomicUsize = AtomicUsize::new(0);
static WND_PROC_ORIGINAL: AtomicIsize = AtomicIsize::new(0);

/// C++ `D3DInitHook::{id, offset}`.
// TODO: the VR offset is unknown (C++ uses 0x00), and the AE offset must be
// re-checked on 1.7.99+; `write_thunk_call` refuses to patch a non-call site.
const D3D_INIT_ID: RelocationID = RelocationID::new(75595, 77226);
const D3D_INIT_OFFSET: VariantOffset = VariantOffset::new(0x9, 0x275, 0x0);
/// C++ `DXGIPresentHook::{id, offset}`.
const PRESENT_ID: RelocationID = RelocationID::new(75461, 77246);
const PRESENT_OFFSET: usize = 0x9;

pub fn is_initialized() -> bool {
    INITIALIZED.load(Ordering::Acquire)
}

/// C++ `RenderManager::Install`
pub fn install() {
    let base = D3D_INIT_ID.address();
    if base != 0 {
        write_thunk_call(
            "D3DInitHook",
            base + D3D_INIT_OFFSET.offset(),
            d3d_init_thunk as *const () as usize,
            &D3D_INIT_ORIGINAL,
        );
    }
    let base = PRESENT_ID.address();
    if base != 0 {
        write_thunk_call(
            "DXGIPresentHook",
            base + PRESENT_OFFSET,
            dxgi_present_thunk as *const () as usize,
            &PRESENT_ORIGINAL,
        );
    }
}

/// Height of the game window's client area, once rendering is initialized
/// (C++ uses the ImGui viewport for `Config::OffsetSizingToViewport`).
pub fn viewport_height() -> Option<f32> {
    let guard = STATE.lock().unwrap_or_else(|e| e.into_inner());
    let state = guard.as_ref()?;
    let mut rect: RECT = unsafe { core::mem::zeroed() };
    unsafe { GetClientRect(state.hwnd, &mut rect) };
    Some((rect.bottom - rect.top) as f32)
}

/// C++ `RenderManager::MessageCallback` (kDataLoaded) plus `Texture::Init`.
pub fn on_data_loaded() {
    let mut guard = STATE.lock().unwrap_or_else(|e| e.into_inner());
    let Some(state) = guard.as_mut() else {
        skse_error!("RenderManager: not initialized on data loaded");
        return;
    };
    let io = state.imgui.io_mut();
    io.mouse_draw_cursor = true;
    io.want_set_mouse_pos = true;

    drop(guard);
    if let Some(renderer) = unsafe { Renderer::get_singleton().as_ref() } {
        unsafe { crate::textures::init(renderer.device()) };
    }
}

unsafe extern "system" fn wnd_proc_thunk(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_KILLFOCUS {
        // C++ clears input characters/keys; Wheeler does not feed keys to
        // ImGui, so the IO has nothing to clear beyond that.
        if let Ok(mut guard) = STATE.try_lock()
            && let Some(state) = guard.as_mut()
        {
            let io = state.imgui.io_mut() as *mut imgui::Io as *mut imgui::sys::ImGuiIO;
            unsafe {
                imgui::sys::ImGuiIO_ClearInputCharacters(io);
                imgui::sys::ImGuiIO_ClearInputKeys(io);
            }
        }
    }
    let original = WND_PROC_ORIGINAL.load(Ordering::Acquire);
    unsafe {
        CallWindowProcA(
            core::mem::transmute::<isize, WNDPROC>(original),
            hwnd,
            msg,
            wparam,
            lparam,
        )
    }
}

/// C++ `D3DInitHook::thunk`
extern "C" fn d3d_init_thunk() {
    let original = D3D_INIT_ORIGINAL.load(Ordering::Acquire);
    unsafe { core::mem::transmute::<usize, extern "C" fn()>(original)() };

    skse_message!("RenderManager: Initializing...");
    if let Err(error) = init_render() {
        skse_error!("RenderManager: initialization failed: {error}");
        return;
    }
    skse_message!("RenderManager: Initialized");
}

fn init_render() -> Result<(), String> {
    let renderer =
        unsafe { Renderer::get_singleton().as_ref() }.ok_or("Cannot find render manager")?;
    let swap_chain = renderer.main_window().swap_chain;
    if swap_chain.is_null() {
        return Err("Cannot find swapchain".into());
    }

    let mut desc: DXGI_SWAP_CHAIN_DESC = unsafe { core::mem::zeroed() };
    let hr = com_call!(swap_chain, GetDesc: fn(*mut DXGI_SWAP_CHAIN_DESC) -> i32; &mut desc);
    if !succeeded(hr) {
        return Err(format!("IDXGISwapChain::GetDesc failed: {hr:#X}"));
    }
    let hwnd: HWND = desc.output_window;

    let mut imgui = imgui::Context::create();
    imgui.set_ini_filename(None);
    imgui.set_platform_name(Some(String::from("wheeler_re-win32")));
    load_custom_font(&mut imgui);

    let dx11 =
        unsafe { wheeler_dx11::Renderer::new(&mut imgui, renderer.device(), renderer.context())? };

    *STATE.lock().unwrap_or_else(|e| e.into_inner()) = Some(RenderState {
        imgui,
        renderer: dx11,
        hwnd,
        last_frame: Instant::now(),
    });
    INITIALIZED.store(true, Ordering::Release);

    let previous = unsafe {
        SetWindowLongPtrA(
            hwnd,
            GWLP_WNDPROC,
            wnd_proc_thunk as *const () as usize as isize,
        )
    };
    if previous == 0 {
        skse_error!("SetWindowLongPtrA failed!");
    } else {
        WND_PROC_ORIGINAL.store(previous, Ordering::Release);
    }
    Ok(())
}

/// C++ font block of `D3DInitHook::thunk`: loads the first `.ttf`/`.ttc` in
/// `fonts/<language>` named by `FontConfig.ini [config] font`.
// TODO: C++ builds the atlas with FreeType (light hinting); imgui-rs is used
// with the default stb_truetype rasterizer for now.
fn load_custom_font(imgui: &mut imgui::Context) {
    let Ok(ini) = plugin::load_ini(FONT_SETTING_PATH) else {
        return;
    };
    let cfg = plugin::config(&ini);
    let Some(language) = cfg.raw("config", "font").map(str::to_owned) else {
        return;
    };

    let dir = std::path::Path::new(FONT_DIR).join(&language);
    let font = std::fs::read_dir(&dir).ok().and_then(|entries| {
        entries.filter_map(Result::ok).map(|e| e.path()).find(|p| {
            p.extension().is_some_and(|ext| {
                ext.eq_ignore_ascii_case("ttf") || ext.eq_ignore_ascii_case("ttc")
            })
        })
    });
    let Some(font) = font else {
        skse_message!("No font found for language: {language}");
        return;
    };

    let glyph_ranges = match language.as_str() {
        "Chinese" => FontGlyphRanges::chinese_full(),
        "Korean" => FontGlyphRanges::korean(),
        "Japanese" => FontGlyphRanges::japanese(),
        "Thai" => FontGlyphRanges::thai(),
        "Vietnamese" => FontGlyphRanges::vietnamese(),
        "Cyrillic" => FontGlyphRanges::cyrillic(),
        "Polish" => FontGlyphRanges::from_slice(GLYPH_RANGES_POLISH),
        _ => FontGlyphRanges::default(),
    };

    match std::fs::read(&font) {
        Ok(data) => {
            skse_message!("Loading font: {}", font.display());
            // The atlas keeps referencing the TTF data; it lives for the process.
            let data: &'static [u8] = Box::leak(data.into_boxed_slice());
            imgui.fonts().add_font(&[FontSource::TtfData {
                data,
                size_pixels: 64.0,
                config: Some(FontConfig {
                    glyph_ranges,
                    ..FontConfig::default()
                }),
            }]);
        }
        Err(error) => skse_error!("Failed to read font {}: {error}", font.display()),
    }
}

/// C++ `FontLoader::GetGlyphRangesPolish`: the default Latin range plus the
/// Polish letters.
const GLYPH_RANGES_POLISH: &[u32] = &[
    0x0020, 0x00FF, // Basic Latin + Latin Supplement (includes Ó ó)
    0x0104, 0x0107, // Ą ą Ć ć
    0x0118, 0x0119, // Ę ę
    0x0141, 0x0144, // Ł ł Ń ń
    0x015A, 0x015B, // Ś ś
    0x0179, 0x017C, // Ź ź Ż ż
    0,
];

/// C++ `DXGIPresentHook::thunk`
extern "C" fn dxgi_present_thunk(p1: u32) {
    let original = PRESENT_ORIGINAL.load(Ordering::Acquire);
    unsafe { core::mem::transmute::<usize, extern "C" fn(u32)>(original)(p1) };

    if !is_initialized() {
        return;
    }
    let mut guard = STATE.lock().unwrap_or_else(|e| e.into_inner());
    let Some(state) = guard.as_mut() else {
        return;
    };

    // prologue (ImGui_ImplDX11_NewFrame / ImGui_ImplWin32_NewFrame)
    let now = Instant::now();
    let delta = now
        .duration_since(state.last_frame)
        .as_secs_f32()
        .max(1.0 / 1_000_000.0);
    state.last_frame = now;
    let mut rect: RECT = unsafe { core::mem::zeroed() };
    unsafe { GetClientRect(state.hwnd, &mut rect) };
    let io = state.imgui.io_mut();
    io.display_size = [
        (rect.right - rect.left) as f32,
        (rect.bottom - rect.top) as f32,
    ];
    io.delta_time = delta;

    let ui = state.imgui.new_frame();
    // do stuff (C++ `RenderManager::draw`)
    crate::wheeler::update(ui, delta);
    // TODO: TimeFloatInterpolatorManager::Update(deltaTime)

    // epilogue
    let draw_data = state.imgui.render();
    if let Err(error) = state.renderer.render(draw_data) {
        skse_error!("ImGui DX11 render failed: {error}");
    }
}
