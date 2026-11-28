//! Port of the control part of `Config.h` / `Config.cpp` (`Controls.ini`).
//! Styling and animation live in [`crate::style`].
//!
//! Bindings are [`KeyCombo`]s (chords and multi-taps); the C++ single key
//! ids still parse.

use std::sync::{LazyLock, RwLock, RwLockReadGuard};

use crate::binding::KeyCombo;
use crate::ini::{IniExt, IniSource};

pub const CONTROL_SETTINGS_PATH: &str = "Data\\SKSE\\Plugins\\wheeler\\Controls.ini";

pub const SECTION_GAMEPAD: &str = "InputBindings.GamePad";
pub const SECTION_MKB: &str = "InputBindings.MKB";
pub const SECTION_WHEEL: &str = "Control.Wheel";
pub const SECTION_SETTINGS_WINDOW: &str = "Settings.Window";

/// Everything a key combo can be bound to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    ToggleWheel,
    ToggleWheelIfInInventory,
    ToggleWheelIfNotInInventory,
    NextWheel,
    PrevWheel,
    NextItem,
    PrevItem,
    ActivatePrimary,
    ActivateSecondary,
    AddWheel,
    AddEmptyEntry,
    MoveEntryForward,
    MoveEntryBack,
    MoveWheelForward,
    MoveWheelBack,
    /// Opens or closes the settings window (new in the Rust port).
    ToggleSettings,
}

impl Action {
    pub const COUNT: usize = 16;
    pub const ALL: [Self; Self::COUNT] = [
        Self::ToggleWheel,
        Self::ToggleWheelIfInInventory,
        Self::ToggleWheelIfNotInInventory,
        Self::NextWheel,
        Self::PrevWheel,
        Self::NextItem,
        Self::PrevItem,
        Self::ActivatePrimary,
        Self::ActivateSecondary,
        Self::AddWheel,
        Self::AddEmptyEntry,
        Self::MoveEntryForward,
        Self::MoveEntryBack,
        Self::MoveWheelForward,
        Self::MoveWheelBack,
        Self::ToggleSettings,
    ];

    /// The `Controls.ini` key.
    pub const fn ini_key(self) -> &'static str {
        match self {
            Self::ToggleWheel => "toggleWheel",
            Self::ToggleWheelIfInInventory => "toggleWheelIfInInventory",
            Self::ToggleWheelIfNotInInventory => "toggleWheelIfNotInInventory",
            Self::NextWheel => "nextWheel",
            Self::PrevWheel => "prevWheel",
            Self::NextItem => "nextItem",
            Self::PrevItem => "prevItem",
            Self::ActivatePrimary => "activatePrimary",
            Self::ActivateSecondary => "activateSecondary",
            Self::AddWheel => "addWheel",
            Self::AddEmptyEntry => "addEmptyEntry",
            Self::MoveEntryForward => "moveEntryForward",
            Self::MoveEntryBack => "moveEntryBack",
            Self::MoveWheelForward => "moveWheelForward",
            Self::MoveWheelBack => "moveWheelBack",
            Self::ToggleSettings => "toggleSettings",
        }
    }

    /// Human-readable name for the settings window.
    pub const fn label(self) -> &'static str {
        match self {
            Self::ToggleWheel => "Toggle wheel",
            Self::ToggleWheelIfInInventory => "Toggle wheel (in inventory)",
            Self::ToggleWheelIfNotInInventory => "Toggle wheel (outside inventory)",
            Self::NextWheel => "Next wheel",
            Self::PrevWheel => "Previous wheel",
            Self::NextItem => "Next item",
            Self::PrevItem => "Previous item",
            Self::ActivatePrimary => "Activate (primary)",
            Self::ActivateSecondary => "Activate (secondary)",
            Self::AddWheel => "Add wheel (edit mode)",
            Self::AddEmptyEntry => "Add entry (edit mode)",
            Self::MoveEntryForward => "Move entry forward (edit mode)",
            Self::MoveEntryBack => "Move entry back (edit mode)",
            Self::MoveWheelForward => "Move wheel forward (edit mode)",
            Self::MoveWheelBack => "Move wheel back (edit mode)",
            Self::ToggleSettings => "Toggle settings",
        }
    }

    const fn index(self) -> usize {
        self as usize
    }
}

/// `Config::InputBindings::{GamePad, MKB}`: one combo per [`Action`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputBindings {
    combos: [KeyCombo; Action::COUNT],
}

impl InputBindings {
    /// From single key ids in [`Action::ALL`] order (`0` = unbound).
    const fn from_keys(keys: [u32; Action::COUNT]) -> Self {
        let mut combos = [KeyCombo::NONE; Action::COUNT];
        let mut i = 0;
        while i < Action::COUNT {
            combos[i] = KeyCombo::single(keys[i]);
            i += 1;
        }
        Self { combos }
    }

