//! Port of the control part of `Config.h` / `Config.cpp` (`Controls.ini`).
//! Styling and animation live in [`crate::style`].

use std::sync::{LazyLock, RwLock, RwLockReadGuard};

use crate::ini::{IniExt, IniSource};

pub const CONTROL_SETTINGS_PATH: &str = "Data\\SKSE\\Plugins\\wheeler\\Controls.ini";

/// `Config::InputBindings::{GamePad, MKB}` (key ids follow `Input.cpp`'s
/// keyboard / mouse(+256) / gamepad(+266) numbering; `0` = unmapped).
#[derive(Debug, Clone, Copy)]
pub struct InputBindings {
    pub next_wheel: u32,
    pub prev_wheel: u32,
    pub toggle_wheel: u32,
    pub next_item: u32,
    pub prev_item: u32,
    pub activate_primary: u32,
    pub activate_secondary: u32,
    pub add_wheel: u32,
    pub add_empty_entry: u32,
    pub move_entry_forward: u32,
    pub move_entry_back: u32,
    pub move_wheel_forward: u32,
    pub move_wheel_back: u32,
    /// Gamepad only.
    pub toggle_wheel_if_in_inventory: u32,
    /// Gamepad only.
    pub toggle_wheel_if_not_in_inventory: u32,
}

impl InputBindings {
    pub const GAMEPAD_DEFAULT: Self = Self {
        next_wheel: 281,         // right trigger
        prev_wheel: 0,           // unmapped
        toggle_wheel: 280,       // left trigger
        next_item: 269,          // DPAD right
        prev_item: 268,          // DPAD left
        activate_primary: 275,   // right shoulder
        activate_secondary: 274, // left shoulder
        add_wheel: 0,
        add_empty_entry: 0,
        move_entry_forward: 0,
        move_entry_back: 0,
        move_wheel_forward: 0,
        move_wheel_back: 0,
        toggle_wheel_if_in_inventory: 0,
        toggle_wheel_if_not_in_inventory: 0,
    };

    pub const MKB_DEFAULT: Self = Self {
        next_wheel: 0x12,        // E
        prev_wheel: 0x10,        // Q
        toggle_wheel: 58,        // caps lock
        prev_item: 264,          // mouse wheel up
        next_item: 265,          // mouse wheel down
        activate_primary: 256,   // left mouse button
        activate_secondary: 257, // right mouse button
        add_wheel: 49,           // N
        add_empty_entry: 50,     // M
        move_entry_forward: 200, // up arrow
        move_entry_back: 208,    // down arrow
        move_wheel_forward: 205, // right arrow
        move_wheel_back: 203,    // left arrow
        toggle_wheel_if_in_inventory: 0,
        toggle_wheel_if_not_in_inventory: 0,
    };
}

/// `Config::Control::Wheel`
#[derive(Debug, Clone, Copy)]
pub struct WheelControl {
    /// Radius that bounds the mouse cursor; grows with each entry so MKB users
    /// don't rotate the cursor too fast.
    pub cursor_radius_per_entry: f32,
    pub double_activate_disable: bool,
    /// Holding longer than this closes the wheel on release; shorter presses
    /// close it on a second press.
    pub toggle_hold_threshold: f32,
    pub hide_game_ui_in_edit_mode: bool,
}

impl WheelControl {
    pub const DEFAULT: Self = Self {
        cursor_radius_per_entry: 10.0,
        double_activate_disable: true,
        toggle_hold_threshold: 0.25,
        hide_game_ui_in_edit_mode: true,
    };
}

#[derive(Debug, Clone, Copy)]
pub struct ControlConfig {
    pub gamepad: InputBindings,
    pub mkb: InputBindings,
    pub wheel: WheelControl,
}

impl ControlConfig {
    pub const DEFAULT: Self = Self {
        gamepad: InputBindings::GAMEPAD_DEFAULT,
        mkb: InputBindings::MKB_DEFAULT,
        wheel: WheelControl::DEFAULT,
    };
}

static CONTROL: LazyLock<RwLock<ControlConfig>> =
    LazyLock::new(|| RwLock::new(ControlConfig::DEFAULT));

pub fn control() -> RwLockReadGuard<'static, ControlConfig> {
    CONTROL.read().unwrap_or_else(|e| e.into_inner())
}

/// Part of C++ `Config::OffsetSizingToViewport` (see [`crate::style`]).
pub(crate) fn scale_cursor_radius(scale: f32) {
    CONTROL
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .wheel
        .cursor_radius_per_entry *= scale;
}

fn read_bindings(cfg: &dyn IniSource, section: &str, b: &mut InputBindings, gamepad: bool) {
    let read = |key: &str, value: &mut u32| *value = cfg.u32_or(section, key, *value);
    read("nextWheel", &mut b.next_wheel);
    read("prevWheel", &mut b.prev_wheel);
    read("toggleWheel", &mut b.toggle_wheel);
    read("nextItem", &mut b.next_item);
    read("prevItem", &mut b.prev_item);
    read("activatePrimary", &mut b.activate_primary);
    read("activateSecondary", &mut b.activate_secondary);
    read("addWheel", &mut b.add_wheel);
    read("addEmptyEntry", &mut b.add_empty_entry);
    read("moveEntryForward", &mut b.move_entry_forward);
    read("moveEntryBack", &mut b.move_entry_back);
    read("moveWheelForward", &mut b.move_wheel_forward);
    read("moveWheelBack", &mut b.move_wheel_back);
    if gamepad {
        read(
            "toggleWheelIfInInventory",
            &mut b.toggle_wheel_if_in_inventory,
        );
        read(
            "toggleWheelIfNotInInventory",
            &mut b.toggle_wheel_if_not_in_inventory,
        );
    }
}

/// C++ `Config::ReadControlConfig`: overrides bindings and wheel controls
/// with every key present in `cfg` (the contents of [`CONTROL_SETTINGS_PATH`]).
pub fn read_control_config(cfg: &dyn IniSource) {
    let mut control = CONTROL.write().unwrap_or_else(|e| e.into_inner());
    // Start from the defaults (see `style::read_style_config`).
    *control = ControlConfig::DEFAULT;
    read_bindings(cfg, "InputBindings.GamePad", &mut control.gamepad, true);
    read_bindings(cfg, "InputBindings.MKB", &mut control.mkb, false);

    let wheel = &mut control.wheel;
    wheel.cursor_radius_per_entry = cfg.f32_or(
        "Control.Wheel",
        "CursorRadiusPerEntry",
        wheel.cursor_radius_per_entry,
    );
    wheel.double_activate_disable = cfg.bool_or(
        "Control.Wheel",
        "DoubleActivateDisable",
        wheel.double_activate_disable,
    );
    wheel.toggle_hold_threshold = cfg.f32_or(
        "Control.Wheel",
        "ToggleHoldThreshold",
        wheel.toggle_hold_threshold,
    );
    wheel.hide_game_ui_in_edit_mode = cfg.bool_or(
        "Control.Wheel",
        "HideGameUIInEditMode",
        wheel.hide_game_ui_in_edit_mode,
    );
}
