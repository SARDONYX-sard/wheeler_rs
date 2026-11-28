//! Port of `Rendering/Drawer.cpp`: primitives, text and textures drawn into
//! the current ImGui window draw list.
//!
//! Every function must be called between `Begin*`/`End*` of an ImGui window
//! (Wheeler draws inside its popup).

use imgui::sys::{self, ImVec2};

use wheeler_core::style;

/// C++ `DrawArgs`
#[derive(Debug, Clone, Copy)]
pub struct DrawArgs {
    pub alpha_mult: f64,
    pub scale_mult: f32,
    pub rotation_offset: f32,
    pub translation_offset: [f32; 2],
    pub center_object: bool,
}

impl Default for DrawArgs {
    fn default() -> Self {
        Self {
            alpha_mult: 1.0,
            scale_mult: 1.0,
            rotation_offset: 0.0,
            translation_offset: [0.0, 0.0],
            center_object: true,
        }
    }
}

/// C++ `Utils::Color::MultAlpha`
#[inline]
pub fn mult_alpha(color: u32, mult: f64) -> u32 {
    (color & 0x00FF_FFFF) | ((((color >> 24) as f64 * mult) as u32) << 24)
}

#[inline]
const fn v2(x: f32, y: f32) -> ImVec2 {
    ImVec2 { x, y }
}

#[inline]
fn draw_list() -> *mut sys::ImDrawList {
    unsafe { sys::igGetWindowDrawList() }
}

/// ImGui internal `GetDefaultFont()`: `io.FontDefault` or the first font.
fn default_font() -> *mut sys::ImFont {
    unsafe {
        let io = &*sys::igGetIO();
        if !io.FontDefault.is_null() {
            return io.FontDefault;
        }
        let fonts = &*io.Fonts;
        if fonts.Fonts.Size > 0 {
            *fonts.Fonts.Data
        } else {
            sys::igGetFont()
        }
    }
}

fn calc_text_size(text: &str) -> [f32; 2] {
    let mut out = v2(0.0, 0.0);
    unsafe {
        sys::igCalcTextSize(
            &mut out,
            text.as_ptr().cast(),
            text.as_ptr().add(text.len()).cast(),
            false,
            -1.0,
        );
    }
    [out.x, out.y]
}

/// C++ `Drawer::draw_text`
pub fn draw_text(
    x: f32,
    y: f32,
    text: &str,
    color: u32,
    font_size: f32,
    args: DrawArgs,
    center_text: bool,
) {
    // C++ replaces null/empty text with this placeholder to avoid a CTD.
    let text = if text.is_empty() {
        "Already consumed out"
    } else {
        text
    };
    if args.alpha_mult == 0.0 || font_size == 0.0 {
        return;
    }
    let color = mult_alpha(color, args.alpha_mult);

    let mut pos = [x, y];
    if center_text {
        let ratio = font_size / unsafe { sys::igGetFontSize() };
        let [w, h] = calc_text_size(text);
        pos[0] -= w * ratio * 0.5;
        pos[1] -= h * ratio * 0.5;
    }

    let shadow_offset = font_size * 0.05;
    let shadow = mult_alpha(style::style().wheel.text_shadow_color, args.alpha_mult);
    let font = default_font();
    let list = draw_list();
    let (begin, end) = (text.as_ptr().cast(), unsafe {
        text.as_ptr().add(text.len()).cast()
    });
    unsafe {
        sys::ImDrawList_AddText_FontPtr(
            list,
            font,
            font_size,
            v2(pos[0] + shadow_offset, pos[1] + shadow_offset),
            shadow,
            begin,
            end,
            0.0,
            core::ptr::null(),
        );
        sys::ImDrawList_AddText_FontPtr(
            list,
            font,
            font_size,
            v2(pos[0], pos[1]),
            color,
            begin,
            end,
            0.0,
            core::ptr::null(),
        );
    }
}

/// C++ `split_text_lines_utf8`: splits into lines of roughly equal byte length
/// without breaking words or UTF-8 sequences.
fn split_text_lines_utf8(text: &str, max_width: f32, font_size: f32) -> Vec<&str> {
    let raw_length = calc_text_size(text)[0] * (font_size / unsafe { sys::igGetFontSize() });
    let num_lines = ((raw_length / max_width).ceil() as usize).max(1);
    let bytes = text.as_bytes();
    // C++ performs integer division before `ceil`.
    let line_length = text.len() / num_lines;

    let mut lines = Vec::new();
    let mut curr = 0usize;
    for i in 0..num_lines {
        if i == num_lines - 1 {
            if curr < text.len() {
                lines.push(&text[curr..]);
            }
            break;
        }
        let mut end = (curr + line_length).min(text.len());
        while end < text.len() && !text.is_char_boundary(end) {
            end -= 1;
        }
        let search_end = (end + 1).min(text.len());
        let last_space = bytes[..search_end].iter().rposition(|&b| b == b' ');
        match last_space {
            Some(space) if space > curr => {
                lines.push(&text[curr..space]);
                curr = space + 1;
            }
            _ => {
                if end <= curr {
                    // Cannot make progress on a single long token: emit the rest.
                    lines.push(&text[curr..]);
                    break;
                }
                lines.push(&text[curr..end]);
                curr = end;
            }
        }
    }
    lines
}

/// C++ `Drawer::draw_text_block`
#[allow(clippy::too_many_arguments)]
pub fn draw_text_block(
    x: f32,
    y: f32,
    text: &str,
    color: u32,
    font_size: f32,
    line_spacing: f32,
    line_length: f32,
    args: DrawArgs,
) {
    if text.is_empty() || args.alpha_mult == 0.0 || font_size == 0.0 {
        return;
    }
    let font_height = calc_text_size("t")[1] * font_size / unsafe { sys::igGetFontSize() };
    let mut line_y = y;
    for line in split_text_lines_utf8(text, line_length, font_size) {
        draw_text(x, line_y, line, color, font_size, args, true);
        line_y += font_height + line_spacing;
    }
}

