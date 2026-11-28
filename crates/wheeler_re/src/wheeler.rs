//! Port of `Wheeler/Wheeler.cpp` (the wheel-menu controller).
//!
//! Owns the wheels and the open/close/fade state machine; drawing and entry
//! logic live in `wheeler_imgui::wheel`.
//! TODO: `WheelItem*` ports, serialization, edit-mode vanilla menu hiding.

use std::sync::{LazyLock, Mutex, MutexGuard};

use imgui::Ui;
use libskyrim::re::{PlayerCharacter, TESObjectREFR, UI, UIBlurManager};
use libskyrim::relocation::RelocationID;

use wheeler_core::control;
use wheeler_core::sound::Sound;
use wheeler_core::style::{self, C_SKYRIMWHITE, WidgetAlignment};
use wheeler_core::texts::{self, TextType};
use wheeler_imgui::drawer::{self, DrawArgs};
use wheeler_imgui::image::IconImageType;
use wheeler_imgui::wheel::WheelHost;

use crate::host::{GameHost, ItemCtx, Wheel};

const WHEEL_WINDOW_ID: &core::ffi::CStr = c"##Wheeler";

const MENU_INVENTORY: &str = "InventoryMenu";
const MENU_MAGIC: &str = "MagicMenu";
const CONFLICTING_MENUS: &[&str] = &[
    "Book Menu",
    "BarterMenu",
    "Crafting Menu",
    "Journal Menu",
    "LevelUp Menu",
    "Lockpicking Menu",
    "Loading Menu",
    "Main Menu",
    "MapMenu",
    "RaceSex Menu",
    "Sleep/Wait Menu",
    "StatsMenu",
    "TweenMenu",
    "Console",
    "Dialogue Menu",
    "GiftMenu",
    "Mod Manager Menu",
    "ContainerMenu",
    "LootMenu", // QuickLoot
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WheelState {
    Opened,
    Closed,
    Opening,
    Closing,
}

struct Wheeler {
    state: WheelState,
    edit_mode: bool,
    cursor_pos: [f32; 2],
    wheels: Vec<Wheel>,
    active_wheel_idx: i32,
    open_timer: f32,
    close_timer: f32,
}

/// Set by items that need the wheel closed (C++ `Wheeler::TryCloseWheeler`
/// called from inside an activation, which already holds the wheel lock).
static CLOSE_REQUESTED: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

/// Asks the wheel to close after the current activation.
pub fn request_close() {
    CLOSE_REQUESTED.store(true, core::sync::atomic::Ordering::Release);
}

fn handle_close_request(w: &mut Wheeler) {
    if CLOSE_REQUESTED.swap(false, core::sync::atomic::Ordering::AcqRel) {
        w.try_close();
    }
}

static WHEELER: LazyLock<Mutex<Wheeler>> = LazyLock::new(|| {
    Mutex::new(Wheeler {
        state: WheelState::Closed,
        edit_mode: false,
        cursor_pos: [0.0, 0.0],
        wheels: Vec::new(),
        active_wheel_idx: 0,
        open_timer: 0.0,
        close_timer: 0.0,
    })
});

fn lock() -> MutexGuard<'static, Wheeler> {
    WHEELER.lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------
// Engine helpers (C++ `Utils::Time`, `RE::PlaySound`)

/// C++ `Utils::Time::GGTM` (global time multiplier)
fn get_time_multiplier() -> f32 {
    let ptr = RelocationID::new(511883, 388443).address() as *const f32;
    if ptr.is_null() {
        1.0
    } else {
        unsafe { ptr.read_volatile() }
    }
}

/// C++ `Utils::Time::SGTM`
fn set_time_multiplier(value: f32) {
    let ptr = RelocationID::new(511883, 388443).address() as *mut f32;
    if !ptr.is_null() {
        unsafe { ptr.write_volatile(value) };
    }
}

fn play_sound(sound: Sound) {
    GameHost.play_sound(sound);
}

fn player_3d_loaded() -> bool {
    let pc = PlayerCharacter::get_singleton();
    !pc.is_null() && unsafe { (*pc.cast::<TESObjectREFR>()).is_3d_loaded() }
}

fn ui() -> Option<&'static mut UI> {
    unsafe { UI::get_singleton().as_mut() }
}

