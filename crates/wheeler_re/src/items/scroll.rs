//! Port of `WheelItemScroll`.

use libskyrim::re::{MagicItem, ScrollItem, SpellItem, TESBoundObject};
use wheeler_core::save::ItemData;
use wheeler_imgui::drawer::DrawArgs;
use wheeler_imgui::image::{IconImageType, Image};
use wheeler_imgui::wheel::{self, WheelItem};

use super::equip;
use super::{Form, ItemCtx, description, icon, magic_item_description};

pub const ITEM_TYPE_STR: &str = "WheelItemScroll";

pub struct WheelItemScroll {
    scroll: Form<ScrollItem>,
    texture: Image,
    description: String,
}

impl WheelItemScroll {
    pub fn new(scroll: Form<ScrollItem>) -> Self {
        let spell: &SpellItem = &scroll;
        Self {
            scroll,
            texture: icon(IconImageType::scroll, scroll),
            description: description(spell.as_ref()),
        }
    }

    pub fn form_id(&self) -> u32 {
        self.scroll.form_id()
    }

    fn obj(&self) -> *mut TESBoundObject {
        self.scroll.ptr().cast()
    }

    fn equip_to(&self, left: bool) {
        if ItemCtx::current().count(self.obj()) <= 0 {
            return;
        }
        let slot = if left {
            equip::left_hand_slot()
        } else {
            equip::right_hand_slot()
        };
        equip::equip(self.obj(), core::ptr::null_mut(), slot);
    }
}

impl WheelItem<ItemCtx> for WheelItemScroll {
    fn save_data(&self) -> Option<ItemData> {
        Some(ItemData::new(ITEM_TYPE_STR, self.form_id(), None))
    }

    fn draw_slot(&self, center: [f32; 2], _hovered: bool, ctx: &ItemCtx, args: DrawArgs) {
        let mut text = self.scroll.name();
        let count = ctx.count(self.obj());
        if count > 0 {
            text += &format!(" ({count})");
        }
        wheel::draw_slot_text(center, &text, args);
        wheel::draw_slot_texture(self.texture, center, args);
    }

    fn draw_highlight(&self, center: [f32; 2], _ctx: &ItemCtx, args: DrawArgs) {
        wheel::draw_highlight_text(center, &self.scroll.name(), args);
        wheel::draw_highlight_texture(self.texture, center, args);
        let desc = if self.description.is_empty() {
            magic_item_description(self.scroll.cast::<MagicItem>().ptr())
        } else {
            self.description.clone()
        };
        wheel::draw_highlight_description(center, &desc, args);
    }

    fn is_active(&self, _ctx: &ItemCtx) -> bool {
        let Some(pc) = equip::player() else {
            return false;
        };
        [true, false].into_iter().any(|left| {
            unsafe { pc.get_equipped_object(left).as_ref() }
                .is_some_and(|o| o.get_form_id() == self.form_id())
        })
    }

    fn is_available(&self, ctx: &ItemCtx) -> bool {
        ctx.count(self.obj()) > 0
    }

    fn activate_primary(&mut self, _ctx: &ItemCtx) {
        self.equip_to(false);
    }

    fn activate_secondary(&mut self, _ctx: &ItemCtx) {
        self.equip_to(true);
    }
}