/// C++ `Drawer::draw_texture`: a (possibly rotated) textured quad centred on
/// `center + offset`.
///
/// `texture` is only passed on as the opaque ImGui texture id, never read.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn draw_texture(
    texture: *mut core::ffi::c_void,
    center: [f32; 2],
    offset_x: f32,
    offset_y: f32,
    size: [f32; 2],
    color: u32,
    args: DrawArgs,
) {
    if texture.is_null() {
        return;
    }
    let c = [center[0] + offset_x, center[1] + offset_y];
    let (sin_a, cos_a) = args.rotation_offset.sin_cos();
    let rot = |x: f32, y: f32| v2(c[0] + x * cos_a - y * sin_a, c[1] + x * sin_a + y * cos_a);
    let (hw, hh) = (size[0] * 0.5, size[1] * 0.5);
    let color = mult_alpha(color, args.alpha_mult);
    unsafe {
        sys::ImDrawList_AddImageQuad(
            draw_list(),
            texture,
            rot(-hw, -hh),
            rot(hw, -hh),
            rot(hw, hh),
            rot(-hw, hh),
            v2(0.0, 0.0),
            v2(1.0, 0.0),
            v2(1.0, 1.0),
            v2(0.0, 1.0),
            color,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn arc_impl(
    center: [f32; 2],
    radius_min: f32,
    radius_max: f32,
    inner_ang_min: f32,
    inner_ang_max: f32,
    outer_ang_min: f32,
    outer_ang_max: f32,
    inner_color: u32,
    outer_color: u32,
    segments: u32,
) {
    let segments = segments.max(1);
    let step_inner = (inner_ang_max - inner_ang_min) / segments as f32;
    let step_outer = (outer_ang_max - outer_ang_min) / segments as f32;
    unsafe {
        // Same value as `GetDrawListSharedData()->TexUvWhitePixel`.
        let mut uv = v2(0.0, 0.0);
        sys::igGetFontTexUvWhitePixel(&mut uv);
        let list = draw_list();
        sys::ImDrawList_PrimReserve(list, (segments * 6) as i32, ((segments + 1) * 2) as i32);
        for seg in 0..=segments {
            let (sin_i, cos_i) = (inner_ang_min + step_inner * seg as f32).sin_cos();
            let (sin_o, cos_o) = (outer_ang_min + step_outer * seg as f32).sin_cos();
            if seg < segments {
                let base = (*list)._VtxCurrentIdx;
                for k in [0, 2, 1, 3, 2, 1] {
                    sys::ImDrawList_PrimWriteIdx(list, (base + k) as sys::ImDrawIdx);
                }
            }
            sys::ImDrawList_PrimWriteVtx(
                list,
                v2(
                    center[0] + cos_i * radius_min,
                    center[1] + sin_i * radius_min,
                ),
                uv,
                inner_color,
            );
            sys::ImDrawList_PrimWriteVtx(
                list,
                v2(
                    center[0] + cos_o * radius_max,
                    center[1] + sin_o * radius_max,
                ),
                uv,
                outer_color,
            );
        }
    }
}

/// C++ `Drawer::draw_arc`
#[allow(clippy::too_many_arguments)]
pub fn draw_arc(
    center: [f32; 2],
    radius_min: f32,
    radius_max: f32,
    inner_ang_min: f32,
    inner_ang_max: f32,
    outer_ang_min: f32,
    outer_ang_max: f32,
    color: u32,
    segments: u32,
    args: DrawArgs,
) {
    let color = mult_alpha(color, args.alpha_mult);
    arc_impl(
        center,
        radius_min,
        radius_max,
        inner_ang_min,
        inner_ang_max,
        outer_ang_min,
        outer_ang_max,
        color,
        color,
        segments,
    );
}

/// C++ `Drawer::draw_arc_gradient`: `color_start` on the inner edge,
/// `color_end` on the outer edge.
#[allow(clippy::too_many_arguments)]
pub fn draw_arc_gradient(
    center: [f32; 2],
    radius_min: f32,
    radius_max: f32,
    inner_ang_min: f32,
    inner_ang_max: f32,
    outer_ang_min: f32,
    outer_ang_max: f32,
    color_start: u32,
    color_end: u32,
    segments: u32,
    args: DrawArgs,
) {
    arc_impl(
        center,
        radius_min,
        radius_max,
        inner_ang_min,
        inner_ang_max,
        outer_ang_min,
        outer_ang_max,
        mult_alpha(color_start, args.alpha_mult),
        mult_alpha(color_end, args.alpha_mult),
        segments,
    );
}

/// C++ `Drawer::draw_circle_filled`
pub fn draw_circle_filled(
    center: [f32; 2],
    radius: f32,
    color: u32,
    segments: i32,
    args: DrawArgs,
) {
    let color = mult_alpha(color, args.alpha_mult);
    unsafe {
        sys::ImDrawList_AddCircleFilled(
            draw_list(),
            v2(center[0], center[1]),
            radius,
            color,
            segments,
        )
    };
}

/// C++ `Drawer::draw_triangle_filled`
pub fn draw_triangle_filled(p1: [f32; 2], p2: [f32; 2], p3: [f32; 2], color: u32, args: DrawArgs) {
    let color = mult_alpha(color, args.alpha_mult);
    unsafe {
        sys::ImDrawList_AddTriangleFilled(
            draw_list(),
            v2(p1[0], p1[1]),
            v2(p2[0], p2[1]),
            v2(p3[0], p3[1]),
            color,
        );
    }
}
