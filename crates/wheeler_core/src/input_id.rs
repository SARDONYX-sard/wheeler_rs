//! Wheeler's key-id numbering from `UserInput/Input.cpp`: keyboard scan codes
//! as-is, mouse buttons `+256`, gamepad buttons as a dense index `+266`.

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
