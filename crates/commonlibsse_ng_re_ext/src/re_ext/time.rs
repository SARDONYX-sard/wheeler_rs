/// Returns a mutable reference to the global game time multiplier
/// if it can be safely accessed.
///
/// # Errors
/// Internally uses unsafe casting, but returns `None` if the address is
/// not available for the current game version.
///
/// # Returns
/// - `Some(&'static mut f32)` if the global time multiplier can be accessed.
/// - `None` if the address is unavailable.
///
/// This is typically used to modify the in-game time scale directly.
#[commonlibsse_ng::relocate(cast_as = "*mut f32", default = "None", id(se = 511883, ae = 38844))]
#[inline]
fn global_time_scale_mut() -> Option<&'static mut f32> {
    |as_type: AsType| unsafe { as_type.as_mut() }
}

/// Sets the global game time multiplier to the given value.
///
/// # Panics
/// Relies on memory relocation and may fail silently if the game version
/// is unsupported or the address is unavailable.
#[commonlibsse_ng::relocate_fn(se_id = 66989, ae_id = 68246)]
#[inline]
fn set_global_time_scale(new_time_scale: f32) {
    if let Some(time_scale) = global_time_scale_mut() {
        *time_scale = new_time_scale;
    }
}
