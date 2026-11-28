//! Port of `WheelItemWeapon`.

use libskyrim::re::{
    BGSKeywordFormExt, MagicItem, TESAttackDamageFormExt, TESBoundObject, TESEnchantableForm,
    TESObjectWEAP, WeaponType,
};
use wheeler_core::save::ItemData;
use wheeler_imgui::drawer::DrawArgs;
use wheeler_imgui::image::{IconImageType, Image};
use wheeler_imgui::wheel::{self, WheelItem};

use super::equip::{self, Hand};
use super::mutable::Mutable;
use super::{Form, ItemCtx, description, icon, magic_item_description};

pub const ITEM_TYPE_STR: &str = "WheelItemWeapon";

pub struct WheelItemWeapon {
    m: Mutable,
    weapon: Form<TESObjectWEAP>,
    texture: Image,
    stat_texture: Image,
    description: String,
}

fn icon_type(weapon: &TESObjectWEAP) -> IconImageType {
    // TODO (C++): animated armory / 2h mace icons.
    match weapon.get_weapon_type() {
        WeaponType::Bow => IconImageType::bow,
        WeaponType::Crossbow => IconImageType::crossbow,
        WeaponType::Staff => IconImageType::staff,
        WeaponType::HandToHandMelee => IconImageType::hand_to_hand,
        WeaponType::OneHandDagger => IconImageType::dagger,
        WeaponType::OneHandAxe => IconImageType::axe_one_handed,
        WeaponType::OneHandMace => IconImageType::mace,
        WeaponType::TwoHandSword => IconImageType::sword_two_handed,
        WeaponType::TwoHandAxe if weapon.has_keyword_string("WeapTypeWarhammer") => {
            IconImageType::warhammer_two_handed
        }
        WeaponType::TwoHandAxe => IconImageType::axe_two_handed,
        _ => IconImageType::sword_one_handed,
    }
}

impl WheelItemWeapon {
    pub fn new(weapon: Form<TESObjectWEAP>, unique_id: u16) -> Self {
        Self {
            m: Mutable::new(weapon.cast(), unique_id),
            weapon,
            texture: icon(icon_type(&weapon), weapon),
            stat_texture: crate::textures::icon(IconImageType::weapon_damage),
            description: description((*weapon).as_ref()),
        }
    }

    pub fn form_id(&self) -> u32 {
        self.weapon.form_id()
    }

    pub fn unique_id(&self) -> u16 {
        self.m.unique_id()
    }

    fn obj(&self) -> *mut TESBoundObject {
        self.m.obj.ptr()
    }

    fn equipped_hand(&self, item_clean: bool) -> Hand {
        equip::weapon_equipped_hand(self.form_id(), self.unique_id(), item_clean)
    }

    fn is_two_handed(&self) -> bool {
        let w = &*self.weapon;
        w.is_crossbow() || w.is_bow() || w.is_two_handed_axe() || w.is_two_handed_sword()
    }

    /// C++ `WheelItemWeapon::equipItem`
    fn equip_item(&self, to_right: bool) {
        if !equip::player_3d_loaded() {
            return;
        }
        let ctx = ItemCtx::current();
        let (count, mut extra) = self.m.extra_data_and_count(&ctx);
        if count <= 0 {
            return; // nothing to equip
        }
        if count < 2 {
            // Can't dual-wield: if it's in the other hand, take it out first.
            let hand = self.equipped_hand(false);
            if (hand == Hand::Right && !to_right) || (hand == Hand::Left && to_right) {
                let opposite = if to_right {
                    equip::left_hand_slot()
                } else {
                    equip::right_hand_slot()
                };
                if let (Some(pc), Some(aem)) = (equip::player_actor(), equip::equip_manager()) {
                    let options = libskyrim::re::EquipOptions {
                        queue_equip: false,
                        force_equip: true,
                        play_sounds: true,
                        apply_now: false,
                    };
                    aem.unequip_object(pc, self.obj(), core::ptr::null_mut(), 1, opposite, options);
                }
            }
        } else {
            // More than one guarantees an untempered item; let the game pick.
            extra = core::ptr::null_mut();
        }
        if self.weapon.is_crossbow() || self.weapon.is_bow() {
            equip::clean_slot(equip::left_hand_slot());
            equip::clean_slot(equip::right_hand_slot());
        }
        let slot = if to_right {
            equip::right_hand_slot()
        } else {
            equip::left_hand_slot()
        };
        equip::equip(self.obj(), extra, slot);
    }
}

impl WheelItem<ItemCtx> for WheelItemWeapon {
    fn save_data(&self) -> Option<ItemData> {
        Some(ItemData::new(
            ITEM_TYPE_STR,
            self.form_id(),
            Some(self.unique_id()),
        ))
    }

    fn draw_slot(&self, center: [f32; 2], _hovered: bool, ctx: &ItemCtx, args: DrawArgs) {
        let mut text = self.weapon.name();
        let count = self.m.extra_data_and_count(ctx).0;
        if count > 1 {
            text += &format!(" ({count})");
        }
        wheel::draw_slot_text(center, &text, args);
        wheel::draw_slot_texture(self.texture, center, args);
    }

    fn draw_highlight(&self, center: [f32; 2], ctx: &ItemCtx, args: DrawArgs) {
        wheel::draw_highlight_text(center, &self.weapon.name(), args);
        wheel::draw_highlight_texture(self.texture, center, args);

        // A weapon's own description (special weapons) wins over its enchantment.
        let desc = if self.description.is_empty() {
            let base = AsRef::<TESEnchantableForm>::as_ref(&*self.weapon).form_enchanting;
            magic_item_description(self.m.enchantment(base, ctx).cast::<MagicItem>())
        } else {
            self.description.clone()
        };
        if !desc.is_empty() {
            wheel::draw_highlight_description(center, &desc, args);
        }

        let entry = ctx.entry_ptr(self.obj());
        let damage = match (entry.is_null(), equip::player()) {
            (false, Some(pc)) => pc.get_damage(entry),
            _ => self.weapon.get_attack_damage() as f32,
        };
        wheel::draw_highlight_stat_icon_and_value(center, self.stat_texture, damage, args);
    }

    fn is_active(&self, ctx: &ItemCtx) -> bool {
        let item_clean = self.m.extra_data_and_count(ctx).0 >= 2;
        self.equipped_hand(item_clean) != Hand::None
    }

    fn is_available(&self, ctx: &ItemCtx) -> bool {
        self.m.extra_data_and_count(ctx).0 > 0
    }

    fn activate_primary(&mut self, _ctx: &ItemCtx) {
        match self.equipped_hand(true) {
            Hand::Right | Hand::Both => equip::unequip(self.obj(), equip::right_hand_slot()),
            _ => self.equip_item(true),
        }
    }

    fn activate_secondary(&mut self, _ctx: &ItemCtx) {
        let two_handed = self.is_two_handed();
        let hand = self.equipped_hand(true);
        if hand == Hand::Left || hand == Hand::Both || (two_handed && hand == Hand::Right) {
            // Two-handed weapons are only truly unequipped from the right slot.
            let slot = if two_handed {
                equip::right_hand_slot()
            } else {
                equip::left_hand_slot()
            };
            equip::unequip(self.obj(), slot);
        } else {
            self.equip_item(false);
        }
    }
}
