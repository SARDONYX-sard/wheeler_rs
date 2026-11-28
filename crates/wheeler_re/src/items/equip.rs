//! Equipping helpers (C++ `Utils::Slot`, `Utils::Inventory`).

use libskyrim::re::{
    Actor, ActorEquipManager, BGSEquipSlot, EquipOptions, ExtraDataList, ExtraDataType,
    ExtraUniqueID, InventoryEntryData, PlayerCharacter, TESBoundObject, TESForm,
    get_left_hand_slot, get_right_hand_slot,
};

pub use libskyrim::re::get_voice_slot;

pub fn left_hand_slot() -> *mut BGSEquipSlot {
    get_left_hand_slot()
}

pub fn right_hand_slot() -> *mut BGSEquipSlot {
    get_right_hand_slot()
}

pub fn player() -> Option<&'static mut PlayerCharacter> {
    unsafe { PlayerCharacter::get_singleton().as_mut() }
}

/// The player as `Actor*`, for `ActorEquipManager` calls.
pub fn player_actor() -> Option<*mut Actor> {
    let pc = PlayerCharacter::get_singleton();
    (!pc.is_null()).then(|| pc.cast())
}

pub fn equip_manager() -> Option<&'static mut ActorEquipManager> {
    unsafe { ActorEquipManager::get_singleton().as_mut() }
}

pub fn player_3d_loaded() -> bool {
    player().is_some_and(|pc| unsafe {
        (*core::ptr::from_mut(pc).cast::<libskyrim::re::TESObjectREFR>()).is_3d_loaded()
    })
}

/// `aem->EquipObject(pc, obj, extra, 1, slot)` with C++ defaults.
pub fn equip(obj: *mut TESBoundObject, extra: *mut ExtraDataList, slot: *const BGSEquipSlot) {
    if let (Some(pc), Some(aem)) = (player_actor(), equip_manager()) {
        aem.equip_object(pc, obj, extra, 1, slot, EquipOptions::DEFAULT);
    }
}

/// `aem->UnequipObject(pc, obj, nullptr, 1, slot)` with C++ defaults.
pub fn unequip(obj: *mut TESBoundObject, slot: *const BGSEquipSlot) {
    if let (Some(pc), Some(aem)) = (player_actor(), equip_manager()) {
        aem.unequip_object(
            pc,
            obj,
            core::ptr::null_mut(),
            1,
            slot,
            EquipOptions::DEFAULT,
        );
    }
}

/// C++ `Utils::Slot::CleanSlot`: force-equips and unequips a dummy weapon
/// (`0x00020163`, Unarmed) to empty `slot`.
pub fn clean_slot(slot: *const BGSEquipSlot) {
    let (Some(pc), Some(aem)) = (player_actor(), equip_manager()) else {
        return;
    };
    let Some(dummy) = TESForm::lookup_by_id(0x0002_0163).filter(|f| !f.is_null()) else {
        return;
    };
    let options = EquipOptions {
        queue_equip: false,
        force_equip: true,
        play_sounds: false,
        apply_now: false,
    };
    let dummy = dummy.cast::<TESBoundObject>();
    aem.equip_object(pc, dummy, core::ptr::null_mut(), 1, slot, options);
    aem.unequip_object(pc, dummy, core::ptr::null_mut(), 1, slot, options);
}

/// The `ExtraUniqueID` of `list`, if any.
pub fn unique_id_of(list: &ExtraDataList) -> Option<u16> {
    if !list.has_type(ExtraDataType::UniqueID) {
        return None;
    }
    unsafe { list.get_by_type_typed::<ExtraUniqueID>().as_ref() }.map(|x| x.unique_id)
}

/// Not enchanted, tempered or poisoned.
pub fn is_clean(list: &ExtraDataList) -> bool {
    !list.has_type(ExtraDataType::Enchantment)
        && !list.has_type(ExtraDataType::Health)
        && !list.has_type(ExtraDataType::Poison)
}

/// The non-null extra data lists of `entry`.
pub fn extra_lists(entry: &InventoryEntryData) -> impl Iterator<Item = &ExtraDataList> {
    unsafe { entry.extra_lists.as_ref() }
        .into_iter()
        .flat_map(|lists| lists.iter())
        .filter_map(|list| unsafe { list.as_ref() })
}

/// C++ `Utils::Inventory::Hand`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hand {
    Left,
    Right,
    Both,
    None,
}

/// C++ `Utils::Inventory::GetWeaponEquippedHand`: which hands hold the
/// weapon with `unique_id`; with `item_clean`, any unmodified instance of
/// the weapon counts.
pub fn weapon_equipped_hand(weapon_form_id: u32, unique_id: u16, item_clean: bool) -> Hand {
    let Some(pc) = player_actor() else {
        return Hand::None;
    };
    let actor = unsafe { &*pc };
    let check = |left: bool| -> (bool, bool) {
        let entry = unsafe { actor.get_equipped_entry_data(left).as_ref() };
        let Some(entry) = entry else {
            return (false, false);
        };
        let matches =
            unsafe { entry.object.as_ref() }.is_some_and(|o| o.get_form_id() == weapon_form_id);
        if !matches {
            return (false, false);
        }
        let (mut equipped, mut base) = (false, false);
        for list in extra_lists(entry) {
            if is_clean(list) {
                base = true;
            }
            if unique_id_of(list) == Some(unique_id) {
                equipped = true;
                break;
            }
        }
        (equipped, base)
    };
    let (lhs, lhs_base) = check(true);
    let (rhs, rhs_base) = check(false);

    let pick = |l: bool, r: bool| match (l, r) {
        (true, true) => Some(Hand::Both),
        (true, false) => Some(Hand::Left),
        (false, true) => Some(Hand::Right),
        (false, false) => None,
    };
    if item_clean && let Some(hand) = pick(lhs_base, rhs_base) {
        return hand;
    }
    pick(lhs, rhs).unwrap_or(Hand::None)
}
