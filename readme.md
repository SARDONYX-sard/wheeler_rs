# wheeler_re

Rust port of [Wheeler](https://github.com/D7ry/wheeler), a quick-access wheel
menu for Skyrim SE / AE (1.5.97 – 1.7.x). Built on
[CommonLib-SkyrimLib](https://github.com/SARDONYX-forks/CommonLib-SkyrimLib/tree/pure-rust)
(pure Rust, no C++).

## Install

Requires [SKSE](https://skse.silverlock.org/) and
[Address Library](https://www.nexusmods.com/skyrimspecialedition/mods/32444).

```text
SKSE/Plugins/wheeler_re.dll
SKSE/Plugins/wheeler_re.pdb        # optional, for crash loggers
SKSE/Plugins/wheeler/              # Controls.ini, Styles.ini, resources/ (from Wheeler)
```

Wheels are saved in the SKSE co-save, compatible with saves made with the C++ Wheeler.

## Default controls

| Action                       | Keyboard / Mouse   | Gamepad        |
| ---------------------------- | ------------------ | -------------- |
| Open / close wheel (hold)    | Caps Lock          | LT             |
| Next / previous wheel        | E / Q              | RT / -         |
| Next / previous item         | Wheel Down / Up    | DPad Right / Left |
| Use item (primary)           | Mouse Left         | RB             |
| Use item (secondary)         | Mouse Right        | LB             |
| Toggle settings window       | LShift + F10       | -              |

Edit mode (open the wheel in the Inventory or Magic menu; shown as "EDIT MODE" with key hints, `ShowEditModeHints`):

| Action                           | Keyboard / Mouse       |
| -------------------------------- | ---------------------- |
| Add hovered menu item to entry   | Mouse Left             |
| Remove item / empty entry        | Mouse Right            |
| Add wheel / add entry            | N / M                  |
| Move entry forward / back        | Up / Down              |
| Move wheel forward / back        | Right / Left           |

## Settings

Press `LShift + F10` in game (again to close): record bindings by pressing them (chords of up
to 4 keys, single / double / triple tap), tap timing, default wheel layout.
Saved to `Controls.ini`, where bindings look like:

```ini
toggleWheel = 58          ; Caps Lock
addWheel = 0x2A+0x31      ; LShift + N
nextWheel = 281*2         ; double-tap RT
```

Default wheels: `DefaultWheelCount = 2`, `DefaultEntryCount = 8` (`[Control.Wheel]`).
Window look: `FontSize = 18`, `Width = 720`, `Height = 560` (`[Settings.Window]`, 1080p pixels).

## Build

```sh
cargo xtask build --release   # -> build/mods/wheeler_re/SKSE/Plugins/
cargo test --workspace
```

## License

[GPL-3.0-or-later](./LICENSE). Original Wheeler: [BSD-3-Clause](./LICENSES/LICENSE).
