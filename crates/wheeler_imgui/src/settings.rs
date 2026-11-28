//! In-game settings window (replaces the dMenu integration): key bindings
//! recorded by pressing them (chords and multi-taps), tap / hold timings.
//!
//! The window edits a draft copy of the [`ControlConfig`]; the host applies
//! it on [`SettingsEvent::Save`]. Key edges must be fed through
//! [`SettingsWindow::on_key`] while [`SettingsWindow::is_recording`].

use imgui::{Condition, TableFlags, Ui};
use wheeler_core::binding::{KeyCombo, MAX_TAPS};
use wheeler_core::control::{Action, ControlConfig, InputBindings};
use wheeler_core::input_id::{KeyId, keys_name};
use wheeler_core::recorder::{RecordResult, Recorder};

/// Which binding set a combo belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Device {
    Mkb,
    Gamepad,
}

impl Device {
    fn bindings(self, cfg: &mut ControlConfig) -> &mut InputBindings {
        match self {
            Self::Mkb => &mut cfg.mkb,
            Self::Gamepad => &mut cfg.gamepad,
        }
    }
}

/// What the host should do after a frame.
#[derive(Debug, Clone, PartialEq)]
pub enum SettingsEvent {
    /// Apply and persist this config.
    Save(Box<ControlConfig>),
    /// Replace every wheel with the defaults.
    ResetWheels,
    /// The window was closed.
    Closed,
}

#[derive(Debug, Clone)]
struct Recording {
    device: Device,
    action: Action,
    taps: u8,
    recorder: Recorder,
}

#[derive(Debug, Clone)]
pub struct SettingsWindow {
    open: bool,
    draft: ControlConfig,
    recording: Option<Recording>,
    status: String,
}

impl Default for SettingsWindow {
    fn default() -> Self {
        Self::new_closed()
    }
}

impl SettingsWindow {
    /// A closed window (usable in `static`s).
    pub const fn new_closed() -> Self {
        Self {
            open: false,
            draft: ControlConfig::DEFAULT,
            recording: None,
            status: String::new(),
        }
    }

