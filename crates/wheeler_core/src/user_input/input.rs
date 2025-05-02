use core::ffi::CStr;

use commonlibsse_ng::re::BSWin32GamepadDevice::{Key, Key_CEnum};
use commonlibsse_ng::re::ControlMap::ControlMap;
use commonlibsse_ng::re::InputDevices::INPUT_DEVICE_SE;
use commonlibsse_ng::re::InputEvent::{Event, InputEvent};
use commonlibsse_ng::re::UserEvents::{INPUT_CONTEXT_ID, INPUT_CONTEXT_ID_SE};

use super::controls::KeyId;

#[inline]
const fn get_gamepad_index(key: Key) -> Option<u32> {
    let index = match key {
        Key::Up => 0,
        Key::Down => 1,
        Key::Left => 2,
        Key::Right => 3,
        Key::Start => 4,
        Key::Back => 5,
        Key::LeftThumb => 6,
        Key::RightThumb => 7,
        Key::LeftShoulder => 8,
        Key::RightShoulder => 9,
        Key::A => 10,
        Key::B => 11,
        Key::X => 12,
        Key::Y => 13,
        Key::LeftTrigger => 14,
        Key::RightTrigger => 15,

        Key::LeftStick | Key::RightStick => return None,
    };

    const GAMEPAD_OFFSET: u32 = 266;
    Some(index + GAMEPAD_OFFSET)
}

const EVENTS_TO_FILTER_WHEN_WHEELER_ACTIVE: [&CStr; 11] = [
    c"Favorites",
    c"Inventory",
    c"Stats",
    c"Map",
    c"Tween Menu",
    c"Quick Inventory",
    c"Quick Magic",
    c"Quick Stats",
    c"Quick Map",
    c"Wait",
    c"Journal",
];

/// # Safety
pub unsafe fn process_and_filter(event: *mut *mut InputEvent) {
    let Some(event) = (unsafe { event.as_ref().and_then(|event| event.as_ref()) }) else {
        return;
    };
    let is_wheeler_open = false; // TODO: crate::wheeler::is_wheeler_open();

    while let Some(event) = unsafe { event.iter().next() } {
        let mut should_dispatch = true;

        let Some(event) = event.cast_to_event() else {
            continue;
        };

        match event {
            Event::Button(button_event) => {
                let mut input = button_event.__base.idCode;
                let mut is_gamepad = false;

                let Some(device) = button_event.__base.__base.device.as_se() else {
                    continue;
                };

                match device {
                    INPUT_DEVICE_SE::Keyboard => {
                        break;
                    }
                    INPUT_DEVICE_SE::Mouse => {
                        const MOUSE_OFFSET: u32 = 266;
                        input += MOUSE_OFFSET;
                        break;
                    }
                    INPUT_DEVICE_SE::Gamepad => {
                        if let Some(key) = Key_CEnum(input).to_enum() {
                            if let Some(index) = get_gamepad_index(key) {
                                input = index;
                            };
                        } else {
                            #[cfg(feature = "tracing")]
                            tracing::error!("Not found gamepad index!")
                        };
                        break;
                    }
                    INPUT_DEVICE_SE::FlatVirtualKeyboard => {}
                }

                let is_key_bound = super::controls::is_key_bound(KeyId(input));

                if is_wheeler_open {
                    if is_key_bound {
                        should_dispatch = false;
                    } else if let Some(ctrl_map) = ControlMap::get_singleton() {
                        const INPUT_CONTEXT: INPUT_CONTEXT_ID =
                            INPUT_CONTEXT_ID(INPUT_CONTEXT_ID_SE::Gameplay as u32);
                        let event_name =
                            ctrl_map.get_user_event_name(input, device.into(), INPUT_CONTEXT);

                        match event_name {
                            Some(event_name) => {
                                should_dispatch = EVENTS_TO_FILTER_WHEN_WHEELER_ACTIVE
                                    .contains(&event_name.as_c_str())
                            }
                            None => should_dispatch = false,
                        };
                    }
                }

                if is_key_bound {
                    let is_down = button_event.is_down();
                    if is_down || button_event.is_up() {
                        super::controls::dispatch(
                            KeyId(button_event.__base.__base.device.0),
                            is_down,
                            is_gamepad,
                        );
                    }
                }
            }
            Event::MouseMove(mouse_move_event) => {
                if is_wheeler_open {
                    // TODO: Wheeler::UpdateCursorPosMouse(mouse_move_ptr->x, mouse_move_ptr->y);
                    should_dispatch = false;
                }
            }
            Event::Char(_) => {}
            Event::Thumbstick(thumbstick_event) => {
                if is_wheeler_open && thumbstick_event.is_right() {
                    let x = thumbstick_event.xValue;
                    let y = thumbstick_event.yValue;
                    // TODO: Wheeler::UpdateCursorPosGamepad(x, y);
                    should_dispatch = false; // block thumbstick right input when wheel is open.
                }
            }
        };
    }
}
