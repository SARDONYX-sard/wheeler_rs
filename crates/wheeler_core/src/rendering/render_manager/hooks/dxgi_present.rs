use core::sync::atomic::{self};
use std::sync::OnceLock;

use commonlibsse_ng::rel::{id::RelocationID, offset::Offset};

use crate::hook::Hook;
use crate::rendering::render_manager::RenderManagerError;

use super::d3d_init::D3D_INIT_HOOK_INITIALIZED;

static DXGI_PRESENT_ORIGINAL_FN: OnceLock<fn(u32)> = OnceLock::new();
pub(crate) struct DXGIPresentHook;
impl DXGIPresentHook {
    pub fn hook_start() -> Result<(), RenderManagerError> {
        let original_fn = unsafe {
            let original_fn = crate::hook::write_thunk_call::<DXGIPresentHook, Offset>()?;
            core::mem::transmute::<*const (), fn(u32)>(original_fn)
        };
        DXGI_PRESENT_ORIGINAL_FN
            .set(original_fn)
            .map_err(|_| RenderManagerError::FailedToSetDxgiPresentOriginalFn)
    }

    fn thunk(p1: u32) {
        if let Some(f) = DXGI_PRESENT_ORIGINAL_FN.get() {
            f(p1)
        }

        if !D3D_INIT_HOOK_INITIALIZED.load(atomic::Ordering::Acquire) {
            return;
        }

        unsafe {
            // prologue
            // TODO: ImGui_ImplDX11_NewFrame();
            // TODO: ImGui_ImplWin32_NewFrame();
            imgui::sys::igNewFrame();

            draw();

            // epilogue
            imgui::sys::igEndFrame();
            imgui::sys::igRender();
            // TODO: ImGui_ImplDX11_RenderDrawData(ImGui::GetDrawData());
        }
    }
}
impl Hook<Offset> for DXGIPresentHook {
    const ID: RelocationID = RelocationID::from_se_ae_id(75461, 77246);
    const OFFSET: Offset = Offset::new(0x9);

    const REPLACED_FN: *const () = Self::thunk as *const ();
}

fn draw() {
    if let Some(io) = unsafe { imgui::sys::igGetIO().as_ref() } {
        crate::wheeler::Wheeler::update(io.DeltaTime);
    }
}
