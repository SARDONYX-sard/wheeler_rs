//! Port of `Config::Styling`, `Config::Animation`, the colour globals of
//! `Config.h`, `Config::ReadStyleConfig` and `Config::OffsetSizingToViewport`.

use std::sync::RwLock;

use crate::ini::{IniExt, IniSource};

pub const STYLE_SETTINGS_PATH: &str = "Data\\SKSE\\Plugins\\wheeler\\Styles.ini";

/// `REFERENCE_HEIGHT`
pub const REFERENCE_HEIGHT: f32 = 1080.0;

/// `IM_COL32(r, g, b, a)` (packed as ABGR).
pub const fn im_col32(r: u32, g: u32, b: u32, a: u32) -> u32 {
    (a << 24) | (b << 16) | (g << 8) | r
}

pub const C_SKYRIMGREY: u32 = im_col32(255, 255, 255, 100);
pub const C_SKYRIMWHITE: u32 = im_col32(255, 255, 255, 255);
pub const C_BLACK: u32 = im_col32(0, 0, 0, 255);
pub const C_SKYRIMDARKGREY_MENUBACKGROUND: u32 = im_col32(0, 0, 0, 125);
pub const C_QUARTERTRANSPARENT: u32 = im_col32(255, 255, 255, 63);
pub const C_HALFTRANSPARENT: u32 = im_col32(255, 255, 255, 127);
pub const C_TRIQUARTERTRANSPARENT: u32 = im_col32(255, 255, 255, 191);
pub const C_VOID: u32 = im_col32(255, 255, 255, 0);

/// `Config::WidgetAlignment`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetAlignment {
    Left = 0,
    Center = 1,
}

/// `Config::Animation`
#[derive(Debug, Clone, Copy)]
pub struct Animation {
    pub entry_highlight_expand_time: f32,
    pub entry_highlight_retract_time: f32,
    pub entry_highlight_expand_scale: f32,
    pub entry_input_bump_time: f32,
    pub entry_input_bump_scale: f32,
    pub toggle_vertical_fade_distance: f32,
    pub toggle_horizontal_fade_distance: f32,
    /// Fade in/out time; 0 disables fading.
    pub fade_time: f32,
    pub snappy_cursor_indicator: bool,
}

/// `Config::Styling::Wheel`
#[derive(Debug, Clone, Copy)]
pub struct WheelStyle {
    pub use_geometric_primitive_for_background_texture: bool,
    pub wheel_background_texture_scale: f32,
    /// Distance from the cursor indicator to the inner circle.
    pub cursor_indicator_dist: f32,
    pub cursor_indicator_arc_width: f32,
    pub cursor_indicator_arc_angle: f32,
    pub cursor_indicator_triangle_side_length: f32,
    pub cursor_indicator_color: u32,
    pub cursor_indicator_inward_facing: bool,
    pub wheel_indicator_offset_x: f32,
    pub wheel_indicator_offset_y: f32,
    pub wheel_indicator_size: f32,
    pub wheel_indicator_spacing: f32,
    pub wheel_indicator_active_color: u32,
    pub wheel_indicator_inactive_color: u32,
    pub wheel_indicator_alignment: WidgetAlignment,
    pub inner_circle_radius: f32,
    pub outer_circle_radius: f32,
    pub inner_spacing: f32,
    pub hovered_color_begin: u32,
    pub hovered_color_end: u32,
    pub unhovered_color_begin: u32,
    pub unhovered_color_end: u32,
    pub active_arc_color_begin: u32,
    pub active_arc_color_end: u32,
    pub inactive_arc_color_begin: u32,
    pub inactive_arc_color_end: u32,
    pub active_arc_width: f32,
    pub blur_on_open: bool,
    pub slow_time_scale: f32,
    /// Offset of the wheel centre from the screen centre; everything else is
    /// relative to the wheel centre.
    pub center_offset_x: f32,
    pub center_offset_y: f32,
    pub text_color: u32,
    pub text_shadow_color: u32,
}

/// `OffsetX / OffsetY / Size` (text) or `OffsetX / OffsetY / Scale` (texture).
#[derive(Debug, Clone, Copy)]
pub struct Placement {
    pub offset_x: f32,
    pub offset_y: f32,
    /// Font size for text blocks, scale for texture blocks.
    pub size: f32,
}

