//! Headless ImGui frames for tests: builds a context with a font atlas, runs
//! one frame inside a full-screen window and returns statistics about the
//! produced draw data. No GPU is involved.

use std::sync::Mutex;

/// ImGui has one global current context, so frames must not overlap.
static LOCK: Mutex<()> = Mutex::new(());

/// Summary of one rendered frame.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FrameStats {
    pub vertices: usize,
    pub indices: usize,
    pub draw_commands: usize,
    /// Distinct texture ids referenced by draw commands.
    pub textures: Vec<usize>,
}

/// Runs `draw` inside one frame of a fresh context with the given display size.
pub fn render_frame(display_size: [f32; 2], draw: impl FnOnce(&imgui::Ui)) -> FrameStats {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let mut ctx = imgui::Context::create();
    ctx.set_ini_filename(None);
    {
        let fonts = ctx.fonts();
        let _ = fonts.build_rgba32_texture();
        fonts.tex_id = imgui::TextureId::new(1);
    }
    let io = ctx.io_mut();
    io.display_size = display_size;
    io.delta_time = 1.0 / 60.0;

    let ui = ctx.new_frame();
    ui.window("headless")
        .position([0.0, 0.0], imgui::Condition::Always)
        .size(display_size, imgui::Condition::Always)
        .flags(imgui::WindowFlags::NO_DECORATION | imgui::WindowFlags::NO_BACKGROUND)
        .build(|| {
            unsafe {
                imgui::sys::ImDrawList_PushClipRectFullScreen(imgui::sys::igGetWindowDrawList())
            };
            draw(ui);
            unsafe { imgui::sys::ImDrawList_PopClipRect(imgui::sys::igGetWindowDrawList()) };
        });

    let data = ctx.render();
    let mut stats = FrameStats {
        vertices: data.total_vtx_count as usize,
        indices: data.total_idx_count as usize,
        ..FrameStats::default()
    };
    // imgui-rs builds a slice from a null pointer when there are no lists.
    if data.draw_lists_count() == 0 {
        return stats;
    }
    for list in data.draw_lists() {
        for cmd in list.commands() {
            if let imgui::DrawCmd::Elements { cmd_params, .. } = cmd {
                stats.draw_commands += 1;
                let id = cmd_params.texture_id.id();
                if !stats.textures.contains(&id) {
                    stats.textures.push(id);
                }
            }
        }
    }
    stats
}
