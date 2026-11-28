//! Port of `UserInput/Controls.cpp`, generalized to key combos: chords,
//! multi-taps and release actions go through a
//! [`ComboDetector`](wheeler_core::binding::ComboDetector).

use std::sync::{LazyLock, Mutex};
use std::time::Instant;

use wheeler_core::binding::{ComboDetector, Edge};
use wheeler_core::control::{self, Action};
use wheeler_core::input_id::KeyId;

use crate::{settings_menu, wheeler};

type Callback = fn();

static DETECTOR: LazyLock<Mutex<ComboDetector<Callback>>> =
    LazyLock::new(|| Mutex::new(ComboDetector::default()));
static START: LazyLock<Instant> = LazyLock::new(Instant::now);

fn now() -> f32 {
    START.elapsed().as_secs_f32()
}

/// What pressing (and, for hold-to-close, releasing) each action does.
fn callbacks(action: Action) -> (Callback, Option<Callback>) {
    match action {
        Action::ToggleWheel => (
            wheeler::toggle_wheeler,
            Some(wheeler::close_wheeler_if_opened_long_enough),
        ),
        Action::ToggleWheelIfInInventory => (
            wheeler::toggle_wheel_if_in_inventory,
            Some(wheeler::close_wheeler_if_opened_long_enough_if_in_inventory),
        ),
        Action::ToggleWheelIfNotInInventory => (
            wheeler::toggle_wheel_if_not_in_inventory,
            Some(wheeler::close_wheeler_if_opened_long_enough_if_not_in_inventory),
        ),
        Action::NextWheel => (wheeler::next_wheel, None),
        Action::PrevWheel => (wheeler::prev_wheel, None),
        Action::NextItem => (wheeler::next_item_in_entry, None),
        Action::PrevItem => (wheeler::prev_item_in_entry, None),
        Action::ActivatePrimary => (wheeler::activate_hovered_entry_primary, None),
        Action::ActivateSecondary => (wheeler::activate_hovered_entry_secondary, None),
        Action::AddWheel => (wheeler::add_wheel, None),
        Action::AddEmptyEntry => (wheeler::add_empty_entry_to_current_wheel, None),
        Action::MoveEntryForward => (wheeler::move_entry_forward_in_current_wheel, None),
        Action::MoveEntryBack => (wheeler::move_entry_back_in_current_wheel, None),
        Action::MoveWheelForward => (wheeler::move_wheel_forward, None),
        Action::MoveWheelBack => (wheeler::move_wheel_back, None),
        Action::OpenSettings => (settings_menu::toggle, None),
    }
}

/// C++ `Controls::BindAllInputsFromConfig`
pub fn bind_all_inputs_from_config() {
    let control = control::control();
    let mut d = DETECTOR.lock().unwrap_or_else(|e| e.into_inner());
    d.clear();
    d.tap_window = control.wheel.multi_tap_window;
    for bindings in [&control.mkb, &control.gamepad] {
        for (action, combo) in bindings.iter() {
            let (press, release) = callbacks(action);
            d.bind(combo, Edge::Press, press);
            if let Some(release) = release {
                d.bind(combo, Edge::Release, release);
            }
        }
    }
}

/// C++ `Controls::IsKeyBound`
pub fn is_key_bound(key: KeyId) -> bool {
    DETECTOR
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .is_key_bound(key)
}

fn run(actions: Vec<Callback>) {
    // Run outside the lock: actions may query bindings themselves.
    for action in actions {
        action();
    }
}

/// C++ `Controls::Dispatch`: feeds one key edge.
pub fn on_key(key: KeyId, down: bool) {
    let actions = DETECTOR
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .on_key(key, down, now());
    run(actions);
}

/// Forgets held keys (see `ComboDetector::reset_keys`).
pub fn reset_keys() {
    DETECTOR
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .reset_keys();
}

/// Fires multi-tap bindings whose tap window passed; call once per frame.
pub fn tick() {
    let actions = DETECTOR
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .tick(now());
    run(actions);
}