fn blur(increment: bool) {
    if let Some(blur) = unsafe { UIBlurManager::get_singleton().as_mut() } {
        if increment {
            blur.increment_blur_count();
        } else {
            blur.decrement_blur_count();
        }
    }
}

/// C++ `Wheeler::hideEditModeVanillaMenus` / `showEditModeVanillaMenus`:
/// hides the inventory/magic menu movies behind the wheel in edit mode.
fn set_edit_menus_visible(ui: &UI, visible: bool) {
    if !visible && !control::control().wheel.hide_game_ui_in_edit_mode {
        return;
    }
    for name in [MENU_INVENTORY, MENU_MAGIC] {
        if !ui.is_menu_open(name) {
            continue;
        }
        let menu = ui.get_menu(name);
        if let Some(menu) = unsafe { menu.as_ptr().as_ref() }
            && let Some(movie) = unsafe { menu.ui_movie.as_ptr().as_ref() }
        {
            movie.set_visible(visible);
        }
    }
}

/// C++ `Wheeler::shouldBeInEditMode`
fn should_be_in_edit_mode(ui: &UI) -> bool {
    ui.is_menu_open(MENU_INVENTORY) || ui.is_menu_open(MENU_MAGIC)
}

// ---------------------------------------------------------------------------
// State machine

impl Wheeler {
    fn active_wheel(&mut self) -> Option<&mut Wheel> {
        usize::try_from(self.active_wheel_idx)
            .ok()
            .and_then(|i| self.wheels.get_mut(i))
    }

    fn reset_active_wheel(&mut self) {
        if let Some(wheel) = self.active_wheel() {
            wheel.set_hovered_entry_index(-1);
            wheel.reset_animation();
        }
    }

    /// C++ `Wheeler::getCursorRadiusMax`
    fn cursor_radius_max(&self) -> f32 {
        match usize::try_from(self.active_wheel_idx)
            .ok()
            .and_then(|i| self.wheels.get(i))
        {
            Some(wheel) => {
                control::control().wheel.cursor_radius_per_entry * wheel.num_entries() as f32
            }
            None => 0.0,
        }
    }

    /// C++ `Wheeler::OpenWheeler`
    fn open(&mut self) {
        if !player_3d_loaded() {
            return;
        }
        let Some(ui) = ui() else {
            return;
        };
        let in_edit_menu = ui.is_menu_open(MENU_INVENTORY) || ui.is_menu_open(MENU_MAGIC);
        if !in_edit_menu && CONFLICTING_MENUS.iter().any(|m| ui.is_menu_open(m)) {
            return;
        }

        if self.state != WheelState::Opened && self.state != WheelState::Opening {
            let s = style::style();
            if s.wheel.slow_time_scale < 1.0 && get_time_multiplier() == 1.0 {
                set_time_multiplier(s.wheel.slow_time_scale);
            }
            if s.wheel.blur_on_open {
                blur(true);
            }
            self.reset_active_wheel();
            self.state = if s.animation.fade_time > 0.0 {
                WheelState::Opening
            } else {
                WheelState::Opened
            };
            self.open_timer = 0.0;
            play_sound(Sound::WheelerToggle);
        }
    }

    /// C++ `Wheeler::CloseWheeler`
    fn close(&mut self) {
        if !player_3d_loaded() {
            return;
        }
        if self.state != WheelState::Closed {
            let s = style::style();
            if s.wheel.slow_time_scale <= 1.0 && get_time_multiplier() != 1.0 {
                set_time_multiplier(1.0);
            }
            if s.wheel.blur_on_open {
                blur(false);
            }
            self.reset_active_wheel();
            self.open_timer = 0.0;
            self.close_timer = 0.0;
        }
        self.state = WheelState::Closed;
    }

    /// C++ `Wheeler::TryCloseWheeler`
    fn try_close(&mut self) {
        if matches!(self.state, WheelState::Closed | WheelState::Closing) {
            return;
        }
        let s = style::style();
        if s.animation.fade_time == 0.0 {
            self.close();
        } else {
            // Reset the time scale before closing to avoid weirdness.
            if s.wheel.slow_time_scale <= 1.0 && get_time_multiplier() != 1.0 {
                set_time_multiplier(1.0);
            }
            self.state = WheelState::Closing;
            self.close_timer = 0.0;
        }
    }

