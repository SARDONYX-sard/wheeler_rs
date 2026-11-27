//! Port of `Wheeler/Wheel.cpp`, `Wheeler/WheelEntry.cpp` and the drawing
//! helpers of `WheelItems/WheelItem.cpp`.
//!
//! Items and game services are abstracted behind [`WheelItem`] and
//! [`WheelHost`] so the wheel can be drawn and tested without the game.
//! `Ctx` is whatever per-frame data items need (the player's inventory in the
//! plugin, `()` in tests).
//!
//! The C++ classes guard themselves with `shared_mutex`; here the whole
//! wheeler state is owned behind a single lock by the caller.

use wheeler_core::animation::{BounceInterpolator, FloatInterpolator};
use wheeler_core::geometry::{self, EntryLayout};
use wheeler_core::save::{self, EntryData, ItemData, WheelData};
use wheeler_core::sound::Sound;
use wheeler_core::style::{self, C_SKYRIMWHITE, Style};

use crate::drawer::{self, DrawArgs};
use crate::image::{IconImageType, Image};

/// C++ `WheelItem` (virtual interface).
pub trait WheelItem<Ctx>: Send {
    /// Everything shown in the wheel slot (icon and name).
    fn draw_slot(&self, center: [f32; 2], hovered: bool, ctx: &Ctx, args: DrawArgs);
    /// Everything shown in the wheel centre while hovered (enlarged icon,
    /// name, description, stats).
    fn draw_highlight(&self, center: [f32; 2], ctx: &Ctx, args: DrawArgs);
    /// Equipped / currently in effect.
    fn is_active(&self, ctx: &Ctx) -> bool;
    /// The player still owns / can use it. Unavailable items are pruned.
    fn is_available(&self, ctx: &Ctx) -> bool;
    fn activate_primary(&mut self, ctx: &Ctx);
    fn activate_secondary(&mut self, ctx: &Ctx);
    fn activate_special(&mut self, _ctx: &Ctx) {}
    /// C++ `WheelEntry::drawSlot` never prunes shouts.
    fn keep_when_unavailable(&self) -> bool {
        false
    }
    /// C++ `SerializeIntoJsonObj`; `None` is not saved.
    fn save_data(&self) -> Option<ItemData> {
        None
    }
}

/// Builds an item from its save data (C++ `MakeWheelItemFromJsonObject`).
pub type ItemLoader<'a, Ctx> = dyn FnMut(&ItemData) -> Option<Box<dyn WheelItem<Ctx>>> + 'a;

/// Services the wheel needs from its environment.
pub trait WheelHost<Ctx> {
    fn play_sound(&self, sound: Sound);
    fn icon(&self, icon: IconImageType) -> Image;
    /// C++ `WheelItemFactory::MakeWheelItemFromMenuHovered`
    fn item_from_menu_hovered(&self) -> Option<Box<dyn WheelItem<Ctx>>>;
}

// ---------------------------------------------------------------------------
// WheelItem drawing helpers (protected members of C++ `WheelItem`)

/// C++ `WheelItem::drawSlotTexture`
pub fn draw_slot_texture(texture: Image, center: [f32; 2], args: DrawArgs) {
    let p = style::style().item_slot_texture;
    drawer::draw_texture(
        texture.texture_ptr(),
        center,
        p.offset_x,
        p.offset_y,
        texture.scaled(p.size),
        C_SKYRIMWHITE,
        args,
    );
}

/// C++ `WheelItem::drawSlotText`
pub fn draw_slot_text(center: [f32; 2], text: &str, args: DrawArgs) {
    let s = style::style();
    let p = s.item_slot_text;
    drawer::draw_text(
        center[0] + p.offset_x,
        center[1] + p.offset_y,
        text,
        s.wheel.text_color,
        p.size,
        args,
        true,
    );
}

/// C++ `WheelItem::drawHighlightTexture`
pub fn draw_highlight_texture(texture: Image, center: [f32; 2], args: DrawArgs) {
    let p = style::style().item_highlight_texture;
    drawer::draw_texture(
        texture.texture_ptr(),
        center,
        p.offset_x,
        p.offset_y,
        texture.scaled(p.size),
        C_SKYRIMWHITE,
        args,
    );
}

/// C++ `WheelItem::drawHighlightText`
pub fn draw_highlight_text(center: [f32; 2], text: &str, args: DrawArgs) {
    let s = style::style();
    let p = s.item_highlight_text;
    drawer::draw_text(
        center[0] + p.offset_x,
        center[1] + p.offset_y,
        text,
        s.wheel.text_color,
        p.size,
        args,
        true,
    );
}

