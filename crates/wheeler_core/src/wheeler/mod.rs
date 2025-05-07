use core::ffi::CStr;
use core::sync::atomic;

pub const WHEEL_WINDOW_ID: &CStr = c"##Wheeler_rs";
static STATE: AtomicWheelState = AtomicWheelState::new(WheelState::Closed);

#[atomic_enum::atomic_enum]
#[derive(PartialEq)]
enum WheelState {
    Opened,
    Closed,
    Opening,
    Closing,
}

pub struct Wheeler {}

impl Wheeler {
    pub fn update(delta_time: f32) {
        use imgui::sys::{
            ImVec2, igBeginPopup, igCloseCurrentPopup, igEndPopup, igIsPopupOpen,
            igSetNextWindowPos,
        };

        if STATE.load(atomic::Ordering::Acquire) == WheelState::Closed {}

        unsafe {
            igIsPopupOpen(WHEEL_WINDOW_ID.as_ptr(), 0);
            igSetNextWindowPos(ImVec2::new(-100.0, -100.0), 0, ImVec2::zero());
            igBeginPopup(WHEEL_WINDOW_ID.as_ptr(), 0);
            igCloseCurrentPopup();
            igEndPopup();
        }
    }
}