    /// C++ `Wheeler::ToggleWheeler`
    fn toggle(&mut self) {
        if self.state == WheelState::Closed {
            self.open(); // TryOpenWheeler
        } else {
            self.try_close();
        }
    }

    fn close_if_opened_long_enough(&mut self) {
        if self.open_timer > control::control().wheel.toggle_hold_threshold {
            self.try_close();
        }
    }

    fn switch_wheel(&mut self, forward: bool) {
        if self.state != WheelState::Opened || self.wheels.is_empty() {
            return;
        }
        self.cursor_pos = [0.0, 0.0];
        if self.active_wheel_idx >= 0 {
            self.reset_active_wheel();
        }
        let len = self.wheels.len() as i32;
        self.active_wheel_idx = if forward {
            if self.active_wheel_idx + 1 >= len {
                0
            } else {
                self.active_wheel_idx + 1
            }
        } else if self.active_wheel_idx - 1 < 0 {
            len - 1
        } else {
            self.active_wheel_idx - 1
        };
        self.reset_active_wheel();
        if self.wheels.len() > 1 {
            play_sound(Sound::WheelSwitch);
        }
    }

    fn can_edit(&self) -> bool {
        self.edit_mode && self.state != WheelState::Closed
    }

    /// C++ `Wheeler::DeleteCurrentWheel`
    fn delete_current_wheel(&mut self) {
        if !self.can_edit() || self.wheels.len() <= 1 {
            return;
        }
        let Ok(idx) = usize::try_from(self.active_wheel_idx) else {
            return;
        };
        if self.wheels.get(idx).is_none_or(|w| !w.is_empty()) {
            return; // never delete a non-empty wheel
        }
        self.wheels.remove(idx);
        if idx == self.wheels.len() && idx != 0 {
            self.active_wheel_idx = self.wheels.len() as i32 - 1;
        }
        if let Some(wheel) = self.active_wheel() {
            wheel.set_hovered_entry_index(-1);
        }
    }

    /// C++ `Wheeler::MoveWheelForward` / `MoveWheelBack`: the last wheel moving
    /// forward becomes the first (and vice versa); others swap.
    fn move_wheel(&mut self, forward: bool) {
        if !self.can_edit() || self.wheels.len() <= 1 {
            return;
        }
        let Ok(idx) = usize::try_from(self.active_wheel_idx) else {
            return;
        };
        let last = self.wheels.len() - 1;
        let target = match (forward, idx) {
            (true, i) if i == last => {
                self.wheels.rotate_right(1);
                0
            }
            (true, i) => {
                self.wheels.swap(i, i + 1);
                i + 1
            }
            (false, 0) => {
                self.wheels.rotate_left(1);
                last
            }
            (false, i) => {
                self.wheels.swap(i, i - 1);
                i - 1
            }
        };
        self.active_wheel_idx = target as i32;
    }
}

// ---------------------------------------------------------------------------
// Public API (C++ static members of `Wheeler`)

/// C++ `Wheeler::Init`
pub fn init() {
    // TODO: C++ also initializes the wheel item factories here.
    LazyLock::force(&WHEELER);
}

/// C++ `Wheeler::SetupDefaultWheels`: `DefaultWheelCount` wheels of
/// `DefaultEntryCount` empty entries (C++: fixed 2 x 4).
pub fn setup_default_wheels() {
    let (wheels, entries) = {
        let c = control::control();
        (
            c.wheel.default_wheel_count.max(1),
            c.wheel.default_entry_count.max(1),
        )
    };
    let mut w = lock();
    w.wheels.clear();
    for _ in 0..wheels {
        let mut wheel = Wheel::new();
        for _ in 0..entries {
            wheel.push_empty_entry();
        }
        w.wheels.push(wheel);
    }
    w.active_wheel_idx = 0;
}

pub fn is_wheeler_open() -> bool {
    lock().state != WheelState::Closed
}

pub fn is_in_edit_mode() -> bool {
    lock().edit_mode
}

pub fn toggle_wheeler() {
    lock().toggle();
}

pub fn toggle_wheel_if_in_inventory() {
    if ui().is_some_and(|ui| should_be_in_edit_mode(ui)) {
        toggle_wheeler();
    }
}

pub fn toggle_wheel_if_not_in_inventory() {
    if ui().is_some_and(|ui| !should_be_in_edit_mode(ui)) {
        toggle_wheeler();
    }
}