/// C++ `WheelItem::drawHighlightDescription`
pub fn draw_highlight_description(center: [f32; 2], text: &str, args: DrawArgs) {
    let s = style::style();
    let d = s.item_highlight_desc;
    drawer::draw_text_block(
        center[0] + d.offset_x,
        center[1] + d.offset_y,
        text,
        s.wheel.text_color,
        d.size,
        d.line_spacing,
        d.line_length,
        args,
    );
}

/// C++ `WheelItem::drawItemHighlightStatIconAndValue`
pub fn draw_highlight_stat_icon_and_value(
    center: [f32; 2],
    stat_icon: Image,
    stat_value: f32,
    args: DrawArgs,
) {
    let s = style::style();
    let t = s.item_highlight_stat_text;
    let text = format!(": {}", stat_value.ceil() as i32);
    drawer::draw_text(
        center[0] + t.offset_x,
        center[1] + t.offset_y,
        &text,
        s.wheel.text_color,
        t.size,
        args,
        false,
    );
    let i = s.item_highlight_stat_icon;
    drawer::draw_texture(
        stat_icon.texture_ptr(),
        center,
        i.offset_x,
        i.offset_y,
        stat_icon.scaled(i.size),
        C_SKYRIMWHITE,
        args,
    );
}

// ---------------------------------------------------------------------------
// WheelEntry

/// C++ `WheelEntry`: a slot holding one or more items.
pub struct WheelEntry<Ctx> {
    items: Vec<Box<dyn WheelItem<Ctx>>>,
    selected_item: i32,
    prev_hovered: bool,
    arc_radius_inc: FloatInterpolator,
    arc_inner_angle_inc: FloatInterpolator,
    arc_outer_angle_inc: FloatInterpolator,
    arc_radius_bounce: BounceInterpolator,
}

