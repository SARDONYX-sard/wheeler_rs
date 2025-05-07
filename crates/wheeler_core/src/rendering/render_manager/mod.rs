mod hooks;

use core::sync::atomic::{self, AtomicBool};
use std::sync::OnceLock;

use self::hooks::{D3D_INIT_HOOK_INITIALIZED, D3DInitHook, DXGIPresentHook};
use crate::hook::HookError;
use commonlibsse_ng::skse::{
    api::{ApiStorageError, get_messaging_interface},
    interfaces::messaging::{Message, MessageType, MessagingError},
};
use windows::Win32::Graphics::Direct3D11::{ID3D11Device, ID3D11DeviceContext};

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

/// Provided by `forwarder` field of `BSRenderManager::get_singleton()`
///
/// # When enabled?
/// `D3DInitHook` -> `render_manager::install`
pub(crate) static DEVICE: OnceLock<&'static ID3D11Device> = OnceLock::new();
/// Provided by `forwarder` field of `BSRenderManager::get_singleton()`
///
/// # When enabled?
/// `D3DInitHook` -> `render_manager::install`
pub(crate) static CONTEXT: OnceLock<&'static ID3D11DeviceContext> = OnceLock::new();

pub fn install() -> Result<(), RenderManagerError> {
    get_messaging_interface()?.register_skse_listener(message_callback)?;

    D3DInitHook::hook_start()?;
    DXGIPresentHook::hook_start()?;

    Ok(())
}

pub fn get_resolution_scale_width() -> Option<f32> {
    Some(unsafe { imgui::sys::igGetIO().as_ref()? }.DisplaySize.x / 1920.0)
}

pub fn get_resolution_scale_height() -> Option<f32> {
    Some(unsafe { imgui::sys::igGetIO().as_ref()? }.DisplaySize.y / 1080.0)
}

#[derive(Debug, snafu::Snafu)]
pub enum RenderManagerError {
    #[snafu(transparent)]
    FailedGetMessagingINterface { source: ApiStorageError },
    #[snafu(transparent)]
    FailedRegisterRegisterListener { source: MessagingError },
    #[snafu(transparent)]
    HookError { source: HookError },

    /// D3D_INIT_ORIGINAL_FN is already set.
    FailedToSetD3dInitOriginalFn,
    /// DXGI_PRESENT_ORIGINAL_FN is already set.
    FailedToSetDxgiPresentOriginalFn,
}
