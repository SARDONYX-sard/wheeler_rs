//! Port of `WheelItemAlchemy` (food, potions, poisons).

use libskyrim::re::{ActorValue, AlchemyFlag, AlchemyItem, BGSKeywordFormExt, MagicItem};
use wheeler_core::save::ItemData;
use wheeler_imgui::drawer::DrawArgs;
use wheeler_imgui::image::{IconImageType, Image};
use wheeler_imgui::wheel::{self, WheelItem};

use super::equip;
use super::spell::costliest_effect_av;
use super::{Form, ItemCtx, icon, magic_item_description};

pub const ITEM_TYPE_STR: &str = "WheelItemAlchemy";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Food,
    Potion,
    Poison,
    None,
}

pub struct WheelItemAlchemy {
    item: Form<AlchemyItem>,
    kind: Kind,
    texture: Image,
    description: String,
}

fn classify(item: &AlchemyItem) -> (Kind, IconImageType) {
    use IconImageType as I;
    let flags = item.data.flags;
    let kw = |k: &str| item.has_keyword_string(k);
    if flags.any(AlchemyFlag::FoodItem) || kw("VendorItemFood") {
        (Kind::Food, I::food)
    } else if flags.any(AlchemyFlag::Poison) || kw("VendorItemPoison") {
        (Kind::Poison, I::poison_default)
    } else if flags.any(AlchemyFlag::Medicine) || kw("VendorItemPotion") {
        let icon = match costliest_effect_av(item.as_ref()).map(|(av, _)| av) {
            Some(ActorValue::Health | ActorValue::HealRateMult | ActorValue::HealRate) => {
                I::potion_health
            }
            Some(ActorValue::Stamina | ActorValue::StaminaRateMult | ActorValue::StaminaRate) => {
                I::potion_stamina
            }
            Some(ActorValue::Magicka | ActorValue::MagickaRateMult | ActorValue::MagickaRate) => {
                I::potion_magicka
            }
            Some(ActorValue::ResistFire) => I::potion_fire_resist,
            Some(ActorValue::ResistShock) => I::potion_shock_resist,
            Some(ActorValue::ResistFrost) => I::potion_frost_resist,
            Some(ActorValue::ResistMagic) => I::potion_magic_resist,
            _ => I::potion_default,
        };
        (Kind::Potion, icon)
    } else {
        (Kind::None, I::food)
    }
}

impl WheelItemAlchemy {
    pub fn new(item: Form<AlchemyItem>) -> Self {
        let (kind, icon_type) = classify(&item);
        Self {
            item,
            kind,
            texture: icon(icon_type, item),
            description: magic_item_description(item.cast::<MagicItem>().ptr()),
        }
    }

    pub fn form_id(&self) -> u32 {
        self.item.form_id()
    }

    /// C++ `consume` / `applyPoison`: equipping an alchemy item uses it.
    fn use_item(&self) {
        match self.kind {
            Kind::Food | Kind::Potion => equip::equip(
                self.item.ptr().cast(),
                core::ptr::null_mut(),
                core::ptr::null(),
            ),
            Kind::Poison => {
                equip::equip(
                    self.item.ptr().cast(),
                    core::ptr::null_mut(),
                    core::ptr::null(),
                );
                // Close the wheel for the poison pop-up.
                crate::wheeler::request_close();
            }
            Kind::None => {}
        }
    }
}

impl WheelItem<ItemCtx> for WheelItemAlchemy {
    fn save_data(&self) -> Option<ItemData> {
        Some(ItemData::new(ITEM_TYPE_STR, self.form_id(), None))
    }

    fn draw_slot(&self, center: [f32; 2], _hovered: bool, ctx: &ItemCtx, args: DrawArgs) {
        // C++ guards against a garbage name pointer seen on some crafted
        // potions; `get_name` already maps null to "".
        let mut name = self.item.name();
        if name.is_empty() {
            name = "Consumed out Item".to_owned();
        }
        let count = ctx.count(self.item.ptr().cast());
        wheel::draw_slot_text(center, &format!("{name} ({count})"), args);
        wheel::draw_slot_texture(self.texture, center, args);
    }

    fn draw_highlight(&self, center: [f32; 2], _ctx: &ItemCtx, args: DrawArgs) {
        wheel::draw_highlight_text(center, &self.item.name(), args);
        wheel::draw_highlight_texture(self.texture, center, args);
        if !self.description.is_empty() {
            wheel::draw_highlight_description(center, &self.description, args);
        }
    }

    fn is_active(&self, _ctx: &ItemCtx) -> bool {
        false
    }

    fn is_available(&self, ctx: &ItemCtx) -> bool {
        ctx.count(self.item.ptr().cast()) > 0
    }

    fn activate_primary(&mut self, _ctx: &ItemCtx) {
        self.use_item();
    }

    fn activate_secondary(&mut self, _ctx: &ItemCtx) {
        self.use_item();
    }
}
