//! Opens a window and renders a mock wheel with the real DX11 backend.
//!
//! ```sh
//! cargo run -p wheeler_dx11 --example preview -- [path/to/wheeler/resources/icons]
//! ```
//! With an icon directory, the slot/wheel background SVGs are loaded from it;
//! otherwise geometric primitives are used. Move the mouse around the wheel
//! centre to hover entries; Esc or closing the window quits.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::ptr::{null, null_mut};

use rex_com::*;
use wheeler_core::sound::Sound;
use wheeler_core::style;
use wheeler_dx11::texture;
use wheeler_imgui::drawer::DrawArgs;
use wheeler_imgui::image::{IconImageType, Image};
use wheeler_imgui::wheel::{self, Wheel, WheelEntry, WheelHost, WheelItem};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryA};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_ESCAPE;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;

// ---------------------------------------------------------------------------
// Mock wheel content

struct PreviewItem {
    name: String,
    icon: Image,
}

impl WheelItem<()> for PreviewItem {
    fn draw_slot(&self, center: [f32; 2], _hovered: bool, _: &(), args: DrawArgs) {
        wheel::draw_slot_texture(self.icon, center, args);
        wheel::draw_slot_text(center, &self.name, args);
    }

    fn draw_highlight(&self, center: [f32; 2], _: &(), args: DrawArgs) {
        wheel::draw_highlight_texture(self.icon, center, args);
        wheel::draw_highlight_text(center, &self.name, args);
        wheel::draw_highlight_description(
            center,
            "Preview item rendered by wheeler_dx11 without Skyrim. Long descriptions wrap onto multiple lines.",
            args,
        );
    }

    fn is_active(&self, _: &()) -> bool {
        self.name.starts_with("Iron")
    }

    fn is_available(&self, _: &()) -> bool {
        true
    }

    fn activate_primary(&mut self, _: &()) {}

    fn activate_secondary(&mut self, _: &()) {}
}

struct PreviewHost {
    icons: HashMap<IconImageType, Image>,
    sounds: RefCell<Vec<Sound>>,
}

impl WheelHost<()> for PreviewHost {
    fn play_sound(&self, sound: Sound) {
        self.sounds.borrow_mut().push(sound);
    }

    fn icon(&self, icon: IconImageType) -> Image {
        self.icons.get(&icon).copied().unwrap_or(Image::NONE)
    }

    fn item_from_menu_hovered(&self) -> Option<Box<dyn WheelItem<()>>> {
        None
    }
}

fn load_icons(device: *mut ID3D11Device, dir: Option<PathBuf>) -> HashMap<IconImageType, Image> {
    let mut icons = HashMap::new();
    let Some(dir) = dir else {
        return icons;
    };
    for &icon in IconImageType::ALL {
        let path = dir.join(icon.file_name());
        match texture::rasterize_svg_file(&path)
            .and_then(|rgba| unsafe { texture::upload(device, &rgba) })
        {
            Ok(t) => {
                icons.insert(
                    icon,
                    Image {
                        texture: t.srv,
                        width: t.width,
                        height: t.height,
                    },
                );
            }
            Err(e) => eprintln!("skip {}: {e}", icon.file_name()),
        }
    }
    icons
}

