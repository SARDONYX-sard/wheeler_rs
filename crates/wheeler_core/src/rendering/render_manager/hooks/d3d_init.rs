use core::sync::atomic::AtomicBool;
use std::sync::OnceLock;

use commonlibsse_ng::rel::{id::RelocationID, offset::VariantOffset};
use commonlibsse_ng_re_ext::re::BSRenderManager::BSRenderManager;

use crate::{
    hook::Hook,
    rendering::render_manager::{
        CONTEXT, DEVICE, RenderManagerError,
        hooks::wnd_proc::{WND_PROC_ORIGINAL_FN, WndProcHook},
    },
};

macro_rules! unwrap_ptr_or_return {
    ($raw:expr, $name:ident) => {
        let $name = match unsafe { commonlibsse_ng::rel::relocation::raw_pointer_as_ref(*$raw) } {
            Ok(val) => val,
            Err(err) => {
                tracing::error!(
                    "[D3DInitHook Error]: Invalid pointer `{}`: {}",
                    stringify!($name),
                    err
                );
                return;
            }
        };
    };
}

pub(crate) static D3D_INIT_HOOK_INITIALIZED: AtomicBool = AtomicBool::new(false);
static D3D_INIT_ORIGINAL_FN: OnceLock<fn()> = OnceLock::new();
pub(crate) struct D3DInitHook;
impl D3DInitHook {
    pub fn hook_start() -> Result<(), RenderManagerError> {
        let original_fn = unsafe {
            let original_fn = crate::hook::write_thunk_call::<D3DInitHook, VariantOffset>()?;
            core::mem::transmute::<*const (), fn()>(original_fn)
        };
        D3D_INIT_ORIGINAL_FN
            .set(original_fn)
            .map_err(|_| RenderManagerError::FailedToSetD3dInitOriginalFn)
    }

    fn thunk() {
        if let Some(f) = D3D_INIT_ORIGINAL_FN.get() {
            f()
        }

        tracing::info!("RenderManager: Initializing...");
        let Some(render_manager) = BSRenderManager::get_singleton() else {
            tracing::error!("[D3DInitHook Error]: Couldn't to get singleton of `BSRenderManager`");
            return;
        };

        let commonlibsse_ng_re_ext::re::BSRenderManager::RUNTIME_DATA {
            swapChain: swap_chain,
            forwarder,
            context,
            ..
        } = match render_manager.get_runtime_data() {
            Ok(runtime_data) => runtime_data,
            Err(err) => {
                tracing::error!(
                    "[D3DInitHook Error]: Couldn't to get runtime data of `BSRenderManager`: {err}"
                );
                return;
            }
        };
        unwrap_ptr_or_return!(swap_chain, swap_chain);
        unwrap_ptr_or_return!(forwarder, forwarder);
        unwrap_ptr_or_return!(context, context);

        let sd = match unsafe { swap_chain.GetDesc() } {
            Ok(sd) => sd,
            Err(err) => {
                tracing::error!("IDXGISwapChain::GetDesc failed.: {err}");
                return;
            }
        };

        let _ = DEVICE.set(forwarder);
        let _ = CONTEXT.set(context);
        let output_window = sd.OutputWindow;
        imgui::Context::create(); // FIXME: panic double call

        // TODO: DX11 backend for imgui
        // if !ImGui_ImplWin32_Init(output_window) {
        //     tracing::error!("ImGui initialization failed (Win32)");
        //     return;
        // }
        // if !ImGui_ImplDX11_Init(forwarder, context) {
        //     tracing::error!("ImGui initialization failed (DX11)");
        //     return;
        // }

        #[allow(clippy::fn_to_numeric_cast)]
        unsafe {
            use crate::rendering::render_manager::hooks::wnd_proc::WndProc;
            use windows::Win32::UI::WindowsAndMessaging::{GWLP_WNDPROC, SetWindowLongPtrA};
            let _ = WND_PROC_ORIGINAL_FN.set(core::mem::transmute::<isize, WndProc>(
                SetWindowLongPtrA(output_window, GWLP_WNDPROC, WndProcHook::thunk as isize),
            ));
        }

        load_custom_font();
    }
}
impl Hook<VariantOffset> for D3DInitHook {
    const ID: RelocationID = RelocationID::from_se_ae_id(75595, 77226);
    const OFFSET: VariantOffset = VariantOffset::new(0x9, 0x275, 0x9); // FIXME: The `VR` offset is unknown.
    const REPLACED_FN: *const () = Self::thunk as *const ();
}

fn load_custom_font() {
    const FONT_DIR: &str = "Data/SKSE/Plugins/wheeler/resources/fonts";
    const FONT_INI_PATH: &str = "Data/SKSE/Plugins/wheeler/resources/fonts/FontConfig.ini";
    let font_ini = std::path::Path::new(FONT_INI_PATH);
    if !font_ini.exists() {
        return;
    }

    // TODO: parse ini
    let language = "Japanese";
    let font_dir = format!("{FONT_DIR}/{language}");
    let font_file = std::fs::read_dir(&font_dir).ok().and_then(|entries| {
        entries
            .filter_map(Result::ok)
            .find(|entry| {
                entry.path().extension().is_some_and(|ext| {
                    ext.eq_ignore_ascii_case("ttf") || ext.eq_ignore_ascii_case("ttc")
                })
            })
            .map(|entry| entry.path())
    });

    if let Some(font_path) = font_file {
        let Some(imgui_io) = (unsafe { imgui::sys::igGetIO().as_ref() }) else {
            tracing::error!("Failed to get `imgui::sys::igGetIO()");
            return;
        };
        let fonts = imgui_io.Fonts;

        let glyph_ranges = unsafe {
            match language {
                "Chinese" => imgui::sys::ImFontAtlas_GetGlyphRangesChineseFull(fonts),
                "Korean" => imgui::sys::ImFontAtlas_GetGlyphRangesKorean(fonts),
                "Japanese" => imgui::sys::ImFontAtlas_GetGlyphRangesJapanese(fonts),
                "Thai" => imgui::sys::ImFontAtlas_GetGlyphRangesThai(fonts),
                "Vietnamese" => imgui::sys::ImFontAtlas_GetGlyphRangesVietnamese(fonts),
                "Cyrillic" => imgui::sys::ImFontAtlas_GetGlyphRangesCyrillic(fonts),
                unsupported_lang => {
                    tracing::error!(
                        "Unsupported custom font language: {unsupported_lang}, Expected one of `Chinese`, `Korean`, `Japanese`, `Thai`, `Vietnamese`, `Cyrillic`",
                    );
                    return;
                }
            }
        };

        let font_path = font_path.to_string_lossy();
        match std::ffi::CString::new(font_path.as_ref()) {
            Ok(font_path) => unsafe {
                imgui::sys::ImFontAtlas_AddFontFromFileTTF(
                    fonts,
                    font_path.as_ptr(),
                    64.0,
                    core::ptr::null(),
                    glyph_ranges,
                );
            },
            Err(_) => {
                tracing::error!("Failed to set custom font. null bytes include in `{font_path}`")
            }
        };
    }
}
