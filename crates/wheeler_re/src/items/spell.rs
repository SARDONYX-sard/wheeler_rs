//! Port of `WheelItemSpell`.

use libskyrim::re::{ActorValue, Delivery, MagicItem, SpellItem, SpellType};
use wheeler_core::save::ItemData;
use wheeler_imgui::drawer::DrawArgs;
use wheeler_imgui::image::{IconImageType, Image};
use wheeler_imgui::wheel::{self, WheelItem};

use super::equip;
use super::{Form, ItemCtx, description, icon, magic_item_description};

pub const ITEM_TYPE_STR: &str = "WheelItemSpell";

pub struct WheelItemSpell {
    spell: Form<SpellItem>,
    texture: Image,
    description: String,
}

/// C++ icon selection shared by spells and potions: the costliest effect's
/// magic skill, else its primary actor value.
pub(super) fn costliest_effect_av(item: &MagicItem) -> Option<(ActorValue, ActorValue)> {
    let effect = unsafe {
        item.get_costliest_effect_item(Delivery::None, false)
            .as_ref()
    }?;
    let setting = unsafe { effect.base_effect.as_ref() }?;
    let mut av = setting.get_magick_skill();
    if av == ActorValue::None {
        av = setting.data.primary_av;
    }
    Some((av, setting.data.resist_variable))
}

fn icon_type(spell: &SpellItem) -> IconImageType {
    use IconImageType as I;
    let icon = match costliest_effect_av(spell.as_ref()) {
        Some((ActorValue::Alteration, _)) => I::alteration,
        Some((ActorValue::Conjuration, _)) => I::conjuration,
        Some((ActorValue::Destruction, resist)) => match resist {
            ActorValue::ResistFire => I::destruction_fire,
            ActorValue::ResistFrost => I::destruction_frost,
            ActorValue::ResistShock => I::destruction_shock,
            _ => I::destruction,
        },
        Some((ActorValue::Illusion, _)) => I::illusion,
        Some((ActorValue::Restoration, _)) => I::restoration,
        _ => I::spell_default,
    };
    // No school icon: maybe a power.
    if icon == I::spell_default
        && matches!(
            spell.data.spell_type,
            SpellType::Power | SpellType::LesserPower
        )
    {
        I::power
    } else {
        icon
    }
}

impl WheelItemSpell {
    pub fn new(spell: Form<SpellItem>) -> Self {
        Self {
            spell,
            texture: icon(icon_type(&spell), spell),
            description: description((*spell).as_ref()),
        }
    }

    pub fn form_id(&self) -> u32 {
        self.spell.form_id()
    }

    fn is_power(&self) -> bool {
        matches!(
            self.spell.get_spell_type(),
            SpellType::Ability | SpellType::Power | SpellType::LesserPower
        )
    }

    fn selected_power_is_this(&self) -> bool {
        equip::player().is_some_and(|pc| {
            unsafe { pc.get_actor_runtime_data().selected_power.as_ref() }
                .is_some_and(|p| p.get_form_id() == self.form_id())
        })
    }

    fn equipped_in(&self, left: bool) -> bool {
        equip::player().is_some_and(|pc| {
            unsafe { pc.get_equipped_object(left).as_ref() }
                .is_some_and(|o| o.get_form_id() == self.form_id())
        })
    }

    /// C++ `Utils::Slot::UnEquipSpell`
    fn unequip_spell(&self, hand: i32) {
        if let (Some(pc), Some(aem)) = (equip::player_actor(), equip::equip_manager()) {
            aem.unequip_spell(pc, self.spell.ptr(), hand);
        }
    }

    fn equip_spell(&self, slot: *const libskyrim::re::BGSEquipSlot) {
        if let (Some(pc), Some(aem)) = (equip::player_actor(), equip::equip_manager()) {
            aem.equip_spell(pc, self.spell.ptr(), slot);
        }
    }

    fn activate(&self, left: bool) {
        let slot = if left {
            equip::left_hand_slot()
        } else {
            equip::right_hand_slot()
        };
        if self.is_power() {
            if self.selected_power_is_this() {
                for hand in [2, 1, 0] {
                    self.unequip_spell(hand);
                }
            } else {
                self.equip_spell(slot);
            }
        } else if self.equipped_in(left) {
            self.unequip_spell(if left { 0 } else { 1 });
        } else {
            self.equip_spell(slot);
        }
    }
}

impl WheelItem<ItemCtx> for WheelItemSpell {
    fn save_data(&self) -> Option<ItemData> {
        Some(ItemData::new(ITEM_TYPE_STR, self.form_id(), None))
    }

    fn draw_slot(&self, center: [f32; 2], _hovered: bool, _ctx: &ItemCtx, args: DrawArgs) {
        wheel::draw_slot_text(center, &self.spell.name(), args);
        wheel::draw_slot_texture(self.texture, center, args);
    }

    fn draw_highlight(&self, center: [f32; 2], _ctx: &ItemCtx, args: DrawArgs) {
        wheel::draw_highlight_text(center, &self.spell.name(), args);
        wheel::draw_highlight_texture(self.texture, center, args);
        let desc = if self.description.is_empty() {
            magic_item_description(self.spell.cast::<MagicItem>().ptr())
        } else {
            self.description.clone()
        };
        if !desc.is_empty() {
            wheel::draw_highlight_description(center, &desc, args);
        }
    }

    fn is_active(&self, _ctx: &ItemCtx) -> bool {
        // Powers can sit in the voice slot or either hand.
        (self.is_power() && self.selected_power_is_this())
            || self.equipped_in(true)
            || self.equipped_in(false)
    }

    fn is_available(&self, _ctx: &ItemCtx) -> bool {
        equip::player().is_some_and(|pc| pc.has_spell(self.spell.ptr()))
    }

    fn activate_primary(&mut self, _ctx: &ItemCtx) {
        self.activate(false);
    }

    fn activate_secondary(&mut self, _ctx: &ItemCtx) {
        self.activate(true);
    }

    // `ActivateItemSpecial` (instant cast) is disabled in C++ too.
}
