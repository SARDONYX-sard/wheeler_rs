use std::sync::OnceLock;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};

pub(crate) type WndProc =
    unsafe extern "system" fn(h_wnd: HWND, u_msg: u32, w_param: WPARAM, l_param: LPARAM) -> LRESULT;

pub(crate) static WND_PROC_ORIGINAL_FN: OnceLock<WndProc> = OnceLock::new();
pub(crate) struct WndProcHook;
impl WndProcHook {
    pub unsafe extern "system" fn thunk(
        h_wnd: HWND,
        u_msg: u32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT {
        if let Some(f) = WND_PROC_ORIGINAL_FN.get() {
            return unsafe { f(h_wnd, u_msg, w_param, l_param) };
        }
        LRESULT(0)
    }
}
