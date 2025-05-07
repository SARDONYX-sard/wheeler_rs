use core::ptr;
use core::sync::atomic::{self, AtomicBool, AtomicPtr};

use commonlibsse_ng::rel::{
    id::RelocationID,
    offset::{Offset, VariantOffset},
};
use commonlibsse_ng::skse::{
    api::{ApiStorageError, get_messaging_interface},
    interfaces::messaging::{Message, MessageType, MessagingError},
};
use windows::Win32::Graphics::Direct3D11::{ID3D11Device, ID3D11DeviceContext};

use crate::wheeler::Wheeler;

// TODO: use AtomicFn: https://github.com/vcfxb/atomic_fn/blob/main/src/lib.rs
static FUNC: AtomicPtr<fn(u32)> = AtomicPtr::new(ptr::null_mut());
static D3D_INIT_HOOK_INITIALIZED: AtomicBool = AtomicBool::new(false);

pub struct D3DInitHook;
impl D3DInitHook {
    const ID: RelocationID = RelocationID::from_se_ae_id(75595, 77226);
    /// FIXME: The `VR` offset is unknown.
    const OFFSET: VariantOffset = VariantOffset::new(0x9, 0x275, 0x9);

    fn thunk(p1: u32) {
        unsafe { FUNC.load(atomic::Ordering::Acquire).as_ref().map(|f| f(p1)) };

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

pub struct DXGIPresentHook;
impl DXGIPresentHook {
    const ID: RelocationID = RelocationID::from_se_ae_id(75461, 77246);
    const OFFSET: Offset = Offset::new(0x9);

    fn thunk(pl: u32) {
        unsafe {
            // prologue
            imgui::sys::igNewFrame();

            draw();

            // epilogue
            imgui::sys::igEndFrame();
            imgui::sys::igRender();
        }
    }
}

fn draw() {
    if let Some(io) = unsafe { imgui::sys::igGetIO().as_ref() } {
        Wheeler::update(io.DeltaTime);
    }
}

fn message_callback(msg: &Message) {
    let is_loaded = msg.msg_type.to_enum() == Some(MessageType::DataLoaded);

    if is_loaded && D3D_INIT_HOOK_INITIALIZED.load(atomic::Ordering::Acquire) {
        if let Some(io) = unsafe { imgui::sys::igGetIO().as_mut() } {
            io.MouseDrawCursor = true;
            io.WantSetMousePos = true;
        };
    }
}

static SHOW_METERS: AtomicBool = AtomicBool::new(false);
static DEVICE: AtomicPtr<ID3D11Device> = AtomicPtr::new(ptr::null_mut());
static CONTEXT: AtomicPtr<ID3D11DeviceContext> = AtomicPtr::new(ptr::null_mut());

pub fn install() -> Result<(), RenderManagerError> {
    let msg_interface = get_messaging_interface()?;
    msg_interface.register_skse_listener(message_callback)?;
    Ok(())
}

#[derive(Debug, snafu::Snafu)]
pub enum RenderManagerError {
    #[snafu(transparent)]
    FailedGetMessagingINterface { source: ApiStorageError },
    #[snafu(transparent)]
    FailedRegisterRegisterListener { source: MessagingError },
}

pub fn get_resolution_scale_width() -> Option<f32> {
    Some(unsafe { imgui::sys::igGetIO().as_ref()? }.DisplaySize.x / 1920.0)
}

pub fn get_resolution_scale_height() -> Option<f32> {
    Some(unsafe { imgui::sys::igGetIO().as_ref()? }.DisplaySize.y / 1080.0)
}