impl<Ctx> Default for WheelEntry<Ctx> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Ctx> WheelEntry<Ctx> {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            selected_item: 0,
            prev_hovered: false,
            arc_radius_inc: FloatInterpolator::new(0.0),
            arc_inner_angle_inc: FloatInterpolator::new(0.0),
            arc_outer_angle_inc: FloatInterpolator::new(0.0),
            arc_radius_bounce: BounceInterpolator::new(0.0),
        }
    }

    fn selected(&self) -> Option<&dyn WheelItem<Ctx>> {
        usize::try_from(self.selected_item)
            .ok()
            .and_then(|i| self.items.get(i))
            .map(|b| &**b)
    }

    fn selected_mut(&mut self) -> Option<&mut Box<dyn WheelItem<Ctx>>> {
        usize::try_from(self.selected_item)
            .ok()
            .and_then(|i| self.items.get_mut(i))
    }

    /// Advances all animations (C++ `TimeFloatInterpolatorManager::Update`).
    pub fn tick(&mut self, dt: f32) {
        let dt = dt as f64;
        self.arc_radius_inc.update(dt);
        self.arc_inner_angle_inc.update(dt);
        self.arc_outer_angle_inc.update(dt);
        self.arc_radius_bounce.update(dt);
    }

    /// C++ `WheelEntry::UpdateAnimation`
    pub fn update_animation(&mut self, inner_spacing_rad: f32, hovered: bool, s: &Style) {
        let w = s.wheel;
        let a = s.animation;
        if hovered {
            if !self.prev_hovered {
                let expand = a.entry_highlight_expand_scale
                    * (w.outer_circle_radius - w.inner_circle_radius);
                let t = a.entry_highlight_expand_time as f64;
                self.arc_radius_inc.interpolate_to(expand as f64, t);
                self.arc_outer_angle_inc.interpolate_to(
                    (inner_spacing_rad * (w.inner_circle_radius / w.outer_circle_radius)) as f64,
                    t,
                );
                self.arc_inner_angle_inc
                    .interpolate_to(inner_spacing_rad as f64, t);
                self.prev_hovered = true;
            }
        } else if self.prev_hovered {
            self.prev_hovered = false;
            let t = a.entry_highlight_retract_time as f64;
            self.arc_radius_inc.interpolate_to(0.0, t);
            self.arc_outer_angle_inc.interpolate_to(0.0, t);
            self.arc_inner_angle_inc.interpolate_to(0.0, t);
        }
    }

    /// C++ `WheelEntry::GetRadiusMod`
    pub fn radius_mod(&self) -> f32 {
        (self.arc_radius_inc.value() + self.arc_radius_bounce.value()) as f32
    }

    /// C++ `WheelEntry::DrawBackGround`
    #[allow(clippy::too_many_arguments)]
    pub fn draw_background(
        &self,
        wheel_center: [f32; 2],
        entry_center: [f32; 2],
        layout: &EntryLayout,
        hovered: bool,
        ctx: &Ctx,
        host: &dyn WheelHost<Ctx>,
        args: DrawArgs,
        s: &Style,
    ) {
        let active = self.is_active(ctx);
        let w = s.wheel;
        if w.use_geometric_primitive_for_background_texture {
            let outer_bound = w.outer_circle_radius
                + self.arc_radius_inc.value() as f32
                + self.arc_radius_bounce.value() as f32;
            let inner_inc = self.arc_inner_angle_inc.value() as f32 * 2.0;
            let outer_inc = self.arc_outer_angle_inc.value() as f32 * 2.0;
            let (inner_min, inner_max) = (
                layout.inner_angle_min - inner_inc,
                layout.inner_angle_max + inner_inc,
            );
            let (outer_min, outer_max) = (
                layout.outer_angle_min - outer_inc,
                layout.outer_angle_max + outer_inc,
            );

            drawer::draw_arc_gradient(
                wheel_center,
                w.inner_circle_radius,
                outer_bound,
                inner_min,
                inner_max,
                outer_min,
                outer_max,
                if hovered {
                    w.hovered_color_begin
                } else {
                    w.unhovered_color_begin
                },
                if hovered {
                    w.hovered_color_end
                } else {
                    w.unhovered_color_end
                },
                layout.num_arc_segments,
                args,
            );
            let (begin, end) = if active {
                (w.active_arc_color_begin, w.active_arc_color_end)
            } else {
                (w.inactive_arc_color_begin, w.inactive_arc_color_end)
            };
            drawer::draw_arc_gradient(
                wheel_center,
                outer_bound,
                outer_bound + w.active_arc_width,
                outer_min,
                outer_max,
                outer_min,
                outer_max,
                begin,
                end,
                layout.num_arc_segments,
                args,
            );
        } else {
            let icon = if active {
                IconImageType::slot_active_background
            } else if hovered {
                IconImageType::slot_highlighted_background
            } else {
                IconImageType::slot_background
            };
            let texture = host.icon(icon);
            drawer::draw_texture(
                texture.texture_ptr(),
                entry_center,
                0.0,
                0.0,
                texture.scaled(s.item_slot_background_texture_scale),
                C_SKYRIMWHITE,
                args,
            );
        }
    }

    /// C++ `WheelEntry::DrawSlotAndHighlight`
    pub fn draw_slot_and_highlight(
        &mut self,
        wheel_center: [f32; 2],
        entry_center: [f32; 2],
        hovered: bool,
        ctx: &Ctx,
        args: DrawArgs,
        s: &Style,
    ) {
        if hovered {
            self.draw_highlight(wheel_center, ctx, args, s);
        }
        self.draw_slot(entry_center, hovered, ctx, args);
    }

    /// C++ `WheelEntry::drawSlot`: prunes unavailable items (except shouts),
    /// then draws the selected one.
    fn draw_slot(&mut self, center: [f32; 2], hovered: bool, ctx: &Ctx, args: DrawArgs) {
        let mut i = 0;
        while i < self.items.len() {
            let item = &self.items[i];
            if !item.keep_when_unavailable() && !item.is_available(ctx) {
                self.items.remove(i);
                if (i as i32) < self.selected_item && self.selected_item > 0 {
                    self.selected_item -= 1;
                }
            } else {
                i += 1;
            }
        }
        if self.items.is_empty() {
            return;
        }
        if self.selected_item as usize >= self.items.len() {
            self.selected_item = self.items.len() as i32 - 1;
        }
        if let Some(item) = self.selected() {
            item.draw_slot(center, hovered, ctx, args);
        }
    }

    /// C++ `WheelEntry::drawHighlight`
    fn draw_highlight(&self, center: [f32; 2], ctx: &Ctx, args: DrawArgs, s: &Style) {
        let Some(item) = self.selected() else {
            return;
        };
        item.draw_highlight(center, ctx, args);
        if self.items.len() > 1 {
            let p = s.entry_highlight_text;
            let text = format!("{} / {}", self.selected_item + 1, self.items.len());
            drawer::draw_text(
                center[0] + p.offset_x,
                center[1] + p.offset_y,
                &text,
                C_SKYRIMWHITE,
                p.size,
                args,
                true,
            );
        }
    }

    /// C++ `WheelEntry::IsActive`
    pub fn is_active(&self, ctx: &Ctx) -> bool {
        self.selected().is_some_and(|i| i.is_active(ctx))
    }

    /// C++ `WheelEntry::IsAvailable`
    pub fn is_available(&self, ctx: &Ctx) -> bool {
        self.selected().is_some_and(|i| i.is_available(ctx))
    }

    fn bump(&mut self, s: &Style) {
        let w = s.wheel;
        let a = s.animation;
        self.arc_radius_bounce.interpolate_to(
            (a.entry_input_bump_scale * (w.outer_circle_radius - w.inner_circle_radius)) as f64,
            a.entry_input_bump_time as f64,
        );
    }

    /// C++ `WheelEntry::ActivateItemSecondary`: uses the item, or removes it in edit mode.
    pub fn activate_item_secondary(&mut self, edit_mode: bool, ctx: &Ctx, s: &Style) {
        if self.items.is_empty() {
            return;
        }
        if !edit_mode {
            if let Some(item) = self.selected_mut() {
                item.activate_secondary(ctx);
            }
            self.bump(s);
        } else {
            let index = self.selected_item.clamp(0, self.items.len() as i32 - 1) as usize;
            self.items.remove(index);
            if self.selected_item > 0 {
                self.selected_item -= 1;
            }
        }
    }

    /// C++ `WheelEntry::ActivateItemPrimary`: uses the item, or in edit mode
    /// inserts the item hovered in the inventory/magic menu.
    pub fn activate_item_primary(
        &mut self,
        edit_mode: bool,
        ctx: &Ctx,
        host: &dyn WheelHost<Ctx>,
        s: &Style,
    ) {
        if !edit_mode {
            if self.items.is_empty() {
                return;
            }
            if let Some(item) = self.selected_mut() {
                item.activate_primary(ctx);
            }
            self.bump(s);
        } else if let Some(item) = host.item_from_menu_hovered() {
            let index = (self.selected_item.max(0) as usize).min(self.items.len());
            self.items.insert(index, item);
        }
    }

    /// C++ `WheelEntry::ActivateItemSpecial`
    pub fn activate_item_special(&mut self, edit_mode: bool, ctx: &Ctx) {
        if edit_mode {
            return;
        }
        if let Some(item) = self.selected_mut() {
            item.activate_special(ctx);
        }
    }

    /// C++ `WheelEntry::PrevItem`
    pub fn prev_item(&mut self, host: &dyn WheelHost<Ctx>) {
        self.selected_item -= 1;
        if self.selected_item < 0 {
            self.selected_item = if self.items.len() > 1 {
                self.items.len() as i32 - 1
            } else {
                0
            };
        }
        if self.items.len() > 1 {
            host.play_sound(Sound::ItemSwitch);
        }
    }

    /// C++ `WheelEntry::NextItem`
    pub fn next_item(&mut self, host: &dyn WheelHost<Ctx>) {
        self.selected_item += 1;
        if self.selected_item as usize >= self.items.len() {
            self.selected_item = 0;
        }
        if self.items.len() > 1 {
            host.play_sound(Sound::ItemSwitch);
        }
    }

    pub fn push_item(&mut self, item: Box<dyn WheelItem<Ctx>>) {
        self.items.push(item);
    }

    pub fn selected_item_index(&self) -> i32 {
        self.selected_item
    }

    pub fn set_selected_item(&mut self, selected: i32) {
        self.selected_item = selected;
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn num_items(&self) -> usize {
        self.items.len()
    }

    pub fn items(&self) -> &[Box<dyn WheelItem<Ctx>>] {
        &self.items
    }

    /// C++ `WheelEntry::SerializeIntoJsonObj`
    pub fn save_data(&self) -> EntryData {
        EntryData {
            items: self.items.iter().filter_map(|i| i.save_data()).collect(),
            selected_item: self.selected_item,
        }
    }

    /// C++ `WheelEntry::SerializeFromJsonObj`: items that fail to load are
    /// skipped.
    pub fn from_save_data(data: &EntryData, load: &mut ItemLoader<'_, Ctx>) -> Self {
        let mut entry = Self::new();
        for item in data.items.iter().filter_map(&mut *load) {
            entry.push_item(item);
        }
        entry.selected_item = save::clamp_selected(data.selected_item, entry.items.len());
        entry
    }

    /// C++ `WheelEntry::ResetAnimation`
    pub fn reset_animation(&mut self) {
        for i in [
            &mut self.arc_inner_angle_inc,
            &mut self.arc_outer_angle_inc,
            &mut self.arc_radius_inc,
        ] {
            i.force_finish();
            i.force_value(0.0);
        }
        self.arc_radius_bounce.force_finish();
        self.prev_hovered = false;
    }
}