    pub const GAMEPAD_DEFAULT: Self = Self::from_keys([
        280, // toggle: left trigger
        0,   // toggle if in inventory
        0,   // toggle if not in inventory
        281, // next wheel: right trigger
        0,   // prev wheel
        269, // next item: DPAD right
        268, // prev item: DPAD left
        275, // activate primary: right shoulder
        274, // activate secondary: left shoulder
        0, 0, 0, 0, 0, 0, // edit-mode actions
        0, // open settings
    ]);

    pub const MKB_DEFAULT: Self = {
        let mut b = Self::from_keys([
            58,   // toggle: caps lock
            0,    // toggle if in inventory
            0,    // toggle if not in inventory
            0x12, // next wheel: E
            0x10, // prev wheel: Q
            265,  // next item: mouse wheel down
            264,  // prev item: mouse wheel up
            256,  // activate primary: left mouse button
            257,  // activate secondary: right mouse button
            49,   // add wheel: N
            50,   // add entry: M
            200,  // move entry forward: up arrow
            208,  // move entry back: down arrow
            205,  // move wheel forward: right arrow
            203,  // move wheel back: left arrow
            0,    // open settings (below)
        ]);
        // Left shift + F10.
        b.combos[Action::ToggleSettings.index()] = KeyCombo::chord2(0x2A, 0x44);
        b
    };

    pub fn get(&self, action: Action) -> KeyCombo {
        self.combos[action.index()]
    }

    pub fn set(&mut self, action: Action, combo: KeyCombo) {
        self.combos[action.index()] = combo;
    }

    pub fn iter(&self) -> impl Iterator<Item = (Action, KeyCombo)> + '_ {
        Action::ALL.iter().map(|&a| (a, self.get(a)))
    }
}

/// `Config::Control::Wheel`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WheelControl {
    /// Radius that bounds the mouse cursor; grows with each entry so MKB users
    /// don't rotate the cursor too fast.
    pub cursor_radius_per_entry: f32,
    pub double_activate_disable: bool,
    /// Holding longer than this closes the wheel on release; shorter presses
    /// close it on a second press.
    pub toggle_hold_threshold: f32,
    pub hide_game_ui_in_edit_mode: bool,
    /// Key hints under the "EDIT MODE" banner.
    pub show_edit_mode_hints: bool,
    /// Seconds allowed between taps of a multi-tap binding.
    pub multi_tap_window: f32,
    /// Wheels (pages) created for a new game / "Reset all wheels".
    pub default_wheel_count: u32,
    /// Empty entries per default wheel.
    pub default_entry_count: u32,
}

impl WheelControl {
    pub const DEFAULT: Self = Self {
        cursor_radius_per_entry: 10.0,
        double_activate_disable: true,
        toggle_hold_threshold: 0.25,
        hide_game_ui_in_edit_mode: true,
        show_edit_mode_hints: true,
        multi_tap_window: 0.25,
        default_wheel_count: 2,
        default_entry_count: 8,
    };
}

/// Look of the settings window, in 1080p pixels (scaled with the screen).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SettingsWindowStyle {
    pub font_size: f32,
    pub width: f32,
    pub height: f32,
}