pub fn close_wheeler_if_opened_long_enough() {
    lock().close_if_opened_long_enough();
}

pub fn close_wheeler_if_opened_long_enough_if_in_inventory() {
    if ui().is_some_and(|ui| should_be_in_edit_mode(ui)) {
        close_wheeler_if_opened_long_enough();
    }
}

pub fn close_wheeler_if_opened_long_enough_if_not_in_inventory() {
    if ui().is_some_and(|ui| !should_be_in_edit_mode(ui)) {
        close_wheeler_if_opened_long_enough();
    }
}

pub fn next_wheel() {
    lock().switch_wheel(true);
}

pub fn prev_wheel() {
    lock().switch_wheel(false);
}

/// C++ `Wheeler::UpdateCursorPosMouse`
pub fn update_cursor_pos_mouse(delta_x: f32, delta_y: f32) {
    let mut w = lock();
    if w.state == WheelState::Closed {
        return;
    }
    let mut pos = [w.cursor_pos[0] + delta_x, w.cursor_pos[1] + delta_y];
    let distance = (pos[0] * pos[0] + pos[1] * pos[1]).sqrt();
    let radius = w.cursor_radius_max();
    if distance > radius {
        // Clamp to the edge of the cursor radius.
        pos = if distance > 0.0 {
            [pos[0] / distance * radius, pos[1] / distance * radius]
        } else {
            [0.0, 0.0]
        };
    }
    w.cursor_pos = pos;
}

/// C++ `Wheeler::UpdateCursorPosGamepad`
pub fn update_cursor_pos_gamepad(x: f32, y: f32) {
    const CONTROLLER_DEADZONE: f32 = 0.1;
    let mut w = lock();
    if w.state == WheelState::Closed {
        return;
    }
    if x.abs() <= CONTROLLER_DEADZONE && y.abs() <= CONTROLLER_DEADZONE {
        return;
    }
    let radius = w.cursor_radius_max();
    w.cursor_pos = [x * radius, -y * radius];
}

/// C++ `Wheeler::PrevItemInEntry`
pub fn prev_item_in_entry() {
    let mut w = lock();
    if w.state == WheelState::Opened
        && let Some(wheel) = w.active_wheel()
    {
        wheel.prev_item_in_hovered_entry(&GameHost);
    }
}

/// C++ `Wheeler::NextItemInEntry`
pub fn next_item_in_entry() {
    let mut w = lock();
    if w.state == WheelState::Opened
        && let Some(wheel) = w.active_wheel()
    {
        wheel.next_item_in_hovered_entry(&GameHost);
    }
}

/// C++ `Wheeler::ActivateHoveredEntryPrimary`
pub fn activate_hovered_entry_primary() {
    let mut w = lock();
    if w.state != WheelState::Opened {
        return;
    }
    let edit_mode = w.edit_mode;
    if let Some(wheel) = w.active_wheel() {
        wheel.activate_hovered_entry_primary(edit_mode, &ItemCtx::current(), &GameHost);
    }
    handle_close_request(&mut w);
}

/// C++ `Wheeler::ActivateHoveredEntrySecondary`: on an empty wheel this
/// deletes the wheel instead (edit mode, more than one wheel).
pub fn activate_hovered_entry_secondary() {
    let mut w = lock();
    if w.state != WheelState::Opened {
        return;
    }
    let edit_mode = w.edit_mode;
    let Some(wheel) = w.active_wheel() else {
        return;
    };
    if wheel.is_empty() {
        w.delete_current_wheel();
    } else {
        wheel.activate_hovered_entry_secondary(edit_mode, &ItemCtx::current());
    }
    handle_close_request(&mut w);
}

/// C++ `Wheeler::AddEmptyEntryToCurrentWheel`
pub fn add_empty_entry_to_current_wheel() {
    let mut w = lock();
    if w.can_edit()
        && let Some(wheel) = w.active_wheel()
    {
        wheel.push_empty_entry();
    }
}

/// C++ `Wheeler::AddWheel`
pub fn add_wheel() {
    let mut w = lock();
    if w.can_edit() {
        w.wheels.push(Wheel::new());
    }
}

/// C++ `Wheeler::MoveEntryForwardInCurrentWheel`
pub fn move_entry_forward_in_current_wheel() {
    let mut w = lock();
    if w.can_edit()
        && let Some(wheel) = w.active_wheel()
    {
        wheel.move_hovered_entry_forward();
    }
}