// ---------------------------------------------------------------------------
// Wheel

/// C++ `Wheel`: a ring of entries.
pub struct Wheel<Ctx> {
    entries: Vec<WheelEntry<Ctx>>,
    /// Highlighted entry; reset every time the wheel reopens.
    hovered_entry: i32,
}

impl<Ctx> Default for Wheel<Ctx> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Ctx> Wheel<Ctx> {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            hovered_entry: -1,
        }
    }

    fn hovered_mut(&mut self) -> Option<&mut WheelEntry<Ctx>> {
        usize::try_from(self.hovered_entry)
            .ok()
            .and_then(|i| self.entries.get_mut(i))
    }

    /// C++ `Wheel::Draw`, plus per-frame animation ticking (`dt`).
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        wheel_center: [f32; 2],
        cursor_angle: f32,
        cursor_centered: bool,
        dt: f32,
        ctx: &Ctx,
        host: &dyn WheelHost<Ctx>,
        args: DrawArgs,
    ) {
        let s = style::style();
        let w = s.wheel;

        if !w.use_geometric_primitive_for_background_texture {
            let bg = host.icon(IconImageType::wheel_background);
            drawer::draw_texture(
                bg.texture_ptr(),
                wheel_center,
                0.0,
                0.0,
                bg.scaled(w.wheel_background_texture_scale),
                C_SKYRIMWHITE,
                args,
            );
        }
        if self.entries.is_empty() {
            drawer::draw_text(
                wheel_center[0],
                wheel_center[1],
                "Empty Wheel",
                C_SKYRIMWHITE,
                40.0,
                args,
                true,
            );
            return;
        }

        for entry in &mut self.entries {
            entry.tick(dt);
        }

        let mut cursor_indicator_angle = cursor_angle;
        let count = self.entries.len();
        let mut foreground = Vec::with_capacity(count);
        for index in 0..count {
            let layout = geometry::entry_layout(
                index,
                count,
                w.inner_circle_radius,
                w.outer_circle_radius,
                w.inner_spacing,
            );

            let mut hovered = false;
            if !cursor_centered {
                if layout.contains_angle(cursor_angle) && index as i32 != self.hovered_entry {
                    self.hovered_entry = index as i32;
                    host.play_sound(Sound::EntrySwitch);
                }
                hovered = self.hovered_entry == index as i32;
            }
            if hovered && s.animation.snappy_cursor_indicator {
                cursor_indicator_angle = layout.center_angle;
            }

            let entry = &mut self.entries[index];
            entry.update_animation(layout.inner_spacing_rad, hovered, &s);
            let entry_center = layout.center(
                wheel_center,
                w.inner_circle_radius,
                w.outer_circle_radius,
                entry.radius_mod(),
            );
            entry.draw_background(
                wheel_center,
                entry_center,
                &layout,
                hovered,
                ctx,
                host,
                args,
                &s,
            );
            foreground.push((entry_center, hovered));
        }

        // Foreground in a second pass so backgrounds never overlap content.
        for (entry, (entry_center, hovered)) in self.entries.iter_mut().zip(foreground) {
            entry.draw_slot_and_highlight(wheel_center, entry_center, hovered, ctx, args, &s);
        }

        if !cursor_centered {
            draw_cursor_indicator(wheel_center, cursor_indicator_angle, args, &s);
        }
    }

    pub fn push_entry(&mut self, entry: WheelEntry<Ctx>) {
        self.entries.push(entry);
    }

    pub fn push_empty_entry(&mut self) {
        self.push_entry(WheelEntry::new());
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn entries(&self) -> &[WheelEntry<Ctx>] {
        &self.entries
    }

    /// C++ `Wheel::SerializeIntoJsonObj`
    pub fn save_data(&self) -> WheelData {
        WheelData {
            entries: self.entries.iter().map(WheelEntry::save_data).collect(),
        }
    }

    /// C++ `Wheel::SerializeFromJsonObj`
    pub fn from_save_data(data: &WheelData, load: &mut ItemLoader<'_, Ctx>) -> Self {
        let mut wheel = Self::new();
        for entry in &data.entries {
            wheel.push_entry(WheelEntry::from_save_data(entry, load));
        }
        wheel
    }

    pub fn prev_item_in_hovered_entry(&mut self, host: &dyn WheelHost<Ctx>) {
        if let Some(entry) = self.hovered_mut() {
            entry.prev_item(host);
        }
    }

    pub fn next_item_in_hovered_entry(&mut self, host: &dyn WheelHost<Ctx>) {
        if let Some(entry) = self.hovered_mut() {
            entry.next_item(host);
        }
    }

    pub fn reset_animation(&mut self) {
        for entry in &mut self.entries {
            entry.reset_animation();
        }
    }

    /// C++ `Wheel::ActivateHoveredEntryPrimary`
    pub fn activate_hovered_entry_primary(
        &mut self,
        edit_mode: bool,
        ctx: &Ctx,
        host: &dyn WheelHost<Ctx>,
    ) {
        let s = style::style();
        if let Some(entry) = self.hovered_mut() {
            entry.activate_item_primary(edit_mode, ctx, host, &s);
        }
    }

    /// C++ `Wheel::ActivateHoveredEntrySecondary`: removes an empty entry,
    /// otherwise forwards to the entry.
    pub fn activate_hovered_entry_secondary(&mut self, edit_mode: bool, ctx: &Ctx) {
        let s = style::style();
        let Ok(index) = usize::try_from(self.hovered_entry) else {
            return;
        };
        let Some(entry) = self.entries.get_mut(index) else {
            return;
        };
        if entry.is_empty() {
            self.entries.remove(index);
            return;
        }
        entry.activate_item_secondary(edit_mode, ctx, &s);
    }

    /// C++ `Wheel::ActivateHoveredEntrySpecial`
    pub fn activate_hovered_entry_special(&mut self, edit_mode: bool, ctx: &Ctx) {
        if edit_mode {
            return;
        }
        if let Some(entry) = self.hovered_mut() {
            entry.activate_item_special(edit_mode, ctx);
        }
    }

    pub fn set_hovered_entry_index(&mut self, index: i32) {
        self.hovered_entry = index;
    }

    pub fn hovered_entry_index(&self) -> i32 {
        self.hovered_entry
    }

    /// C++ `Wheel::MoveHoveredEntryForward` (wraps around).
    pub fn move_hovered_entry_forward(&mut self) {
        let Ok(index) = usize::try_from(self.hovered_entry) else {
            return;
        };
        if index >= self.entries.len() {
            return;
        }
        let target = if index == self.entries.len() - 1 {
            0
        } else {
            index + 1
        };
        self.entries.swap(index, target);
    }

    /// C++ `Wheel::MoveHoveredEntryBack` (wraps around).
    pub fn move_hovered_entry_back(&mut self) {
        let Ok(index) = usize::try_from(self.hovered_entry) else {
            return;
        };
        if index >= self.entries.len() {
            return;
        }
        let target = if index == 0 {
            self.entries.len() - 1
        } else {
            index - 1
        };
        self.entries.swap(index, target);
    }

    pub fn num_entries(&self) -> usize {
        self.entries.len()
    }
}

