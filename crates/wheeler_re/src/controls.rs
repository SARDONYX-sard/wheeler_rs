//! Port of `UserInput/Controls.cpp`: key id -> wheel action tables.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use crate::wheeler;
use wheeler_core::control::{self as config, InputBindings};

pub type KeyId = u32;
pub type Action = fn();

#[derive(Default)]
struct Tables {
    down: HashMap<KeyId, Action>,
    up: HashMap<KeyId, Action>,
    down_gamepad: HashMap<KeyId, Action>,
    up_gamepad: HashMap<KeyId, Action>,
}

impl Tables {
    fn bind(&mut self, key: KeyId, action: Action, is_down: bool, is_gamepad: bool) {
        let table = match (is_down, is_gamepad) {
            (true, true) => &mut self.down_gamepad,
            (true, false) => &mut self.down,
            (false, true) => &mut self.up_gamepad,
            (false, false) => &mut self.up,
        };
        table.insert(key, action);
    }
}

static TABLES: LazyLock<Mutex<Tables>> = LazyLock::new(Mutex::default);

fn common_actions(b: &InputBindings) -> [(KeyId, Action); 13] {
    [
        (b.activate_primary, wheeler::activate_hovered_entry_primary),
        (
            b.activate_secondary,
            wheeler::activate_hovered_entry_secondary,
        ),
        (b.add_wheel, wheeler::add_wheel),
        (b.add_empty_entry, wheeler::add_empty_entry_to_current_wheel),
        (
            b.move_entry_forward,
            wheeler::move_entry_forward_in_current_wheel,
        ),
        (b.move_entry_back, wheeler::move_entry_back_in_current_wheel),
        (b.move_wheel_forward, wheeler::move_wheel_forward),
        (b.move_wheel_back, wheeler::move_wheel_back),
        (b.next_wheel, wheeler::next_wheel),
        (b.prev_wheel, wheeler::prev_wheel),
        (b.toggle_wheel, wheeler::toggle_wheeler),
        (b.prev_item, wheeler::prev_item_in_entry),
        (b.next_item, wheeler::next_item_in_entry),
    ]
}

/// C++ `Controls::BindAllInputsFromConfig`
///
/// Like the original, unmapped (`0`) bindings are inserted too; scan code `0`
/// is not a real key, so they never fire.
pub fn bind_all_inputs_from_config() {
    let control = config::control();
    let mut t = TABLES.lock().unwrap_or_else(|e| e.into_inner());
    *t = Tables::default();

    let mkb = &control.mkb;
    for (key, action) in common_actions(mkb) {
        t.bind(key, action, true, false);
    }
    t.bind(
        mkb.toggle_wheel,
        wheeler::close_wheeler_if_opened_long_enough,
        false,
        false,
    );

    let gp = &control.gamepad;
    for (key, action) in common_actions(gp) {
        t.bind(key, action, true, true);
    }
    t.bind(
        gp.toggle_wheel_if_not_in_inventory,
        wheeler::toggle_wheel_if_not_in_inventory,
        true,
        true,
    );
    t.bind(
        gp.toggle_wheel_if_in_inventory,
        wheeler::toggle_wheel_if_in_inventory,
        true,
        true,
    );
    t.bind(
        gp.toggle_wheel,
        wheeler::close_wheeler_if_opened_long_enough,
        false,
        true,
    );
    t.bind(
        gp.toggle_wheel_if_in_inventory,
        wheeler::close_wheeler_if_opened_long_enough_if_in_inventory,
        false,
        true,
    );
    t.bind(
        gp.toggle_wheel_if_not_in_inventory,
        wheeler::close_wheeler_if_opened_long_enough_if_not_in_inventory,
        false,
        true,
    );
}

/// C++ `Controls::IsKeyBound`
pub fn is_key_bound(key: KeyId) -> bool {
    let t = TABLES.lock().unwrap_or_else(|e| e.into_inner());
    t.down.contains_key(&key)
        || t.up.contains_key(&key)
        || t.down_gamepad.contains_key(&key)
        || t.up_gamepad.contains_key(&key)
}

/// C++ `Controls::Dispatch`: runs the action bound to `key`, if any.
pub fn dispatch(key: KeyId, is_down: bool, is_gamepad: bool) -> bool {
    // The C++ version calls the action while holding the lock; copy the fn
    // pointer out instead so actions may query bindings themselves.
    let action = {
        let t = TABLES.lock().unwrap_or_else(|e| e.into_inner());
        let table = match (is_down, is_gamepad) {
            (true, true) => &t.down_gamepad,
            (true, false) => &t.down,
            (false, true) => &t.up_gamepad,
            (false, false) => &t.up,
        };
        table.get(&key).copied()
    };
    match action {
        Some(action) => {
            action();
            true
        }
        None => false,
    }
}
