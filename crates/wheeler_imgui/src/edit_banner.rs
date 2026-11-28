//! "EDIT MODE" banner with key hints, drawn above the wheel while it is open
//! in the inventory / magic menu. The C++ Wheeler gave no visible cue of
//! edit mode.

use wheeler_core::control::{Action, ControlConfig};
use wheeler_core::style::im_col32;

use crate::drawer::{self, DrawArgs};
use crate::settings::describe;

/// Title color (gold).
const TITLE_COLOR: u32 = im_col32(255, 205, 90, 255);
const HINT_COLOR: u32 = im_col32(255, 255, 255, 220);
/// Sizes at 1080p.
const TITLE_SIZE: f32 = 34.0;
const HINT_SIZE: f32 = 20.0;
const GAP: f32 = 24.0;
/// Left margin of the hint lines.
const MARGIN: f32 = 48.0;

/// What each edit-mode binding does.
const HINTS: [(Action, &str); 8] = [
    (Action::ActivatePrimary, "Add the item hovered in the menu"),
    (
        Action::ActivateSecondary,
        "Remove item / empty entry / empty wheel",
    ),
    (Action::AddEmptyEntry, "Add entry"),
    (Action::AddWheel, "Add wheel"),
    (Action::MoveEntryForward, "Move entry forward"),
    (Action::MoveEntryBack, "Move entry back"),
    (Action::MoveWheelForward, "Move wheel forward"),
    (Action::MoveWheelBack, "Move wheel back"),
];

/// `key: what it does` lines for bound actions (keyboard / gamepad).
pub fn hint_lines(control: &ControlConfig) -> Vec<String> {
    HINTS
        .iter()
        .filter_map(|&(action, text)| {
            let keys: Vec<String> = [control.mkb.get(action), control.gamepad.get(action)]
                .iter()
                .filter(|c| !c.is_none())
                .map(describe)
                .collect();
            (!keys.is_empty()).then(|| format!("{}: {text}", keys.join(" / ")))
        })
        .collect()
}

/// Draws "EDIT MODE" above the wheel and the hint lines down the left
/// edge of the screen, centered on the wheel. Sizes are 1080p pixels times
/// `scale` (viewport height / 1080); `outer_radius` is already scaled.
pub fn draw(center: [f32; 2], outer_radius: f32, lines: &[String], scale: f32, args: DrawArgs) {
    let title = TITLE_SIZE * scale;
    let title_y = center[1] - outer_radius - GAP * scale - title * 0.5;
    drawer::draw_text(
        center[0],
        title_y,
        "EDIT MODE",
        TITLE_COLOR,
        title,
        args,
        true,
    );

    let hint = HINT_SIZE * scale;
    let line_height = hint * 1.25;
    let mut y = center[1] - line_height * lines.len() as f32 / 2.0;
    for line in lines {
        drawer::draw_text(MARGIN * scale, y, line, HINT_COLOR, hint, args, false);
        y += line_height;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::headless::render_frame;

    #[test]
    fn hints_follow_bindings() {
        let lines = hint_lines(&ControlConfig::DEFAULT);
        assert_eq!(
            lines[0],
            "Mouse Left / RB: Add the item hovered in the menu"
        );
        assert!(lines.iter().any(|l| l == "N: Add wheel"));
        assert_eq!(lines.len(), 8);
    }

    #[test]
    fn draws() {
        let lines = hint_lines(&ControlConfig::DEFAULT);
        let stats = render_frame([1920.0, 1080.0], |_| {
            draw([960.0, 540.0], 360.0, &lines, 1.0, DrawArgs::default())
        });
        assert!(stats.vertices > 0);
    }
}
