//! Port of `WheelItemAmmo`.

use libskyrim::re::{TESAmmo, TESBoundObject};
use wheeler_core::save::ItemData;
use wheeler_imgui::drawer::DrawArgs;
use wheeler_imgui::image::{IconImageType, Image};
use wheeler_imgui::wheel::{self, WheelItem};

use super::equip;
use super::{Form, ItemCtx, description, icon};

pub const ITEM_TYPE_STR: &str = "WheelItemAmmo";

pub struct WheelItemAmmo {
    ammo: Form<TESAmmo>,
    texture: Image,
    stat_texture: Image,
    description: String,
}

impl WheelItemAmmo {
    pub fn new(ammo: Form<TESAmmo>) -> Self {
        Self {
            ammo,
            texture: icon(IconImageType::arrow, ammo),
            stat_texture: crate::textures::icon(IconImageType::weapon_damage),
            description: description(ammo.as_description_form()),
        }
    }

    pub fn form_id(&self) -> u32 {
        self.ammo.form_id()
    }

    fn obj(&self) -> *mut TESBoundObject {
        self.ammo.ptr().cast()
    }
}

/// C++ `toggleEquip` of ammo/light: equip, or unequip when worn.
pub(super) fn toggle_worn(obj: *mut TESBoundObject) {
    let ctx = ItemCtx::current();
    let Some(entry) = ctx.entry(obj) else {
        return;
    };
    if entry.is_worn() {
        equip::unequip(obj, core::ptr::null());
    } else {
        equip::equip(obj, core::ptr::null_mut(), core::ptr::null());
    }
}

impl WheelItem<ItemCtx> for WheelItemAmmo {
    fn save_data(&self) -> Option<ItemData> {
        Some(ItemData::new(ITEM_TYPE_STR, self.form_id(), None))
    }

    fn draw_slot(&self, center: [f32; 2], _hovered: bool, ctx: &ItemCtx, args: DrawArgs) {
        let text = format!("{} ({})", self.ammo.name(), ctx.count(self.obj()));
        wheel::draw_slot_text(center, &text, args);
        wheel::draw_slot_texture(self.texture, center, args);
    }

    fn draw_highlight(&self, center: [f32; 2], ctx: &ItemCtx, args: DrawArgs) {
        wheel::draw_highlight_text(center, &self.ammo.name(), args);
        wheel::draw_highlight_texture(self.texture, center, args);
        if !self.description.is_empty() {
            wheel::draw_highlight_description(center, &self.description, args);
        }
        let entry = ctx.entry_ptr(self.obj());
        let damage = match (entry.is_null(), equip::player()) {
            (false, Some(pc)) => pc.get_damage(entry),
            _ => self.ammo.get_runtime_data().data.damage,
        };
        wheel::draw_highlight_stat_icon_and_value(center, self.stat_texture, damage, args);
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
