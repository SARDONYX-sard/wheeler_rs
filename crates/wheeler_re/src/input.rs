//! Port of `UserInput/Input.cpp`: filters the engine's input-event linked list
//! while the wheel is open and forwards bound keys to [`crate::controls`].

use libskyrim::re::{
    ButtonEvent, ControlMap, INPUT_CONTEXT_ID, INPUT_DEVICE, INPUT_EVENT_TYPE, InputEvent,
    MouseMoveEvent, ThumbstickEvent,
};

use wheeler_core::input_id::{KEYBOARD_OFFSET, MOUSE_OFFSET, gamepad_index};

use crate::{controls, settings_menu, wheeler};

/// User events that must not reach the game while the wheel is open, to avoid
/// opening the tween menu and friends on top of it.
const EVENTS_TO_FILTER_WHEN_WHEELER_ACTIVE: &[&str] = &[
    "Favorites",
    "Inventory",
    "Stats",
    "Map",
    "Tween Menu",
    "Quick Inventory",
    "Quick Magic",
    "Quick Stats",
    "Quick Map",
    "Wait",
    "Journal",
];

fn is_filtered_user_event(id_code: u32, device: INPUT_DEVICE) -> bool {
    let Some(control_map) = (unsafe { ControlMap::get_singleton().as_ref() }) else {
        return false;
    };
    control_map
        .get_user_event_name(id_code, device, INPUT_CONTEXT_ID::kGameplay)
        .is_some_and(|name| EVENTS_TO_FILTER_WHEN_WHEELER_ACTIVE.contains(&name.as_str()))
}

/// Wheeler's unified key id of a button event.
fn key_id(button: &ButtonEvent, device: INPUT_DEVICE) -> u32 {
    let id_code = button.get_id_code();
    match device {
        INPUT_DEVICE::kMouse => id_code + MOUSE_OFFSET,
        INPUT_DEVICE::kKeyboard => id_code + KEYBOARD_OFFSET,
        INPUT_DEVICE::kGamepad => gamepad_index(id_code),
        _ => id_code,
    }
}

/// While the settings window is open every event goes to it (mouse for
/// ImGui, keys for recording) and none reaches the game.
fn process_settings_event(event: &mut InputEvent) {
    match event.try_get_event_type() {
        Some(INPUT_EVENT_TYPE::kMouseMove) => {
            let mouse = unsafe { &*(core::ptr::from_mut(event).cast::<MouseMoveEvent>()) };
            settings_menu::on_mouse_move(mouse.mouse_input_x as f32, mouse.mouse_input_y as f32);
        }
        Some(INPUT_EVENT_TYPE::kButton) => {
            let button = unsafe { &*(core::ptr::from_mut(event).cast::<ButtonEvent>()) };
            if button.is_down() || button.is_up() {
                let key = key_id(button, event.get_device());
                settings_menu::on_key(key, button.is_down());
            }
        }
        _ => {}
    }
}

/// Returns whether `event` should still reach the game.
fn process_event(event: &mut InputEvent, wheeler_open: bool) -> bool {
    match event.try_get_event_type() {
        Some(INPUT_EVENT_TYPE::kMouseMove) => {
            if wheeler_open {
                let mouse = unsafe { &*(core::ptr::from_mut(event).cast::<MouseMoveEvent>()) };
                wheeler::update_cursor_pos_mouse(
                    mouse.mouse_input_x as f32,
                    mouse.mouse_input_y as f32,
                );
                return false; // block mouse input while the wheel is open
            }
        }
        Some(INPUT_EVENT_TYPE::kThumbstick) => {
            if wheeler_open {
                let stick = unsafe { &*(core::ptr::from_mut(event).cast::<ThumbstickEvent>()) };
                if stick.is_right() {
                    wheeler::update_cursor_pos_gamepad(stick.x_value, stick.y_value);
                    return false; // block the right stick while the wheel is open
                }
            }
        }
        Some(INPUT_EVENT_TYPE::kButton) => {
            let button = unsafe { &*(core::ptr::from_mut(event).cast::<ButtonEvent>()) };
            let id_code = button.get_id_code();
            let device = event.get_device();
            let input = key_id(button, device);

            let is_key_bound = controls::is_key_bound(input);
            let mut dispatch = true;
            if wheeler_open {
                if is_key_bound {
                    // Block bound keys regardless of down/up/held.
                    dispatch = false;
                } else {
                    // NOTE: C++ looks the user event up with the offset-adjusted
                    // id, which never matches for mouse/gamepad; the raw id
                    // code is what `ControlMap` stores.
                    dispatch = !is_filtered_user_event(id_code, device);
                }
            }

            if is_key_bound && (button.is_down() || button.is_up()) {
                // Dispatch whether or not the wheel is open; Wheeler handles it.
                controls::on_key(input, button.is_down());
            }
            return dispatch;
        }
        _ => {}
    }
    true
}

/// C++ `Input::ProcessAndFilter`: walks the event list and unlinks every event
/// that must not reach the game.
///
/// # Safety
/// `head` must be the engine's input-event list head passed to the dispatcher.
pub unsafe fn process_and_filter(head: *mut *mut InputEvent) {
    if head.is_null() {
        return;
    }
    controls::tick();
    let settings_open = settings_menu::is_open();
    let wheeler_open = wheeler::is_wheeler_open();

    unsafe {
        let mut event = *head;
        let mut prev: *mut InputEvent = core::ptr::null_mut();
        while let Some(current) = event.as_mut() {
            let next = current.next;
            let keep = if settings_open {
                process_settings_event(current);
                false
            } else {
                process_event(current, wheeler_open)
            };
            if keep {
                prev = current;
            } else if let Some(prev) = prev.as_mut() {
                prev.next = next;
            } else {
                *head = next;
            }
            event = next;
        }
    }
}