    /// Opens the window editing a copy of `current`.
    pub fn open(&mut self, current: &ControlConfig) {
        self.open = true;
        self.draft = *current;
        self.recording = None;
        self.status.clear();
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn close(&mut self) {
        self.open = false;
        self.recording = None;
    }

    pub fn is_recording(&self) -> bool {
        self.recording.is_some()
    }

    pub fn draft(&self) -> &ControlConfig {
        &self.draft
    }

    /// Starts recording a binding (also what clicking its button does).
    pub fn start_recording(&mut self, device: Device, action: Action) {
        let taps = device.bindings(&mut self.draft).get(action).taps();
        self.recording = Some(Recording {
            device,
            action,
            taps,
            recorder: Recorder::new(),
        });
    }

    /// Feeds a key edge while recording.
    pub fn on_key(&mut self, key: KeyId, down: bool) {
        let Some(rec) = &mut self.recording else {
            return;
        };
        match rec.recorder.on_key(key, down, rec.taps) {
            RecordResult::Pending => {}
            RecordResult::Done(combo) => {
                let (device, action) = (rec.device, rec.action);
                device.bindings(&mut self.draft).set(action, combo);
                self.status = format!("{}: {}", action.label(), describe(&combo));
                self.recording = None;
            }
            RecordResult::Cancelled => {
                self.status = "Recording cancelled".to_owned();
                self.recording = None;
            }
        }
    }

    /// Draws the window; returns what the host must do.
    pub fn draw(&mut self, ui: &Ui) -> Vec<SettingsEvent> {
        let mut events = Vec::new();
        if !self.open {
            return events;
        }
        let mut open = true;
        ui.window("Wheeler Settings")
            .size([760.0, 640.0], Condition::FirstUseEver)
            .opened(&mut open)
            .collapsible(false)
            .build(|| self.draw_contents(ui, &mut events));
        if !open {
            self.close();
            events.push(SettingsEvent::Closed);
        }
        events
    }

    fn draw_contents(&mut self, ui: &Ui, events: &mut Vec<SettingsEvent>) {
        ui.text_wrapped(
            "Click a binding and press the keys (up to 4 at once). Esc alone cancels. \
             'x1/x2/x3' cycles single / double / triple tap.",
        );
        ui.separator();

        if let Some(_table) = ui.begin_table_with_flags(
            "bindings",
            5,
            TableFlags::BORDERS_INNER_H | TableFlags::ROW_BG | TableFlags::SIZING_STRETCH_PROP,
        ) {
            ui.table_setup_column("Action");
            ui.table_setup_column("Keyboard / Mouse");
            ui.table_setup_column("##mkb_taps");
            ui.table_setup_column("Gamepad");
            ui.table_setup_column("##pad_taps");
            ui.table_headers_row();
            for (row, action) in Action::ALL.into_iter().enumerate() {
                let _row_id = ui.push_id_usize(row);
                ui.table_next_row();
                ui.table_next_column();
                ui.text(action.label());
                for device in [Device::Mkb, Device::Gamepad] {
                    let _dev_id = ui.push_id_usize(device as usize);
                    ui.table_next_column();
                    self.draw_binding_button(ui, device, action);
                    ui.table_next_column();
                    self.draw_taps_button(ui, device, action);
                }
            }
        }

        ui.separator();
        let w = &mut self.draft.wheel;
        ui.slider("Multi-tap window (s)", 0.1, 1.0, &mut w.multi_tap_window);
        ui.slider(
            "Toggle hold threshold (s)",
            0.05,
            1.0,
            &mut w.toggle_hold_threshold,
        );
        ui.checkbox(
            "Hide game UI in edit mode",
            &mut w.hide_game_ui_in_edit_mode,
        );
        ui.slider("Default wheels (pages)", 1, 10, &mut w.default_wheel_count);
        ui.slider(
            "Default entries per wheel",
            1,
            16,
            &mut w.default_entry_count,
        );

        ui.separator();
        if ui.button("Save") {
            events.push(SettingsEvent::Save(Box::new(self.draft)));
            self.status = "Saved".to_owned();
        }
        ui.same_line();
        if ui.button("Defaults") {
            let radius = self.draft.wheel.cursor_radius_per_entry;
            self.draft = ControlConfig::DEFAULT;
            self.draft.wheel.cursor_radius_per_entry = radius;
            self.status = "Defaults loaded (not saved yet)".to_owned();
        }
        ui.same_line();
        if ui.button("Reset all wheels") {
            events.push(SettingsEvent::ResetWheels);
            self.status = "Wheels reset".to_owned();
        }
        if !self.status.is_empty() {
            ui.text(&self.status);
        }
    }

    fn draw_binding_button(&mut self, ui: &Ui, device: Device, action: Action) {
        let recording_this = self
            .recording
            .as_ref()
            .is_some_and(|r| r.device == device && r.action == action);
        let label = if recording_this {
            let held = self
                .recording
                .as_ref()
                .map(|r| keys_name(r.recorder.peak()))
                .unwrap_or_default();
            if held.is_empty() {
                "Press keys...".to_owned()
            } else {
                held
            }
        } else {
            describe(&device.bindings(&mut self.draft).get(action))
        };
        if ui.button_with_size(format!("{label}##bind"), [-1.0, 0.0]) && !self.is_recording() {
            self.start_recording(device, action);
        }
        if ui.is_item_clicked_with_button(imgui::MouseButton::Right) && !self.is_recording() {
            device.bindings(&mut self.draft).set(action, KeyCombo::NONE);
        }
        if ui.is_item_hovered() {
            ui.tooltip_text("Click to record, right-click to clear");
        }
    }

    fn draw_taps_button(&mut self, ui: &Ui, device: Device, action: Action) {
        let bindings = device.bindings(&mut self.draft);
        let combo = bindings.get(action);
        if ui.small_button(format!("x{}##taps", combo.taps())) {
            let next = combo.taps() % MAX_TAPS + 1;
            bindings.set(action, combo.with_taps(next));
        }
    }
}

/// `LShift + E (x2)`, or `-` when unbound.
pub fn describe(combo: &KeyCombo) -> String {
    if combo.is_none() {
        return "-".to_owned();
    }
    let keys = keys_name(combo.keys());
    if combo.taps() > 1 {
        format!("{keys} (x{})", combo.taps())
    } else {
        keys
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::headless::render_frame;

    #[test]
    fn records_a_chord_into_the_draft() {
        let mut w = SettingsWindow::default();
        w.open(&ControlConfig::DEFAULT);
        w.start_recording(Device::Mkb, Action::NextWheel);
        w.on_key(0x2A, true);
        w.on_key(0x12, true);
        assert!(w.is_recording());
        w.on_key(0x12, false);
        w.on_key(0x2A, false);
        assert!(!w.is_recording());
        assert_eq!(
            w.draft().mkb.get(Action::NextWheel),
            KeyCombo::new(&[0x2A, 0x12], 1).unwrap()
        );
    }

    #[test]
    fn escape_keeps_the_old_binding() {
        let mut w = SettingsWindow::default();
        w.open(&ControlConfig::DEFAULT);
        w.start_recording(Device::Gamepad, Action::ToggleWheel);
        w.on_key(0x01, true);
        w.on_key(0x01, false);
        assert_eq!(
            w.draft().gamepad.get(Action::ToggleWheel),
            KeyCombo::single(280)
        );
    }

    #[test]
    fn draws_without_events() {
        let mut w = SettingsWindow::default();
        w.open(&ControlConfig::DEFAULT);
        let mut events = Vec::new();
        let stats = render_frame([1920.0, 1080.0], |ui| events = w.draw(ui));
        assert!(stats.vertices > 0);
        assert!(events.is_empty());
        assert!(w.is_open());
    }
}
