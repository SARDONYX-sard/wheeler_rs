//! `Config::Sound`: UI sounds Wheeler plays, by editor id.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sound {
    /// Switching wheels.
    WheelSwitch,
    /// Hovering a different entry.
    EntrySwitch,
    /// Opening the wheel.
    WheelerToggle,
    /// Cycling items within an entry.
    ItemSwitch,
}

impl Sound {
    /// Editor id of the `BGSSoundDescriptorForm` (C++ `SD_*`).
    pub const fn editor_id(self) -> &'static core::ffi::CStr {
        match self {
            Self::WheelSwitch => c"UIFavorite",
            Self::EntrySwitch => c"UIMenuFocus",
            Self::WheelerToggle => c"UIInventoryOpenSD",
            Self::ItemSwitch => c"UIMenuPrevNextSD",
        }
    }
}
