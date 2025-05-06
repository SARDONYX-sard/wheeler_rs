use core::ffi::CStr;

pub struct DXGIPresentHook;

const WHEEL_WINDOW_ID: &CStr = c"##Wheeler_rs";

impl DXGIPresentHook {
    pub fn thunk(pl: u32) {
        unsafe {
            // prologue
            imgui::sys::igNewFrame();

            RenderManager::draw();

            // epilogue
            imgui::sys::igEndFrame();
            imgui::sys::igRender();
        }
    }
}

struct Wheeler;

impl Wheeler {
    fn update(delta_time: f32) {
        use imgui::sys::{
            ImVec2, igBeginPopup, igCloseCurrentPopup, igEndPopup, igIsPopupOpen,
            igSetNextWindowPos,
        };

        unsafe {
            igIsPopupOpen(WHEEL_WINDOW_ID.as_ptr(), 0);
            igSetNextWindowPos(ImVec2::new(-100.0, -100.0), 0, ImVec2::zero());
            igBeginPopup(WHEEL_WINDOW_ID.as_ptr(), 0);
            igCloseCurrentPopup();
            igEndPopup();
        }
    }
}

fn get_resolution_scale_width() -> Option<f32> {
    Some(unsafe { imgui::sys::igGetIO().as_ref()? }.DisplaySize.x / 1920.0)
}

fn get_resolution_scale_height() -> Option<f32> {
    Some(unsafe { imgui::sys::igGetIO().as_ref()? }.DisplaySize.y / 1080.0)
}

pub struct RenderManager {}

impl RenderManager {
    pub fn draw() {
        if let Some(io) = unsafe { imgui::sys::igGetIO().as_ref() } {
            let delta_time = io.DeltaTime;
            Wheeler::update(delta_time);
        }
    }
}
