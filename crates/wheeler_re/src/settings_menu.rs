//! The in-game settings window (`wheeler_imgui::settings`), opened with the
//! `toggleSettings` binding. While it is open all input is taken from the game:
//! the mouse drives ImGui and key presses feed binding recording.

use std::sync::Mutex;

use libskyrim::{skse_error, skse_message};
use wheeler_core::binding::{ComboDetector, Edge};
use wheeler_core::control::{self, Action, CONTROL_SETTINGS_PATH, ControlConfig};
use wheeler_core::ini::patch_ini;
use wheeler_core::input_id::{KeyId, MOUSE_OFFSET};
use wheeler_core::recorder::CANCEL_KEY;
use wheeler_imgui::settings::{SettingsEvent, SettingsWindow};

use crate::{controls, wheeler};

struct Menu {
    window: SettingsWindow,
    /// Mouse movement since the last frame.
    mouse_delta: [f32; 2],
    mouse_pos: Option<[f32; 2]>,
    buttons: [bool; 3],
    wheel: f32,
    /// `toggleSettings` only, to close the window with the same combo.
    toggle: Option<ComboDetector<()>>,
}

static MENU: Mutex<Menu> = Mutex::new(Menu {
    window: SettingsWindow::new_closed(),
    mouse_delta: [0.0; 2],
    mouse_pos: None,
    buttons: [false; 3],
    wheel: 0.0,
    toggle: None,
});

static EPOCH: std::sync::LazyLock<std::time::Instant> =
    std::sync::LazyLock::new(std::time::Instant::now);

fn lock() -> std::sync::MutexGuard<'static, Menu> {
    MENU.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn is_open() -> bool {
    lock().window.is_open()
}

/// Opens or closes the window (`toggleSettings` binding).
pub fn toggle() {
    let mut m = lock();
    if m.window.is_open() {
        m.window.close();
    } else {
        let control = control::control();
        m.window.open(&control);
        let mut toggle = ComboDetector::new(control.wheel.multi_tap_window);
        for bindings in [&control.mkb, &control.gamepad] {
            toggle.bind(bindings.get(Action::ToggleSettings), Edge::Press, ());
        }
        drop(control);
        m.toggle = Some(toggle);
        m.mouse_pos = None; // centred on the next frame
        m.buttons = [false; 3];
        if wheeler::is_wheeler_open() {
            wheeler::toggle_wheeler();
        }
    }
    drop(m);
    // Keys held while opening/closing are released elsewhere.
    controls::reset_keys();
}

pub fn on_mouse_move(dx: f32, dy: f32) {
    let mut m = lock();
    m.mouse_delta[0] += dx;
    m.mouse_delta[1] += dy;
}

pub fn on_key(key: KeyId, down: bool) {
    let mut m = lock();
    if m.window.is_recording() {
        m.window.on_key(key, down);
        return;
    }
    let now = EPOCH.elapsed().as_secs_f32();
    let toggled = m
        .toggle
        .as_mut()
        .is_some_and(|t| !t.on_key(key, down, now).is_empty());
    if toggled {
        m.window.close();
        drop(m);
        controls::reset_keys();
        return;
    }
    match key.checked_sub(MOUSE_OFFSET) {
        Some(i @ 0..=2) => m.buttons[i as usize] = down,
        Some(8) if down => m.wheel += 1.0,
        Some(9) if down => m.wheel -= 1.0,
        _ if key == CANCEL_KEY && down => {
            m.window.close();
            drop(m);
            controls::reset_keys();
        }
        _ => {}
    }
}

/// Feeds the mouse to ImGui; call before `new_frame`.
pub fn prepare_frame(io: &mut imgui::Io) {
    let mut m = lock();
    if !m.window.is_open() {
        if m.mouse_pos.take().is_some() {
            io.mouse_pos = [-f32::MAX, -f32::MAX];
            io.mouse_down = [false; 5];
        }
        return;
    }
    let size = io.display_size;
    let [x, y] = m.mouse_pos.unwrap_or([size[0] / 2.0, size[1] / 2.0]);
    let [dx, dy] = core::mem::take(&mut m.mouse_delta);
    let pos = [(x + dx).clamp(0.0, size[0]), (y + dy).clamp(0.0, size[1])];
    m.mouse_pos = Some(pos);
    io.mouse_pos = pos;
    io.mouse_down[..3].copy_from_slice(&m.buttons);
    io.mouse_wheel = core::mem::take(&mut m.wheel);
}

/// Draws the window and applies its results.
pub fn draw(ui: &imgui::Ui) {
    let events = lock().window.draw(ui);
    for event in events {
        match event {
            SettingsEvent::Save(config) => save(&config),
            SettingsEvent::ResetWheels => wheeler::setup_default_wheels(),
            SettingsEvent::Closed => controls::reset_keys(),
        }
    }
}

/// Applies `config` and writes it to `Controls.ini`, keeping the file's
/// comments and other keys.
fn save(config: &ControlConfig) {
    control::set_control(config);
    controls::bind_all_inputs_from_config();

    let text = std::fs::read_to_string(CONTROL_SETTINGS_PATH).unwrap_or_default();
    let patched = patch_ini(&text, &config.ini_entries());
    match std::fs::write(CONTROL_SETTINGS_PATH, patched) {
        Ok(()) => skse_message!("Saved {CONTROL_SETTINGS_PATH}"),
        Err(e) => skse_error!("Failed to save {CONTROL_SETTINGS_PATH}: {e}"),
    }
}
