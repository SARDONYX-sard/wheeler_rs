//! Port of `Utilities/UniqueIDHandler.cpp`.
//!
//! Every weapon/armor instance in the player's inventory gets its own
//! `ExtraDataList` carrying an `ExtraUniqueID`, so `WheelItemMutable` can tell
//! two copies of the same form apart.

use libskyrim::re::{
    ExtraDataList, ExtraDataType, ExtraUniqueID, FormType, ITEM_REMOVE_REASON, PlayerCharacter,
    TESBoundObject, TESForm, TESObjectREFR,
};

pub(crate) fn is_mutable_item_type(object: *const TESBoundObject) -> bool {
    match unsafe { object.cast::<TESForm>().as_ref() } {
        Some(form) => matches!(form.get_form_type(), FormType::Weapon | FormType::Armor),
        None => false,
    }
}

#[inline]
pub(crate) fn player() -> Option<&'static mut PlayerCharacter> {
    unsafe { PlayerCharacter::get_singleton().as_mut() }
}

#[inline]
pub(crate) fn player_refr(player: &mut PlayerCharacter) -> &mut TESObjectREFR {
    // `TESObjectREFR` is the primary base of `PlayerCharacter`.
    unsafe { &mut *core::ptr::from_mut(player).cast::<TESObjectREFR>() }
}

/// C++ `UniqueIDHandler::EnsureXListUniqueness`: allocates `extra_list` when
/// null and makes sure it carries an `ExtraUniqueID`.
pub fn ensure_xlist_uniqueness(extra_list: &mut *mut ExtraDataList) {
    let Some(pc) = player() else {
        return;
    };
    let inv_changes = player_refr(pc).get_inventory_changes(false);
    let Some(inv_changes) = (unsafe { inv_changes.as_mut() }) else {
        return;
    };

    if extra_list.is_null() {
        // C++ allocated a hard-coded 24 bytes, which is too small on AE 1.6.629+
        // (`ExtraDataList` is 0x20 there); `create` uses the runtime size.
        *extra_list = ExtraDataList::create();
        if extra_list.is_null() {
            return;
        }
    }

    let list = unsafe { &mut **extra_list };
    if !list.has_type(ExtraDataType::UniqueID) {
        let next_id = inv_changes.get_next_unique_id();
        let x_id = ExtraUniqueID::create(0x14, next_id);
        list.add(x_id.cast());
    }
}

/// C++ `UniqueIDHandler::EnsureXListUniquenessInPcInventory`
pub fn ensure_xlist_uniqueness_in_pc_inventory() {
    let Some(pc) = player() else {
        return;
    };
    let pc_ptr: *mut TESObjectREFR = player_refr(pc);
    let inventory = unsafe { (*pc_ptr).get_inventory() };

    for (&bound_obj, (raw_count, entry)) in &inventory {
        if !is_mutable_item_type(entry.object) {
            continue;
        }

        let mut raw_count = *raw_count;
        if let Some(lists) = unsafe { entry.extra_lists.as_ref() } {
            for &x_list in lists.iter() {
                if let Some(list) = unsafe { x_list.as_ref() } {
                    raw_count -= list.get_count();
                    let mut x_list = x_list;
                    ensure_xlist_uniqueness(&mut x_list);
                }
            }
        }

        // Workaround from the original: adding an extra list directly does
        // not stick, so the item is moved from the player to the player and
        // the AddObjectToContainer hook attaches a fresh unique list.
        while raw_count > 0 {
            raw_count -= 1;
            unsafe {
                (*pc_ptr).remove_item(
                    bound_obj,
                    1,
                    ITEM_REMOVE_REASON::StoreInContainer,
                    core::ptr::null_mut(),
                    pc_ptr,
                    core::ptr::null(),
                    core::ptr::null(),
                );
            }
        }
    }
}
