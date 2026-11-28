//! Wheel layout math extracted from `Wheel::Draw` and `Wheeler::UpdateCursorPos*`.

use core::f32::consts::PI;

/// Angles of one wheel entry (radians, screen space: +y is down).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntryLayout {
    pub inner_angle_min: f32,
    pub inner_angle_max: f32,
    pub outer_angle_min: f32,
    pub outer_angle_max: f32,
    /// Angle of the entry's centre line.
    pub center_angle: f32,
    /// Half of the spacing between entries, in radians on the inner circle.
    pub inner_spacing_rad: f32,
    /// Arc tessellation used by the background (C++ `numArcSegments`).
    pub num_arc_segments: u32,
}

/// Lays out entry `index` of `count` (C++ per-entry block of `Wheel::Draw`).
pub fn entry_layout(
    index: usize,
    count: usize,
    inner_radius: f32,
    outer_radius: f32,
    inner_spacing: f32,
) -> EntryLayout {
    let span = 2.0 * PI / count as f32;
    let inner_spacing_rad = inner_spacing / inner_radius / 2.0;
    let outer_spacing_rad = inner_spacing_rad * (inner_radius / outer_radius);
    let i = index as f32;

    let mut inner_min = span * (i - 0.5) + inner_spacing_rad + PI / 2.0;
    let mut inner_max = span * (i + 0.5) - inner_spacing_rad + PI / 2.0;
    let mut outer_min = span * (i - 0.5) + outer_spacing_rad + PI / 2.0;
    let mut outer_max = span * (i + 0.5) - outer_spacing_rad + PI / 2.0;
    if inner_max > PI * 2.0 {
        inner_min -= PI * 2.0;
        inner_max -= PI * 2.0;
    }
    if outer_max > PI * 2.0 {
        outer_min -= PI * 2.0;
        outer_max -= PI * 2.0;
    }

    EntryLayout {
        inner_angle_min: inner_min,
        inner_angle_max: inner_max,
        outer_angle_min: outer_min,
        outer_angle_max: outer_max,
        center_angle: (inner_max - inner_min) / 2.0 + inner_min,
        inner_spacing_rad,
        num_arc_segments: (256.0 * span / (2.0 * PI)) as u32 + 1,
    }
}

impl EntryLayout {
    /// C++ hover test: the cursor angle (from `atan2`, in `(-π, π]`) falls
    /// within the inner arc, also accounting for wrap-around.
    pub fn contains_angle(&self, cursor_angle: f32) -> bool {
        if cursor_angle >= self.inner_angle_min {
            cursor_angle < self.inner_angle_max
        } else {
            let wrapped = cursor_angle + 2.0 * PI;
            wrapped < self.inner_angle_max && wrapped >= self.inner_angle_min
        }
    }

    /// Centre of the entry's slot, halfway between the circles, pushed out by
    /// `radius_mod` (hover expand / bump animation).
    pub fn center(
        &self,
        wheel_center: [f32; 2],
        inner_radius: f32,
        outer_radius: f32,
        radius_mod: f32,
    ) -> [f32; 2] {
        let mid = inner_radius + (outer_radius - inner_radius) / 2.0 + radius_mod;
        let (sin, cos) = self.center_angle.sin_cos();
        [wheel_center[0] + mid * cos, wheel_center[1] + mid * sin]
    }
}

/// C++ `Wheeler::UpdateCursorPosMouse`: add the delta and clamp to `radius`.
pub fn move_cursor_mouse(cursor: [f32; 2], delta: [f32; 2], radius: f32) -> [f32; 2] {
    let pos = [cursor[0] + delta[0], cursor[1] + delta[1]];
    let distance = (pos[0] * pos[0] + pos[1] * pos[1]).sqrt();
    if distance > radius {
        if distance > 0.0 {
            [pos[0] / distance * radius, pos[1] / distance * radius]
        } else {
            [0.0, 0.0]
        }
    } else {
        pos
    }
}

/// C++ `Wheeler::UpdateCursorPosGamepad`; `None` inside the dead zone.
pub fn move_cursor_gamepad(stick: [f32; 2], radius: f32) -> Option<[f32; 2]> {
    const CONTROLLER_DEADZONE: f32 = 0.1;
    if stick[0].abs() <= CONTROLLER_DEADZONE && stick[1].abs() <= CONTROLLER_DEADZONE {
        return None;
    }
    Some([stick[0] * radius, -stick[1] * radius])
}

/// C++ `ImRotate`
#[inline]
pub fn rotate(v: [f32; 2], angle: f32) -> [f32; 2] {
    let (sin, cos) = angle.sin_cos();
    [v[0] * cos - v[1] * sin, v[0] * sin + v[1] * cos]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_entries_start_at_bottom() {
        // Entry 0 is centred straight down (+y) in screen space.
        let l = entry_layout(0, 4, 220.0, 360.0, 0.0);
        assert!((l.center_angle - PI / 2.0).abs() < 1e-5);
        assert!(l.contains_angle(PI / 2.0));
        assert!(!l.contains_angle(-PI / 2.0));
    }

    #[test]
    fn every_angle_hits_exactly_one_entry_without_spacing() {
        let count = 5;
        let layouts: Vec<_> = (0..count)
            .map(|i| entry_layout(i, count, 200.0, 300.0, 0.0))
            .collect();
        for step in 0..360 {
            let angle = (step as f32).to_radians() - PI + 0.001;
            let hits = layouts.iter().filter(|l| l.contains_angle(angle)).count();
            assert_eq!(hits, 1, "angle {angle}");
        }
    }

    #[test]
    fn mouse_cursor_is_clamped() {
        let p = move_cursor_mouse([0.0, 0.0], [30.0, 40.0], 10.0);
        assert!(((p[0] * p[0] + p[1] * p[1]).sqrt() - 10.0).abs() < 1e-5);
        assert_eq!(move_cursor_gamepad([0.05, 0.05], 10.0), None);
        assert_eq!(move_cursor_gamepad([1.0, 1.0], 10.0), Some([10.0, -10.0]));
    }
}
