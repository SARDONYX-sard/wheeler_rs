//! Port of `WheelItemMisc`.

use libskyrim::re::{TESBoundObject, TESObjectMISC};
use wheeler_core::save::ItemData;
use wheeler_imgui::drawer::DrawArgs;
use wheeler_imgui::image::{IconImageType, Image};
use wheeler_imgui::wheel::{self, WheelItem};

use super::equip;
use super::{Form, ItemCtx, icon};

pub const ITEM_TYPE_STR: &str = "WheelItemMisc";

pub struct WheelItemMisc {
    item: Form<TESObjectMISC>,
    texture: Image,
}

impl WheelItemMisc {
    pub fn new(item: Form<TESObjectMISC>) -> Self {
        Self {
            item,
            texture: icon(IconImageType::icon_default, item),
        }
    }

    pub fn form_id(&self) -> u32 {
        self.item.form_id()
    }

    fn obj(&self) -> *mut TESBoundObject {
        self.item.ptr().cast()
    }
}

impl WheelItem<ItemCtx> for WheelItemMisc {
    fn save_data(&self) -> Option<ItemData> {
        Some(ItemData::new(ITEM_TYPE_STR, self.form_id(), None))
    }

    fn draw_slot(&self, center: [f32; 2], _hovered: bool, _ctx: &ItemCtx, args: DrawArgs) {
        wheel::draw_slot_text(center, &self.item.name(), args);
        wheel::draw_slot_texture(self.texture, center, args);
    }

    fn draw_highlight(&self, center: [f32; 2], _ctx: &ItemCtx, args: DrawArgs) {
        wheel::draw_highlight_text(center, &self.item.name(), args);
        wheel::draw_highlight_texture(self.texture, center, args);
    }

    fn is_active(&self, _ctx: &ItemCtx) -> bool {
        false
    }

    fn is_available(&self, ctx: &ItemCtx) -> bool {
        ctx.count(self.obj()) > 0
    }

    /// C++ `useItem`: "equipping" a misc item uses it.
    fn activate_primary(&mut self, _ctx: &ItemCtx) {
        equip::equip(self.obj(), core::ptr::null_mut(), core::ptr::null());
    }

    fn activate_secondary(&mut self, _ctx: &ItemCtx) {
        equip::equip(self.obj(), core::ptr::null_mut(), core::ptr::null());
    }
}
