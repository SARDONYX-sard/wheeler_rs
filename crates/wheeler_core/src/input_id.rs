//! Wheeler's key-id numbering from `UserInput/Input.cpp`: keyboard scan codes
//! as-is, mouse buttons `+256`, gamepad buttons as a dense index `+266`.

/// A key in Wheeler's unified numbering.
pub type KeyId = u32;

pub const INVALID: u32 = u32::MAX;
pub const KEYBOARD_OFFSET: u32 = 0;
pub const MOUSE_OFFSET: u32 = 256;
pub const GAMEPAD_OFFSET: u32 = 266;

/// `RE::BSWin32GamepadDevice::Key` values (XInput button masks, plus the two
/// trigger pseudo keys) in Wheeler's dense order.
pub const GAMEPAD_KEYS: [u32; 16] = [
    0x0001, // kUp
    0x0002, // kDown
    0x0004, // kLeft
    0x0008, // kRight
    0x0010, // kStart
    0x0020, // kBack
    0x0040, // kLeftThumb
    0x0080, // kRightThumb
    0x0100, // kLeftShoulder
    0x0200, // kRightShoulder
    0x1000, // kA
    0x2000, // kB
    0x4000, // kX
    0x8000, // kY
    0x0009, // kLeftTrigger
    0x000A, // kRightTrigger
];

/// C++ `GetGamepadIndex`
pub fn gamepad_index(key: u32) -> u32 {
    GAMEPAD_KEYS
        .iter()
        .position(|&k| k == key)
        .map_or(INVALID, |index| index as u32 + GAMEPAD_OFFSET)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_controls_ini_comments() {
        // Controls.ini documents LT=280, RT=281, DPAD left/right=268/269.
        assert_eq!(gamepad_index(0x0009), 280);
        assert_eq!(gamepad_index(0x000A), 281);
        assert_eq!(gamepad_index(0x0004), 268);
        assert_eq!(gamepad_index(0x0008), 269);
        assert_eq!(gamepad_index(0x0B), INVALID);
    }
}

/// Display name of a key id (DirectInput scan codes, mouse, XInput).
pub fn key_name(key: KeyId) -> String {
    const GAMEPAD: [&str; 16] = [
        "DPad Up",
        "DPad Down",
        "DPad Left",
        "DPad Right",
        "Start",
        "Back",
        "LS",
        "RS",
        "LB",
        "RB",
        "A",
        "B",
        "X",
        "Y",
        "LT",
        "RT",
    ];
    const MOUSE: [&str; 10] = [
        "Mouse Left",
        "Mouse Right",
        "Mouse Middle",
        "Mouse 4",
        "Mouse 5",
        "Mouse 6",
        "Mouse 7",
        "Mouse 8",
        "Wheel Up",
        "Wheel Down",
    ];
    let name = if key >= GAMEPAD_OFFSET {
        GAMEPAD.get((key - GAMEPAD_OFFSET) as usize).copied()
    } else if key >= MOUSE_OFFSET {
        MOUSE.get((key - MOUSE_OFFSET) as usize).copied()
    } else {
        keyboard_name(key)
    };
    name.map_or_else(|| format!("0x{key:X}"), str::to_owned)
}

/// Display name of a chord, e.g. `LShift + E`.
pub fn keys_name(keys: &[KeyId]) -> String {
    keys.iter()
        .map(|&k| key_name(k))
        .collect::<Vec<_>>()
        .join(" + ")
}

/// DirectInput (`DIK_*`) scan code names.
fn keyboard_name(scan: KeyId) -> Option<&'static str> {
    const LETTERS_Q: &[&str] = &["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P"]; // 0x10..
    const LETTERS_A: &[&str] = &["A", "S", "D", "F", "G", "H", "J", "K", "L"]; // 0x1E..
    const LETTERS_Z: &[&str] = &["Z", "X", "C", "V", "B", "N", "M"]; // 0x2C..
    const DIGITS: &[&str] = &["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"]; // 0x02..
    const F1_10: &[&str] = &["F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10"]; // 0x3B..
    let range = |start: KeyId, names: &'static [&'static str]| {
        scan.checked_sub(start)
            .and_then(|i| names.get(i as usize))
            .copied()
    };
    if let Some(name) = range(0x10, LETTERS_Q)
        .or_else(|| range(0x1E, LETTERS_A))
        .or_else(|| range(0x2C, LETTERS_Z))
        .or_else(|| range(0x02, DIGITS))
        .or_else(|| range(0x3B, F1_10))
    {
        return Some(name);
    }
    Some(match scan {
        0x01 => "Esc",
        0x0C => "-",
        0x0D => "=",
        0x0E => "Backspace",
        0x0F => "Tab",
        0x1A => "[",
        0x1B => "]",
        0x1C => "Enter",
        0x1D => "LCtrl",
        0x27 => ";",
        0x28 => "Quote",
        0x29 => "`",
        0x2A => "LShift",
        0x2B => "\\",
        0x33 => ",",
        0x34 => ".",
        0x35 => "/",
        0x36 => "RShift",
        0x37 => "Num *",
        0x38 => "LAlt",
        0x39 => "Space",
        0x3A => "Caps Lock",
        0x45 => "Num Lock",
        0x46 => "Scroll Lock",
        0x47 => "Num 7",
        0x48 => "Num 8",
        0x49 => "Num 9",
        0x4A => "Num -",
        0x4B => "Num 4",
        0x4C => "Num 5",
        0x4D => "Num 6",
        0x4E => "Num +",
        0x4F => "Num 1",
        0x50 => "Num 2",
        0x51 => "Num 3",
        0x52 => "Num 0",
        0x53 => "Num .",
        0x57 => "F11",
        0x58 => "F12",
        0x9C => "Num Enter",
        0x9D => "RCtrl",
        0xB5 => "Num /",
        0xB8 => "RAlt",
        0xC7 => "Home",
        0xC8 => "Up",
        0xC9 => "Page Up",
        0xCB => "Left",
        0xCD => "Right",
        0xCF => "End",
        0xD0 => "Down",
        0xD1 => "Page Down",
        0xD2 => "Insert",
        0xD3 => "Delete",
        _ => return None,
    })
}

#[cfg(test)]
mod name_tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(key_name(0x12), "E");
        assert_eq!(key_name(0x2A), "LShift");
        assert_eq!(key_name(0x44), "F10");
        assert_eq!(key_name(256), "Mouse Left");
        assert_eq!(key_name(265), "Wheel Down");
        assert_eq!(key_name(280), "LT");
        assert_eq!(key_name(0x7F), "0x7F");
        assert_eq!(keys_name(&[0x2A, 0x12]), "LShift + E");
    }
}
