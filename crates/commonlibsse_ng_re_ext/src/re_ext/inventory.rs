use core::ptr::NonNull;

use commonlibsse_ng::re::Actor::Actor;
use commonlibsse_ng::re::BSTList::BSSimpleList;
use commonlibsse_ng::re::EnchantmentItem::EnchantmentItem;
use commonlibsse_ng::re::ExtraDataList::ExtraDataList;
use commonlibsse_ng::re::ExtraDataType::ExtraDataType;
use commonlibsse_ng::re::ExtraEnchantment::ExtraEnchantment;
use commonlibsse_ng::re::ExtraHealth::ExtraHealth;
use commonlibsse_ng::re::ExtraUniqueID::ExtraUniqueID;
use commonlibsse_ng::re::InventoryEntryData::InventoryEntryData;
use commonlibsse_ng::re::InventoryMenu::InventoryMenu;
use commonlibsse_ng::re::PlayerCharacter::PlayerCharacter;
use commonlibsse_ng::re::TESForm::DerivedTESForm;
use commonlibsse_ng::re::TESObjectWEAP::TESObjectWEAP;

#[commonlibsse_ng::relocate_fn(se_id = 11437, ae_id = 11583)]
#[inline]
pub fn init_extra_data_list(list: *mut ExtraDataList) {}

pub fn get_entry_enchant_and_health(
    inventory: &InventoryEntryData,
) -> (Option<NonNull<EnchantmentItem>>, Option<f32>) {
    let mut ret = (None, None);

    let Some(extra_lists) = inventory.extraLists.as_ref() else {
        return ret;
    };
    let found_enchant = false;
    let found_health = false;

    for extra_list in extra_lists.iter() {
        if extra_list.has_type(ExtraDataType::Enchantment) {
            if let Some(ex_enchant) = extra_list.get_by_type_as::<ExtraEnchantment>() {
                ret.0 = unsafe { ex_enchant.as_ref() }.enchantment;
                if found_health {
                    return ret;
                }
            };
        }

        if extra_list.has_type(ExtraDataType::Health) {
            if let Some(ex_health) = extra_list.get_by_type_as::<ExtraHealth>() {
                ret.1 = Some(unsafe { ex_health.as_ref() }.health);
                if found_enchant {
                    return ret;
                }
            };
        }
    }

    ret
}

#[inline]
pub fn get_entry_extra_data_lists<'a>(
    ret: &mut Vec<&'a ExtraDataList>,
    inventory: &'a InventoryEntryData,
) {
    let Some(extra_lists) = inventory.extraLists.as_ref() else {
        return;
    };

    for extra_list in extra_lists.iter() {
        ret.push(extra_list)
    }
}

#[inline]
pub fn get_next_unique_id() -> Option<u16> {
    let pc = PlayerCharacter::get_singleton()?;
    let inventory = pc.__base.__base.__base.get_inventory_changes(false)?;

    unsafe {
        inventory
            .as_ref()
            .map(|inventory| inventory.get_next_unique_id())
    }
}

pub enum Hand {
    Left,
    Right,
    Both,
    None,
}

/// # Safety
pub unsafe fn get_weapon_equipped_hand(
    actor: *mut Actor,
    weapon: *mut TESObjectWEAP,
    unique_id: u32,
    item_clean: bool,
) -> Option<Hand> {
    let actor = unsafe { actor.as_ref() }?;

    let (lhs_equipped_base, lhs_equipped) = {
        let lhs = actor.get_equipped_entry_data(true)?;
        let left_form_id = unsafe { lhs.as_ref() }
            .get_object()?
            .__base
            .__base
            .get_form()
            .formID;
        let weapon_form_id = unsafe { weapon.as_ref() }?
            .__base0
            .__base
            .__base
            .get_form()
            .formID;

        if left_form_id == weapon_form_id {
            let left_extra_lists = unsafe { lhs.as_ref().extraLists.as_ref()? };
            equipped_status(unique_id, left_extra_lists)
        } else {
            (false, false)
        }
    };

    let (rhs_equipped_base, rhs_equipped) = {
        let rhs = actor.get_equipped_entry_data(true)?;
        let right_extra_lists = unsafe { rhs.as_ref().extraLists.as_ref()? };
        equipped_status(unique_id, right_extra_lists)
    };

    if item_clean {
        match (lhs_equipped_base, rhs_equipped_base) {
            (true, true) => return Some(Hand::Both),
            (true, false) => return Some(Hand::Left),
            (false, true) => return Some(Hand::Right),
            _ => {}
        };
    }

    Some(match (lhs_equipped, rhs_equipped) {
        (true, true) => Hand::Both,
        (true, false) => Hand::Left,
        (false, true) => Hand::Right,
        (false, false) => Hand::None,
    })
}

/// Return (equip_base, equip)
fn equipped_status(unique_id: u32, extra_lists: &BSSimpleList<ExtraDataList>) -> (bool, bool) {
    let exclude_types: ExtraDataType =
        ExtraDataType::Enchantment | ExtraDataType::Health | ExtraDataType::Poison;

    let mut equipped_base = false;
    let mut equipped = false;

    for extra_list in extra_lists {
        if !extra_list.has_type(exclude_types) {
            equipped_base = true;
        }

        if extra_list.has_type(ExtraDataType::UniqueID)
            && unsafe {
                extra_list
                    .get_by_type_as::<ExtraUniqueID>()
                    .is_some_and(|x| x.as_ref().uniqueID == (unique_id as u16))
            }
        {
            equipped = true;
            break;
        }
    }

    (equipped_base, equipped)
}

pub fn get_selected_item_in_inventory(
    inventory_menu: &InventoryMenu,
) -> Option<&InventoryEntryData> {
    let item_list = inventory_menu.get_runtime_data().ok()?.itemList;
    let selected_item = unsafe { item_list.as_ref()?.get_selected_item()?.as_ref() }?;
    unsafe { selected_item.data.objDesc.as_ref() }
}