/// C++ `Wheeler::MoveEntryBackInCurrentWheel`
pub fn move_entry_back_in_current_wheel() {
    let mut w = lock();
    if w.can_edit()
        && let Some(wheel) = w.active_wheel()
    {
        wheel.move_hovered_entry_back();
    }
}

pub fn move_wheel_forward() {
    lock().move_wheel(true);
}

pub fn move_wheel_back() {
    lock().move_wheel(false);
}

// ---------------------------------------------------------------------------
// Drawing

fn is_popup_open() -> bool {
    unsafe { imgui::sys::igIsPopupOpen(WHEEL_WINDOW_ID.as_ptr(), 0) }
}

fn set_next_window_offscreen() {
    let pos = imgui::sys::ImVec2 {
        x: -100.0,
        y: -100.0,
    };
    let pivot = imgui::sys::ImVec2 { x: 0.0, y: 0.0 };
    unsafe { imgui::sys::igSetNextWindowPos(pos, 0, pivot) };
}

/// C++ `Wheeler::getWheelCenter`
fn wheel_center(ui: &Ui) -> [f32; 2] {
    let s = style::style();
    let [w, h] = ui.io().display_size;
    [
        w / 2.0 + s.wheel.center_offset_x,
        h / 2.0 + s.wheel.center_offset_y,
    ]
}

/// C++ wheel indicator block of `Wheeler::Update`: one dot per wheel.
fn draw_wheel_indicator(wheel_center: [f32; 2], count: usize, active: i32, args: DrawArgs) {
    let w = style::style().wheel;
    for i in 0..count {
        let is_active = i as i32 == active;
        let mut pos = [
            wheel_center[0] + w.wheel_indicator_offset_x + i as f32 * w.wheel_indicator_spacing,
            wheel_center[1] + w.wheel_indicator_offset_y,
        ];
        if w.wheel_indicator_alignment == WidgetAlignment::Center {
            pos[0] -= (count - 1) as f32 * w.wheel_indicator_spacing / 2.0;
        }
        if w.use_geometric_primitive_for_background_texture {
            let color = if is_active {
                w.wheel_indicator_active_color
            } else {
                w.wheel_indicator_inactive_color
            };
            drawer::draw_circle_filled(pos, w.wheel_indicator_size / 2.0, color, 10, args);
        } else {
            let icon = if is_active {
                IconImageType::wheel_indicator_active
            } else {
                IconImageType::wheel_indicator_inactive
            };
            let size = [w.wheel_indicator_size, w.wheel_indicator_size];
            drawer::draw_texture(
                GameHost.icon(icon).texture_ptr(),
                pos,
                0.0,
                0.0,
                size,
                C_SKYRIMWHITE,
                args,
            );
        }
    }
}