// ---------------------------------------------------------------------------
// Win32 / D3D11 boilerplate

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            0
        }
        WM_KEYDOWN if wparam == VK_ESCAPE as usize => {
            unsafe { DestroyWindow(hwnd) };
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

type CreateDeviceAndSwapChain = unsafe extern "system" fn(
    *mut core::ffi::c_void,
    u32,
    *mut core::ffi::c_void,
    u32,
    *const u32,
    u32,
    u32,
    *const DXGI_SWAP_CHAIN_DESC,
    *mut *mut IDXGISwapChain,
    *mut *mut ID3D11Device,
    *mut u32,
    *mut *mut ID3D11DeviceContext,
) -> HRESULT;

fn main() {
    let icon_dir = std::env::args().nth(1).map(PathBuf::from);

    unsafe {
        let instance = GetModuleHandleW(null());
        let class = wide("wheeler_preview");
        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: instance,
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            lpszClassName: class.as_ptr(),
            ..core::mem::zeroed()
        };
        RegisterClassW(&wc);
        let title = wide("wheeler preview");
        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            WIDTH as i32,
            HEIGHT as i32,
            null_mut(),
            null_mut(),
            instance,
            null(),
        );

        let d3d11 = LoadLibraryA(c"d3d11.dll".as_ptr().cast());
        let create: CreateDeviceAndSwapChain = core::mem::transmute(
            GetProcAddress(d3d11, c"D3D11CreateDeviceAndSwapChain".as_ptr().cast())
                .expect("D3D11CreateDeviceAndSwapChain"),
        );
        let desc = DXGI_SWAP_CHAIN_DESC {
            buffer_desc: DXGI_MODE_DESC {
                width: 0,
                height: 0,
                refresh_rate: DXGI_RATIONAL {
                    numerator: 60,
                    denominator: 1,
                },
                format: DXGI_FORMAT_R8G8B8A8_UNORM,
                scanline_ordering: 0,
                scaling: 0,
            },
            sample_desc: DXGI_SAMPLE_DESC {
                count: 1,
                quality: 0,
            },
            buffer_usage: 0x20, // DXGI_USAGE_RENDER_TARGET_OUTPUT
            buffer_count: 2,
            output_window: hwnd,
            windowed: 1,
            swap_effect: 0, // DXGI_SWAP_EFFECT_DISCARD
            flags: 0,
        };
        let (mut swap_chain, mut device, mut context) = (null_mut(), null_mut(), null_mut());
        let hr = create(
            null_mut(),
            1,
            null_mut(),
            0,
            null(),
            0,
            7,
            &desc,
            &mut swap_chain,
            &mut device,
            null_mut(),
            &mut context,
        );
        assert!(
            succeeded(hr),
            "D3D11CreateDeviceAndSwapChain failed: {hr:#X}"
        );

        let mut back_buffer: *mut ID3D11Texture2D = null_mut();
        let hr = com_call!(swap_chain, GetBuffer: fn(u32, *const GUID, *mut *mut ID3D11Texture2D) -> HRESULT;
            0, &ID3D11Texture2D::IID, &mut back_buffer);
        assert!(succeeded(hr));
        let mut rtv: *mut ID3D11RenderTargetView = null_mut();
        let hr = com_call!(device, CreateRenderTargetView: fn(*mut ID3D11Texture2D, *const core::ffi::c_void, *mut *mut ID3D11RenderTargetView) -> HRESULT;
            back_buffer, null(), &mut rtv);
        assert!(succeeded(hr));
        com_release(back_buffer);

        let mut imgui = imgui::Context::create();
        imgui.set_ini_filename(None);
        let mut renderer =
            wheeler_dx11::Renderer::new(&mut imgui, device, context).expect("renderer");

        let icons = load_icons(device, icon_dir);
        if icons.is_empty() {
            let mut s = style::Style::DEFAULT;
            s.wheel.use_geometric_primitive_for_background_texture = true;
            style::set_style(s);
        }
        let item_icon = icons
            .get(&IconImageType::sword_one_handed)
            .copied()
            .unwrap_or(Image::NONE);
        let host = PreviewHost {
            icons,
            sounds: RefCell::default(),
        };

        let mut wheel: Wheel<()> = Wheel::new();
        for name in [
            "Iron Sword",
            "Fireball",
            "Health Potion",
            "Torch",
            "Shout",
            "Arrows",
        ] {
            let mut entry = WheelEntry::new();
            entry.push_item(Box::new(PreviewItem {
                name: name.to_owned(),
                icon: item_icon,
            }));
            wheel.push_entry(entry);
        }

        let mut last = std::time::Instant::now();
        let mut msg: MSG = core::mem::zeroed();
        'main: loop {
            while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_QUIT {
                    break 'main;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            let mut rect: RECT = core::mem::zeroed();
            GetClientRect(hwnd, &mut rect);
            let size = [
                (rect.right - rect.left) as f32,
                (rect.bottom - rect.top) as f32,
            ];
            let now = std::time::Instant::now();
            let dt = now.duration_since(last).as_secs_f32();
            last = now;

            let mut cursor: POINT = core::mem::zeroed();
            GetCursorPos(&mut cursor);
            windows_sys::Win32::Graphics::Gdi::ScreenToClient(hwnd, &mut cursor);
            let center = [size[0] / 2.0, size[1] / 2.0];
            let rel = [cursor.x as f32 - center[0], cursor.y as f32 - center[1]];
            let centered = (rel[0] * rel[0] + rel[1] * rel[1]).sqrt() < 20.0;

            let io = imgui.io_mut();
            io.display_size = size;
            io.delta_time = dt.max(1e-6);
            let ui = imgui.new_frame();
            ui.window("wheel")
                .position([0.0, 0.0], imgui::Condition::Always)
                .size(size, imgui::Condition::Always)
                .flags(
                    imgui::WindowFlags::NO_DECORATION
                        | imgui::WindowFlags::NO_BACKGROUND
                        | imgui::WindowFlags::NO_INPUTS,
                )
                .build(|| {
                    wheel.draw(
                        center,
                        rel[1].atan2(rel[0]),
                        centered,
                        dt,
                        &(),
                        &host,
                        DrawArgs::default(),
                    );
                });

            let clear = [0.12f32, 0.12, 0.14, 1.0];
            com_call!(context, OMSetRenderTargets: fn(u32, *const *mut ID3D11RenderTargetView, *mut core::ffi::c_void); 1, &rtv, null_mut());
            com_call!(context, ClearRenderTargetView: fn(*mut ID3D11RenderTargetView, *const f32); rtv, clear.as_ptr());
            renderer.render(imgui.render()).expect("render");
            com_call!(swap_chain, Present: fn(u32, u32) -> HRESULT; 1, 0);
        }

        drop(renderer);
        com_release(rtv);
        com_release(swap_chain);
        com_release(context);
        com_release(device);
    }
}
