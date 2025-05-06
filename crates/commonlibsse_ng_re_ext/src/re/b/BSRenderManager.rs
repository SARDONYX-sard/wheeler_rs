use commonlibsse_ng::rel::relocation::{RelocationError, relocate_member, relocate_member_mut};
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Device, ID3D11DeviceContext, ID3D11RenderTargetView1, ID3D11ShaderResourceView1,
};
use windows::Win32::Graphics::Dxgi::IDXGISwapChain;
use windows::Win32::System::Threading::CRITICAL_SECTION;

#[derive(Debug)]
pub struct BSRenderManager {}

impl BSRenderManager {
    /// Returns the singleton instance of `Self`.
    #[commonlibsse_ng::relocate(
        cast_as = "*mut BSRenderManager",
        default = "None",
        id(se = 524907, ae = 411393)
    )]
    pub fn get_singleton() -> Option<&'static BSRenderManager> {
        |as_type: AsType| unsafe { as_type.as_ref() }
    }

    /// Returns the singleton instance of `Self`.
    ///
    /// # Safety
    /// When it is known whether this is mutable or not. (The author does not know if it is safe.)
    #[commonlibsse_ng::relocate(
        cast_as = "*mut BSRenderManager",
        default = "None",
        id(se = 524907, ae = 411393)
    )]
    pub unsafe fn get_singleton_mut() -> Option<&'static mut BSRenderManager> {
        |as_type: AsType| unsafe { as_type.as_mut() }
    }

    /// Get runtime offset definition fields.
    ///
    /// # Errors
    /// - This function may return an error if the module's state cannot be accessed, or if the `map_active` call fails when fetching the current version.
    /// - If the pointer is null
    /// - If the pointer is unaligned
    #[inline]
    pub fn get_runtime_data(&self) -> Result<&RUNTIME_DATA, RelocationError> {
        relocate_member(self, 0x48, 0x50)
    }

    /// Get mutable runtime offset definition fields.
    ///
    /// # Errors
    /// - This function may return an error if the module's state cannot be accessed, or if the `map_active` call fails when fetching the current version.
    /// - If the pointer is null
    /// - If the pointer is unaligned
    #[inline]
    pub fn get_runtime_data_mut(&mut self) -> Result<&mut RUNTIME_DATA, RelocationError> {
        relocate_member_mut(self, 0x48, 0x50)
    }

    #[commonlibsse_ng::relocate_fn(se_id = 75507, ae_id = 77299)]
    pub fn create_render_texture(&self, width: u32, height: u32) {}

    // this,     /// Get runtime offset definition fields.
    ///
    /// # Errors
    /// - This function may return an error if the module's state cannot be accessed, or if the `map_active` call fails when fetching the current version.
    /// - If the pointer is null
    /// - If the pointer is unaligned
    #[inline]
    pub fn get_lock(&self) -> Result<&CRITICAL_SECTION, RelocationError> {
        relocate_member(self, 0x2790, 0x2F00)
    }

    /// Get mutable runtime offset definition fields.
    ///
    /// # Errors
    /// - This function may return an error if the module's state cannot be accessed, or if the `map_active` call fails when fetching the current version.
    /// - If the pointer is null
    /// - If the pointer is unaligned
    #[inline]
    pub fn get_lock_mut(&mut self) -> Result<&mut CRITICAL_SECTION, RelocationError> {
        relocate_member_mut(self, 0x2790, 0x2F00)
    }
}

#[derive(Debug)]
pub struct RUNTIME_DATA {
    pub forwarder: *mut ID3D11Device,                 // 0x00
    pub context: *mut ID3D11DeviceContext,            // 0x08
    pub unk58: u64,                                   // 0x10
    pub unk60: u64,                                   // 0x18
    pub unk68: u64,                                   // 0x20
    pub swapChain: *mut IDXGISwapChain,               // 0x28
    pub unk78: u64,                                   // 0x30
    pub unk80: u64,                                   // 0x38
    pub renderView: *mut ID3D11RenderTargetView1,     // 0x40
    pub resourceView: *mut ID3D11ShaderResourceView1, // 0x48
}
const _: () = assert!(core::mem::size_of::<RUNTIME_DATA>() == 0x50);
