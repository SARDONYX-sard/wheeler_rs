//! Port of `WheelItemLight` (torches).

use libskyrim::re::{TESBoundObject, TESObjectLIGH};
use wheeler_core::save::ItemData;
use wheeler_imgui::drawer::DrawArgs;
use wheeler_imgui::image::{IconImageType, Image};
use wheeler_imgui::wheel::{self, WheelItem};

use super::ammo::toggle_worn;
use super::{Form, ItemCtx, icon};

pub const ITEM_TYPE_STR: &str = "WheelItemLight";

pub struct WheelItemLight {
    light: Form<TESObjectLIGH>,
    texture: Image,
}

impl WheelItemLight {
    /// `None` unless the light can be carried (C++ throws).
    pub fn new(light: Form<TESObjectLIGH>) -> Option<Self> {
        light.can_be_carried().then(|| Self {
            light,
            texture: icon(IconImageType::torch, light),
        })
    }

    pub fn form_id(&self) -> u32 {
        self.light.form_id()
    }

    fn obj(&self) -> *mut TESBoundObject {
        self.light.ptr().cast()
    }
}

impl WheelItem<ItemCtx> for WheelItemLight {
    fn save_data(&self) -> Option<ItemData> {
        Some(ItemData::new(ITEM_TYPE_STR, self.form_id(), None))
    }

    fn draw_slot(&self, center: [f32; 2], _hovered: bool, _ctx: &ItemCtx, args: DrawArgs) {
        wheel::draw_slot_text(center, &self.light.name(), args);
        wheel::draw_slot_texture(self.texture, center, args);
    }

    fn draw_highlight(&self, center: [f32; 2], _ctx: &ItemCtx, args: DrawArgs) {
        wheel::draw_highlight_text(center, &self.light.name(), args);
        wheel::draw_highlight_texture(self.texture, center, args);
    }

    fn is_active(&self, ctx: &ItemCtx) -> bool {
        ctx.entry(self.obj()).is_some_and(|e| e.is_worn())
    }

    fn is_available(&self, ctx: &ItemCtx) -> bool {
        ctx.contains(self.obj())
    }

    fn activate_primary(&mut self, _ctx: &ItemCtx) {
        toggle_worn(self.obj());
    }

    fn activate_secondary(&mut self, _ctx: &ItemCtx) {
        toggle_worn(self.obj());
    }
}