impl SettingsWindowStyle {
    pub const DEFAULT: Self = Self {
        font_size: 18.0,
        width: 720.0,
        height: 560.0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ControlConfig {
    pub gamepad: InputBindings,
    pub mkb: InputBindings,
    pub wheel: WheelControl,
    pub settings_window: SettingsWindowStyle,
}

impl ControlConfig {
    pub const DEFAULT: Self = Self {
        gamepad: InputBindings::GAMEPAD_DEFAULT,
        mkb: InputBindings::MKB_DEFAULT,
        wheel: WheelControl::DEFAULT,
        settings_window: SettingsWindowStyle::DEFAULT,
    };

    /// Every `(section, key, value)` written to `Controls.ini`; the cursor
    /// radius is left out because it is stored unscaled in the file.
    pub fn ini_entries(&self) -> Vec<(&'static str, &'static str, String)> {
        let mut out = Vec::new();
        for (section, bindings) in [(SECTION_GAMEPAD, &self.gamepad), (SECTION_MKB, &self.mkb)] {
            for (action, combo) in bindings.iter() {
                out.push((section, action.ini_key(), combo.to_ini_string()));
            }
        }
        let w = &self.wheel;
        out.push((
            SECTION_WHEEL,
            "ToggleHoldThreshold",
            format!("{:.6}", w.toggle_hold_threshold),
        ));
        out.push((
            SECTION_WHEEL,
            "MultiTapWindow",
            format!("{:.6}", w.multi_tap_window),
        ));
        out.push((
            SECTION_WHEEL,
            "HideGameUIInEditMode",
            w.hide_game_ui_in_edit_mode.to_string(),
        ));
        out.push((
            SECTION_WHEEL,
            "DefaultWheelCount",
            w.default_wheel_count.to_string(),
        ));
        out.push((
            SECTION_WHEEL,
            "DefaultEntryCount",
            w.default_entry_count.to_string(),
        ));
        out.push((
            SECTION_WHEEL,
            "ShowEditModeHints",
            w.show_edit_mode_hints.to_string(),
        ));
        let sw = &self.settings_window;
        out.push((
            SECTION_SETTINGS_WINDOW,
            "FontSize",
            format!("{:.1}", sw.font_size),
        ));
        out.push((SECTION_SETTINGS_WINDOW, "Width", format!("{:.0}", sw.width)));
        out.push((
            SECTION_SETTINGS_WINDOW,
            "Height",
            format!("{:.0}", sw.height),
        ));
        out
    }
}

static CONTROL: LazyLock<RwLock<ControlConfig>> =
    LazyLock::new(|| RwLock::new(ControlConfig::DEFAULT));

pub fn control() -> RwLockReadGuard<'static, ControlConfig> {
    CONTROL.read().unwrap_or_else(|e| e.into_inner())
}

/// Replaces the bindings and tap/hold settings (settings window), keeping the
/// viewport-scaled cursor radius.
pub fn set_control(new: &ControlConfig) {
    let mut control = CONTROL.write().unwrap_or_else(|e| e.into_inner());
    let radius = control.wheel.cursor_radius_per_entry;
    *control = *new;
    control.wheel.cursor_radius_per_entry = radius;
}

/// Part of C++ `Config::OffsetSizingToViewport` (see [`crate::style`]).
pub(crate) fn scale_cursor_radius(scale: f32) {
    CONTROL
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .wheel
        .cursor_radius_per_entry *= scale;
}

fn read_bindings(cfg: &dyn IniSource, section: &str, b: &mut InputBindings) {
    for action in Action::ALL {
        if let Some(combo) = cfg
            .value(section, action.ini_key())
            .and_then(KeyCombo::parse)
        {
            b.set(action, combo);
        }
    }
}

/// C++ `Config::ReadControlConfig`: overrides bindings and wheel controls
/// with every key present in `cfg` (the contents of [`CONTROL_SETTINGS_PATH`]).
pub fn read_control_config(cfg: &dyn IniSource) {
    let mut control = CONTROL.write().unwrap_or_else(|e| e.into_inner());
    // Start from the defaults (see `style::read_style_config`).
    *control = ControlConfig::DEFAULT;
    read_bindings(cfg, SECTION_GAMEPAD, &mut control.gamepad);
    read_bindings(cfg, SECTION_MKB, &mut control.mkb);

    let w = &mut control.wheel;
    w.cursor_radius_per_entry = cfg.f32_or(
        SECTION_WHEEL,
        "CursorRadiusPerEntry",
        w.cursor_radius_per_entry,
    );
    w.double_activate_disable = cfg.bool_or(
        SECTION_WHEEL,
        "DoubleActivateDisable",
        w.double_activate_disable,
    );
    w.toggle_hold_threshold = cfg.f32_or(
        SECTION_WHEEL,
        "ToggleHoldThreshold",
        w.toggle_hold_threshold,
    );
    w.hide_game_ui_in_edit_mode = cfg.bool_or(
        SECTION_WHEEL,
        "HideGameUIInEditMode",
        w.hide_game_ui_in_edit_mode,
    );
    w.multi_tap_window = cfg.f32_or(SECTION_WHEEL, "MultiTapWindow", w.multi_tap_window);
    w.default_wheel_count = cfg.u32_or(SECTION_WHEEL, "DefaultWheelCount", w.default_wheel_count);
    w.default_entry_count = cfg.u32_or(SECTION_WHEEL, "DefaultEntryCount", w.default_entry_count);

    w.show_edit_mode_hints =
        cfg.bool_or(SECTION_WHEEL, "ShowEditModeHints", w.show_edit_mode_hints);

    let sw = &mut control.settings_window;
    sw.font_size = cfg.f32_or(SECTION_SETTINGS_WINDOW, "FontSize", sw.font_size);
    sw.width = cfg.f32_or(SECTION_SETTINGS_WINDOW, "Width", sw.width);
    sw.height = cfg.f32_or(SECTION_SETTINGS_WINDOW, "Height", sw.height);
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::ini::MemoryIni;

    #[test]
    fn reads_cpp_and_combo_syntax() {
        let ini = MemoryIni::parse(
            "[InputBindings.MKB]
toggleWheel = 58
nextWheel = 0x2A+0x12*2
[Control.Wheel]
MultiTapWindow = 0.4
",
        );
        read_control_config(&ini);
        let c = *control();
        assert_eq!(c.mkb.get(Action::ToggleWheel), KeyCombo::single(58));
        assert_eq!(
            c.mkb.get(Action::NextWheel),
            KeyCombo::new(&[0x2A, 0x12], 2).unwrap()
        );
        assert_eq!(c.mkb.get(Action::PrevWheel), KeyCombo::single(0x10)); // default kept
        assert_eq!(c.wheel.multi_tap_window, 0.4);
        assert!(
            c.ini_entries()
                .contains(&(SECTION_MKB, "nextWheel", "18+42*2".to_owned()))
        );
    }
}