/// C++ `Wheeler::Update`, called every frame from the present hook.
pub fn update(ui: &Ui, delta_time: f32) {
    if !player_3d_loaded() {
        return;
    }
    let Some(game_ui) = self::ui() else {
        return;
    };
    let mut w = lock();

    if w.state == WheelState::Closed {
        if is_popup_open() {
            // Close the popup (positioned off screen).
            set_next_window_offscreen();
            unsafe {
                imgui::sys::igBeginPopup(WHEEL_WINDOW_ID.as_ptr(), 0);
                imgui::sys::igCloseCurrentPopup();
                imgui::sys::igEndPopup();
            }
            if let Some(wheel) = w.active_wheel() {
                wheel.set_hovered_entry_index(-1);
            }
            if w.edit_mode {
                set_edit_menus_visible(game_ui, true);
            }
        }
        return;
    }

    // Opened, opening or closing: draw with the matching alpha.
    if !is_popup_open() {
        unsafe { imgui::sys::igOpenPopup_Str(WHEEL_WINDOW_ID.as_ptr(), 0) };
        if let Some(wheel) = w.active_wheel() {
            wheel.set_hovered_entry_index(-1);
        }
        w.cursor_pos = [0.0, 0.0];
    }

    set_next_window_offscreen();

    w.edit_mode = should_be_in_edit_mode(game_ui);
    if w.edit_mode {
        set_edit_menus_visible(game_ui, false);
    }

    let popped_up = unsafe {
        if w.edit_mode {
            imgui::sys::igBeginPopupModal(WHEEL_WINDOW_ID.as_ptr(), core::ptr::null_mut(), 0)
        } else {
            imgui::sys::igBeginPopup(WHEEL_WINDOW_ID.as_ptr(), 0)
        }
    };
    if !popped_up {
        return;
    }

    let draw_list = unsafe { imgui::sys::igGetWindowDrawList() };
    unsafe { imgui::sys::ImDrawList_PushClipRectFullScreen(draw_list) };

    let fade_time = style::style().animation.fade_time;
    w.open_timer += delta_time;
    let mut fade = 1.0f32;
    match w.state {
        WheelState::Opening => {
            fade = (w.open_timer / fade_time).min(1.0);
            if w.open_timer >= fade_time {
                w.state = WheelState::Opened;
            }
        }
        WheelState::Closing => {
            w.close_timer += delta_time;
            fade = (1.0 - w.close_timer / fade_time).max(0.0);
            if w.close_timer >= fade_time {
                w.close();
                w.close_timer = 0.0;
            }
        }
        _ => {}
    }

    let args = DrawArgs {
        alpha_mult: fade as f64,
        ..DrawArgs::default()
    };
    let anim = style::style().animation;
    let mut center = wheel_center(ui);
    center[1] += (1.0 - fade) * anim.toggle_vertical_fade_distance;
    center[0] += (1.0 - fade) * anim.toggle_horizontal_fade_distance;

    if w.wheels.is_empty() {
        let text = texts::get_text(TextType::NoWheelPresent);
        drawer::draw_text(center[0], center[1], &text, C_SKYRIMWHITE, 40.0, args, true);
    } else {
        // C++ `atan2f(_cursorPos.y, _cursorPos.x)`
        let cursor = w.cursor_pos;
        let cursor_angle = cursor[1].atan2(cursor[0]);
        let centered = cursor == [0.0, 0.0];
        if let Some(wheel) = w.active_wheel() {
            wheel.draw(
                center,
                cursor_angle,
                centered,
                delta_time,
                &ItemCtx::current(),
                &GameHost,
                args,
            );
        }
    }
    draw_wheel_indicator(center, w.wheels.len(), w.active_wheel_idx, args);

    if w.edit_mode {
        // The C++ version gave no cue that the wheel is editable.
        let control = control::control();
        let lines = if control.wheel.show_edit_mode_hints {
            wheeler_imgui::edit_banner::hint_lines(&control)
        } else {
            Vec::new()
        };
        drop(control);
        let scale = ui.io().display_size[1] / 1080.0;
        let outer = style::style().wheel.outer_circle_radius;
        wheeler_imgui::edit_banner::draw(center, outer, &lines, scale, args);
    }

    unsafe {
        imgui::sys::ImDrawList_PopClipRect(draw_list);
        imgui::sys::igEndPopup();
    }
}

// ---------------------------------------------------------------------------
// Serialization (C++ `Wheeler::SerializeIntoJsonObj` / `SerializeFromJsonObj`)

/// C++ `Wheeler::Clear`: closes the wheel and drops every wheel.
pub fn clear() {
    let mut w = lock();
    if w.state != WheelState::Closed {
        w.close(); // force close, items are being reloaded
    }
    w.edit_mode = false; // C++ `exitEditMode`
    w.wheels.clear();
}

/// C++ `Wheeler::SerializeIntoJsonObj`
pub fn save_data() -> wheeler_core::save::WheelerData {
    let w = lock();
    wheeler_core::save::WheelerData {
        wheels: w.wheels.iter().map(Wheel::save_data).collect(),
        active_wheel: w.active_wheel_idx,
    }
}

/// C++ `Wheeler::Clear` + `Wheeler::SerializeFromJsonObj`
pub fn load_data(
    data: &wheeler_core::save::WheelerData,
    load: &mut wheeler_imgui::wheel::ItemLoader<'_, ItemCtx>,
) {
    clear();
    let wheels: Vec<Wheel> = data
        .wheels
        .iter()
        .map(|wheel| Wheel::from_save_data(wheel, load))
        .collect();
    let mut w = lock();
    w.active_wheel_idx = if wheels.is_empty() {
        0
    } else {
        data.active_wheel.clamp(0, wheels.len() as i32 - 1)
    };
    w.wheels = wheels;
}