/// Cursor arc + triangle of C++ `Wheel::Draw`.
fn draw_cursor_indicator(wheel_center: [f32; 2], angle: f32, args: DrawArgs, s: &Style) {
    let w = s.wheel;
    let dist = w.inner_circle_radius - w.cursor_indicator_dist;
    let half_width = w.cursor_indicator_arc_width / 2.0;
    let half_angle = w.cursor_indicator_arc_angle / 2.0;
    drawer::draw_arc(
        wheel_center,
        dist - half_width,
        dist + half_width,
        angle - half_angle,
        angle + half_angle,
        angle - half_angle,
        angle + half_angle,
        w.cursor_indicator_color,
        32,
        args,
    );

    let side = w.cursor_indicator_triangle_side_length;
    let pts = if w.cursor_indicator_inward_facing {
        [
            [dist - half_width, side],
            [dist - half_width, -side],
            [dist - half_width - side, 0.0],
        ]
    } else {
        [
            [dist + half_width, side],
            [dist + half_width, -side],
            [dist + half_width + side, 0.0],
        ]
    };
    let p = pts.map(|p| {
        let r = geometry::rotate(p, angle);
        [r[0] + wheel_center[0], r[1] + wheel_center[1]]
    });
    drawer::draw_triangle_filled(p[0], p[1], p[2], w.cursor_indicator_color, args);
}