/// `Config::Styling::Item::Highlight::Desc`
#[derive(Debug, Clone, Copy)]
pub struct DescStyle {
    pub offset_x: f32,
    pub offset_y: f32,
    pub size: f32,
    pub line_length: f32,
    pub line_spacing: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct Style {
    pub wheel: WheelStyle,
    /// `Styling::Entry::Highlight::Text`
    pub entry_highlight_text: Placement,
    /// `Styling::Item::Highlight::Texture` (`size` = scale)
    pub item_highlight_texture: Placement,
    pub item_highlight_text: Placement,
    pub item_highlight_desc: DescStyle,
    /// `Styling::Item::Highlight::StatIcon` (`size` = scale)
    pub item_highlight_stat_icon: Placement,
    pub item_highlight_stat_text: Placement,
    /// `Styling::Item::Slot::Texture` (`size` = scale)
    pub item_slot_texture: Placement,
    pub item_slot_text: Placement,
    /// `Styling::Item::Slot::BackgroundTexture::Scale`
    pub item_slot_background_texture_scale: f32,
    pub animation: Animation,
}

impl Style {
    pub const DEFAULT: Self = Self {
        wheel: WheelStyle {
            use_geometric_primitive_for_background_texture: false,
            wheel_background_texture_scale: 1.0,
            cursor_indicator_dist: 10.0,
            cursor_indicator_arc_width: 3.0,
            cursor_indicator_arc_angle: 2.0 * core::f32::consts::PI / 12.0,
            cursor_indicator_triangle_side_length: 5.0,
            cursor_indicator_color: C_SKYRIMWHITE,
            cursor_indicator_inward_facing: true,
            wheel_indicator_offset_x: 260.0,
            wheel_indicator_offset_y: 340.0,
            wheel_indicator_size: 10.0,
            wheel_indicator_spacing: 25.0,
            wheel_indicator_active_color: C_SKYRIMWHITE,
            wheel_indicator_inactive_color: C_SKYRIMGREY,
            wheel_indicator_alignment: WidgetAlignment::Left,
            inner_circle_radius: 220.0,
            outer_circle_radius: 360.0,
            inner_spacing: 10.0,
            hovered_color_begin: C_QUARTERTRANSPARENT,
            hovered_color_end: C_HALFTRANSPARENT,
            unhovered_color_begin: C_SKYRIMDARKGREY_MENUBACKGROUND,
            unhovered_color_end: C_SKYRIMDARKGREY_MENUBACKGROUND,
            active_arc_color_begin: C_SKYRIMWHITE,
            active_arc_color_end: C_SKYRIMWHITE,
            inactive_arc_color_begin: C_SKYRIMGREY,
            inactive_arc_color_end: C_SKYRIMGREY,
            active_arc_width: 7.0,
            blur_on_open: true,
            slow_time_scale: 0.1,
            center_offset_x: 450.0,
            center_offset_y: 0.0,
            text_color: C_SKYRIMWHITE,
            text_shadow_color: C_BLACK,
        },
        entry_highlight_text: Placement {
            offset_x: 0.0,
            offset_y: -130.0,
            size: 27.0,
        },
        item_highlight_texture: Placement {
            offset_x: 0.0,
            offset_y: -50.0,
            size: 0.2,
        },
        item_highlight_text: Placement {
            offset_x: 0.0,
            offset_y: 20.0,
            size: 35.0,
        },
        item_highlight_desc: DescStyle {
            offset_x: 0.0,
            offset_y: 50.0,
            size: 30.0,
            line_length: 500.0,
            line_spacing: 5.0,
        },
        item_highlight_stat_icon: Placement {
            offset_x: 0.0,
            offset_y: 0.0,
            size: 0.2,
        },
        item_highlight_stat_text: Placement {
            offset_x: 0.0,
            offset_y: 0.0,
            size: 35.0,
        },
        item_slot_texture: Placement {
            offset_x: 0.0,
            offset_y: -25.0,
            size: 0.1,
        },
        item_slot_text: Placement {
            offset_x: 0.0,
            offset_y: 10.0,
            size: 30.0,
        },
        item_slot_background_texture_scale: 0.1,
        animation: Animation {
            entry_highlight_expand_time: 0.2,
            entry_highlight_retract_time: 0.2,
            entry_highlight_expand_scale: 0.15,
            entry_input_bump_time: 0.1,
            entry_input_bump_scale: -0.1,
            toggle_vertical_fade_distance: 0.0,
            toggle_horizontal_fade_distance: 0.0,
            fade_time: 0.08,
            snappy_cursor_indicator: false,
        },
    };
}

static STYLE: RwLock<Style> = RwLock::new(Style::DEFAULT);

/// Snapshot of the current style (cheap `Copy`).
pub fn style() -> Style {
    *STYLE.read().unwrap_or_else(|e| e.into_inner())
}

/// Replaces the whole style (tools, tests).
pub fn set_style(style: Style) {
    *STYLE.write().unwrap_or_else(|e| e.into_inner()) = style;
}

/// C++ `Config::ReadStyleConfig`: overrides the current style with every key
/// present in `cfg` (the contents of [`STYLE_SETTINGS_PATH`]).
pub fn read_style_config(cfg: &dyn IniSource) {
    let mut guard = STYLE.write().unwrap_or_else(|e| e.into_inner());
    let s = &mut *guard;
    // Start from the defaults: missing keys must not keep a value that
    // `offset_sizing_to_viewport` already scaled (re-read on dMenu updates).
    *s = Style::DEFAULT;

    let f = |sec: &str, key: &str, v: &mut f32| *v = cfg.f32_or(sec, key, *v);
    let u = |sec: &str, key: &str, v: &mut u32| *v = cfg.u32_or(sec, key, *v);
    let b = |sec: &str, key: &str, v: &mut bool| *v = cfg.bool_or(sec, key, *v);

    const W: &str = "Styling.Wheel";
    let w = &mut s.wheel;
    f(W, "CursorIndicatorDist", &mut w.cursor_indicator_dist);
    f(
        W,
        "CusorIndicatorArcWidth",
        &mut w.cursor_indicator_arc_width,
    );
    f(
        W,
        "CursorIndicatorArcAngle",
        &mut w.cursor_indicator_arc_angle,
    );
    f(
        W,
        "CursorIndicatorTriangleSideLength",
        &mut w.cursor_indicator_triangle_side_length,
    );
    u(W, "CursorIndicatorColor", &mut w.cursor_indicator_color);
    b(
        W,
        "CursorIndicatorInwardFacing",
        &mut w.cursor_indicator_inward_facing,
    );
    b(
        W,
        "UseGeometricPrimitiveForBackgroundTexture",
        &mut w.use_geometric_primitive_for_background_texture,
    );
    f(W, "WheelIndicatorOffsetX", &mut w.wheel_indicator_offset_x);
    f(W, "WheelIndicatorOffsetY", &mut w.wheel_indicator_offset_y);
    f(W, "WheelIndicatorSize", &mut w.wheel_indicator_size);
    f(W, "WheelIndicatorSpacing", &mut w.wheel_indicator_spacing);
    u(
        W,
        "WheelIndicatorActiveColor",
        &mut w.wheel_indicator_active_color,
    );
    u(
        W,
        "WheelIndicatorInactiveColor",
        &mut w.wheel_indicator_inactive_color,
    );
    f(W, "InnerCircleRadius", &mut w.inner_circle_radius);
    f(W, "OuterCircleRadius", &mut w.outer_circle_radius);
    f(W, "InnerSpacing", &mut w.inner_spacing);
    u(W, "HoveredColorBegin", &mut w.hovered_color_begin);
    u(W, "HoveredColorEnd", &mut w.hovered_color_end);
    u(W, "UnhoveredColorBegin", &mut w.unhovered_color_begin);
    u(W, "UnhoveredColorEnd", &mut w.unhovered_color_end);
    u(W, "ActiveArcColorBegin", &mut w.active_arc_color_begin);
    u(W, "ActiveArcColorEnd", &mut w.active_arc_color_end);
    u(W, "InActiveArcColorBegin", &mut w.inactive_arc_color_begin);
    u(W, "InActiveArcColorEnd", &mut w.inactive_arc_color_end);
    f(W, "ActiveArcWidth", &mut w.active_arc_width);
    b(W, "BlurOnOpen", &mut w.blur_on_open);
    f(W, "SlowTimeScale", &mut w.slow_time_scale);
    f(W, "CenterOffsetX", &mut w.center_offset_x);
    f(W, "CenterOffsetY", &mut w.center_offset_y);
    u(W, "TextShadowColor", &mut w.text_shadow_color);
    u(W, "TextColor", &mut w.text_color);

    let placement = |sec: &str, size_key: &str, p: &mut Placement| {
        f(sec, "OffsetX", &mut p.offset_x);
        f(sec, "OffsetY", &mut p.offset_y);
        f(sec, size_key, &mut p.size);
    };
    placement(
        "Styling.Entry.Highlight.Text",
        "Size",
        &mut s.entry_highlight_text,
    );
    placement(
        "Styling.Item.Highlight.Texture",
        "Scale",
        &mut s.item_highlight_texture,
    );
    placement(
        "Styling.Item.Highlight.Text",
        "Size",
        &mut s.item_highlight_text,
    );
    const DESC: &str = "Styling.Item.Highlight.Desc";
    let d = &mut s.item_highlight_desc;
    f(DESC, "OffsetX", &mut d.offset_x);
    f(DESC, "OffsetY", &mut d.offset_y);
    f(DESC, "Size", &mut d.size);
    f(DESC, "LineLength", &mut d.line_length);
    f(DESC, "LineSpacing", &mut d.line_spacing);
    placement(
        "Styling.Item.Highlight.StatIcon",
        "Scale",
        &mut s.item_highlight_stat_icon,
    );
    placement(
        "Styling.Item.Highlight.StatText",
        "Size",
        &mut s.item_highlight_stat_text,
    );
    placement(
        "Styling.Item.Slot.Texture",
        "Scale",
        &mut s.item_slot_texture,
    );
    placement("Styling.Item.Slot.Text", "Size", &mut s.item_slot_text);
    f(
        "Styling.Item.Slot.BackgroundTexture",
        "Scale",
        &mut s.item_slot_background_texture_scale,
    );

    const A: &str = "Animation";
    let a = &mut s.animation;
    f(
        A,
        "EntryHighlightExpandTime",
        &mut a.entry_highlight_expand_time,
    );
    f(
        A,
        "EntryHighlightRetractTime",
        &mut a.entry_highlight_retract_time,
    );
    f(
        A,
        "EntryHighlightExpandScale",
        &mut a.entry_highlight_expand_scale,
    );
    f(A, "EntryInputBumpScale", &mut a.entry_input_bump_scale);
    f(A, "EntryInputBumpTime", &mut a.entry_input_bump_time);
    f(
        A,
        "ToggleHorizontalFadeDistance",
        &mut a.toggle_horizontal_fade_distance,
    );
    f(
        A,
        "ToggleVerticalFadeDistance",
        &mut a.toggle_vertical_fade_distance,
    );
    f(A, "FadeTime", &mut a.fade_time);
}

/// C++ `Config::OffsetSizingToViewport`: scales every pixel-based setting by
/// `viewport_height / 1080`.
pub fn offset_sizing_to_viewport(viewport_height: f32) {
    let scale = viewport_height / REFERENCE_HEIGHT;
    if !(scale.is_finite() && scale > 0.0) {
        return;
    }
    crate::control::scale_cursor_radius(scale);

    let mut guard = STYLE.write().unwrap_or_else(|e| e.into_inner());
    let s = &mut *guard;
    let w = &mut s.wheel;
    for v in [
        &mut w.cursor_indicator_dist,
        &mut w.cursor_indicator_arc_width,
        &mut w.cursor_indicator_triangle_side_length,
        &mut w.wheel_indicator_offset_x,
        &mut w.wheel_indicator_offset_y,
        &mut w.wheel_indicator_size,
        &mut w.wheel_indicator_spacing,
        &mut w.inner_circle_radius,
        &mut w.outer_circle_radius,
        &mut w.inner_spacing,
        &mut w.active_arc_width,
        &mut w.center_offset_x,
        &mut w.center_offset_y,
    ] {
        *v *= scale;
    }
    for p in [
        &mut s.entry_highlight_text,
        &mut s.item_highlight_texture,
        &mut s.item_highlight_text,
        &mut s.item_highlight_stat_icon,
        &mut s.item_highlight_stat_text,
        &mut s.item_slot_texture,
        &mut s.item_slot_text,
    ] {
        p.offset_x *= scale;
        p.offset_y *= scale;
        p.size *= scale;
    }
    let d = &mut s.item_highlight_desc;
    for v in [
        &mut d.offset_x,
        &mut d.offset_y,
        &mut d.size,
        &mut d.line_length,
        &mut d.line_spacing,
    ] {
        *v *= scale;
    }
    s.item_slot_background_texture_scale *= scale;
    s.animation.toggle_vertical_fade_distance *= scale;
    s.animation.toggle_horizontal_fade_distance *= scale;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ini::MemoryIni;

    #[test]
    fn style_ini_overrides_defaults() {
        let ini = MemoryIni::parse(
            "[Styling.Wheel]
InnerCircleRadius = 100
[Styling.Item.Slot.Text]
Size = 12
",
        );
        read_style_config(&ini);
        let s = style();
        assert_eq!(s.wheel.inner_circle_radius, 100.0);
        assert_eq!(s.item_slot_text.size, 12.0);
        assert_eq!(
            s.wheel.outer_circle_radius,
            Style::DEFAULT.wheel.outer_circle_radius
        );

        // Re-reading after scaling starts from the defaults again.
        offset_sizing_to_viewport(2160.0);
        assert_eq!(style().wheel.inner_circle_radius, 200.0);
        read_style_config(&ini);
        offset_sizing_to_viewport(2160.0);
        let s = style();
        assert_eq!(s.wheel.inner_circle_radius, 200.0);
        assert_eq!(
            s.wheel.outer_circle_radius,
            Style::DEFAULT.wheel.outer_circle_radius * 2.0
        );
    }
}
