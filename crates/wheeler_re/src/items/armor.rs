//! Port of `WheelItemArmor`.

use libskyrim::re::{
    BGSBipedObjectForm, BGSEquipType, BGSKeywordFormExt, BipedObjectSlot as Slot, MagicItem,
    TESBoundObject, TESEnchantableForm, TESObjectARMO,
};
use wheeler_core::save::ItemData;
use wheeler_imgui::drawer::DrawArgs;
use wheeler_imgui::image::{IconImageType, Image};
use wheeler_imgui::wheel::{self, WheelItem};

use super::equip;
use super::mutable::Mutable;
use super::{Form, ItemCtx, description, icon, magic_item_description};

pub const ITEM_TYPE_STR: &str = "WheelItemArmor";

pub struct WheelItemArmor {
    m: Mutable,
    armor: Form<TESObjectARMO>,
    texture: Image,
    stat_texture: Image,
    description: String,
}

fn icon_type(armor: &TESObjectARMO) -> IconImageType {
    use IconImageType as I;
    let biped: &BGSBipedObjectForm = armor.as_ref();
    let part = |slot: Slot| biped.has_part_of(slot);
    let kw = |k: &str| armor.has_keyword_string(k);
    let head = || part(Slot::kHead) || part(Slot::kHair) || part(Slot::kCirclet);
    let foot = || part(Slot::kFeet) || part(Slot::kCalves);

    if kw("ArmorJewelry") || kw("VendorItemJewelry") || kw("JewelryExpensive") {
        if kw("ClothingNecklace") || part(Slot::kAmulet) {
            I::armor_necklace
        } else if kw("ClothingCirclet") || part(Slot::kCirclet) {
            I::armor_circlet
        } else if kw("ClothingRing") || part(Slot::kRing) {
            I::armor_ring
        } else {
            I::armor_default
        }
    } else if biped.is_heavy_armor() {
        if part(Slot::kBody) {
            I::armor_heavy_chest
        } else if head() {
            I::armor_heavy_head
        } else if foot() {
            I::armor_heavy_foot
        } else if part(Slot::kHands) {
            I::armor_heavy_arm
        } else if part(Slot::kShield) {
            I::armor_heavy_shield
        } else {
            I::armor_default
        }
    } else if biped.is_light_armor() {
        if part(Slot::kBody) {
            I::armor_light_chest
        } else if head() {
            I::armor_light_head
        } else if foot() {
            I::armor_light_foot
        } else if part(Slot::kHands) {
            I::armor_light_arm
        } else if part(Slot::kShield) {
            I::armor_light_shield
        } else {
            I::armor_default
        }
    } else if biped.is_clothing() {
        if part(Slot::kBody) {
            I::armor_clothing_chest
        } else if head() {
            I::armor_clothing_head
        } else if foot() {
            I::armor_clothing_foot
        } else if part(Slot::kHands) {
            I::armor_clothing_arm
        } else {
            I::armor_default
        }
    } else {
        I::armor_default
    }
}

impl WheelItemArmor {
    pub fn new(armor: Form<TESObjectARMO>, unique_id: u16) -> Self {
        Self {
            m: Mutable::new(armor.cast(), unique_id),
            armor,
            texture: icon(icon_type(&armor), armor),
            stat_texture: crate::textures::icon(IconImageType::armor_default),
            description: description((*armor).as_ref()),
        }
    }

    pub fn form_id(&self) -> u32 {
        self.armor.form_id()
    }

    pub fn unique_id(&self) -> u16 {
        self.m.unique_id()
    }

    fn obj(&self) -> *mut TESBoundObject {
        self.m.obj.ptr()
    }

    fn equip_slot(&self) -> *const libskyrim::re::BGSEquipSlot {
        AsRef::<BGSEquipType>::as_ref(&*self.armor).get_equip_slot()
    }

    /// C++ `WheelItemArmor::toggleEquip`
    fn toggle_equip(&self) {
        if !equip::player_3d_loaded() {
            return;
        }
        let ctx = ItemCtx::current();
        if self.is_active(&ctx) {
            equip::unequip(self.obj(), self.equip_slot());
        } else {
            let (count, extra) = self.m.extra_data_and_count(&ctx);
            if count > 0 {
                equip::equip(self.obj(), extra, self.equip_slot());
            }
        }
    }
}

impl WheelItem<ItemCtx> for WheelItemArmor {
    fn save_data(&self) -> Option<ItemData> {
        Some(ItemData::new(
            ITEM_TYPE_STR,
            self.form_id(),
            Some(self.unique_id()),
        ))
    }

    fn draw_slot(&self, center: [f32; 2], _hovered: bool, ctx: &ItemCtx, args: DrawArgs) {
        let mut text = self.armor.name();
        let count = self.m.extra_data_and_count(ctx).0;
        if count > 1 {
            text += &format!(" ({count})");
        }
        wheel::draw_slot_text(center, &text, args);
        wheel::draw_slot_texture(self.texture, center, args);
    }

    fn draw_highlight(&self, center: [f32; 2], ctx: &ItemCtx, args: DrawArgs) {
        wheel::draw_highlight_text(center, &self.armor.name(), args);
        wheel::draw_highlight_texture(self.texture, center, args);

        let entry = ctx.entry_ptr(self.obj());
        let desc = if !self.description.is_empty() {
            self.description.clone()
        } else if !entry.is_null() {
            let base = AsRef::<TESEnchantableForm>::as_ref(&*self.armor).form_enchanting;
            magic_item_description(self.m.enchantment(base, ctx).cast::<MagicItem>())
        } else {
            String::new()
        };
        wheel::draw_highlight_description(center, &desc, args);

        let rating = match (entry.is_null(), equip::player()) {
            (false, Some(pc)) => pc.get_armor_value(entry),
            _ => self.armor.get_armor_rating(),
        };
        wheel::draw_highlight_stat_icon_and_value(center, self.stat_texture, rating, args);
    }

    fn is_active(&self, ctx: &ItemCtx) -> bool {
        ctx.entry(self.obj()).is_some_and(|e| e.is_worn())
    }

    fn is_available(&self, ctx: &ItemCtx) -> bool {
        self.m.extra_data_and_count(ctx).0 > 0
    }

    fn activate_primary(&mut self, _ctx: &ItemCtx) {
        self.toggle_equip();
    }

    fn activate_secondary(&mut self, _ctx: &ItemCtx) {
        self.toggle_equip();
    }
}
