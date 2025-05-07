use commonlibsse_ng::{
    rel::{ResolvableAddress, id::RelocationID, relocation::Relocation},
    skse,
};

/// To Hook information trait
pub trait Hook<OFFSET: ResolvableAddress> {
    const ID: RelocationID;
    /// Expected `Offset` or `VariantOffset`
    const OFFSET: OFFSET;

    /// Replaced any function pointer
    const REPLACED_FN: *const ();
}

/// Return original function pointer
///
/// # Safety
/// Safe function signature.
pub unsafe fn write_thunk_call<T, OFFSET>() -> Result<*const (), HookError>
where
    T: Hook<OFFSET>,
    OFFSET: ResolvableAddress,
{
    let original_addr = Relocation::from_id_offset(T::ID, T::OFFSET)?.address()?;
    let original_fn = original_addr.as_ptr().cast_const().cast();

    if let Err(err) = unsafe { skse::trampoline::add_hook(original_fn, T::REPLACED_FN) } {
        tracing::error!("Hook Error: {err}");
        return Err(HookError::FailedToHookFn);
    };
    Ok(original_fn)
}

#[derive(Debug, snafu::Snafu)]
pub enum HookError {
    #[snafu(transparent)]
    DataBaseError {
        source: commonlibsse_ng::rel::id::DataBaseError,
    },

    /// Failed to hook function
    FailedToHookFn,
}
