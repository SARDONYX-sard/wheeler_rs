//! Port of `Hooks.cpp`. Written in the same "store the original, call it from
//! the thunk" style as the C++ (`write_vfunc` / `write_call<5>`), no detours.

use core::sync::atomic::{AtomicUsize, Ordering};

use libskyrim::re::{
    BSTEventSource, ExtraDataList, InputEvent, PlayerCharacter, TESBoundObject, TESObjectREFR,
};
use libskyrim::relocation::{self, RelocationID, VariantOffset};
use libskyrim::{skse_error, skse_message};

use crate::{input, unique_id};

/// Reads an original function pointer stored by an install routine.
macro_rules! original {
    ($slot:expr, $ty:ty) => {
        unsafe { core::mem::transmute::<usize, $ty>($slot.load(Ordering::Acquire)) }
    };
}

/// Installs a 5-byte `call` hook only when the patch site really is an
/// `E8 rel32`, so an unknown runtime (or a moved call) is logged instead of
/// corrupting code.
pub(crate) fn write_thunk_call(name: &str, src: usize, thunk: usize, slot: &AtomicUsize) -> bool {
    if src == 0 {
        skse_error!("{name}: address could not be resolved");
        return false;
    }
    let opcode = unsafe { core::ptr::read_unaligned(src as *const u8) };
    if opcode != 0xE8 {
        skse_error!("{name}: expected a call (E8) at {src:#X}, found {opcode:#04X}; hook skipped");
        return false;
    }
    let original = relocation::write_call(src, thunk, 5);
    slot.store(original, Ordering::Release);
    true
}

// ---------------------------------------------------------------------------
// OnChangePlayerInventory

/// Ensures every weapon/armor added to the player's inventory gets a unique
/// `ExtraDataList` (see [`unique_id`]).
mod on_change_player_inventory {
    use super::*;

    /// `TESObjectREFR::AddObjectToContainer` (same slot on every runtime).
    const VFUNC_ADD_OBJECT_TO_CONTAINER: usize = 0x5A;
    /// `Actor::PickUpObject`
    const VFUNC_PICK_UP_OBJECT: VariantOffset = VariantOffset::new_se_ae(0x0CC, 0x0CE);

    type AddObjectToContainer = extern "C" fn(
        *mut PlayerCharacter,
        *mut TESBoundObject,
        *mut ExtraDataList,
        i32,
        *mut TESObjectREFR,
    );
    type PickUpObject = extern "C" fn(*mut PlayerCharacter, *mut TESObjectREFR, u32, bool, bool);

    static ADD_OBJECT_TO_CONTAINER: AtomicUsize = AtomicUsize::new(0);
    static PICK_UP_OBJECT: AtomicUsize = AtomicUsize::new(0);

    pub(super) fn install() {
        let vtable = PlayerCharacter::VTABLE[0].address();
        let original = relocation::write_vfunc(
            vtable,
            VFUNC_ADD_OBJECT_TO_CONTAINER,
            hook_add_object_to_container as *const () as usize,
        );
        ADD_OBJECT_TO_CONTAINER.store(original, Ordering::Release);
        let original = relocation::write_vfunc(
            vtable,
            VFUNC_PICK_UP_OBJECT.offset(),
            hook_pick_up_object as *const () as usize,
        );
        PICK_UP_OBJECT.store(original, Ordering::Release);
        skse_message!("Installed hooks for OnChangePlayerInventory");
    }

    /// Gives every extra list of `object` in the player's inventory a unique
    /// id, attaching new lists for the remaining un-listed count.
    fn fix_inventory_entry(this: *mut PlayerCharacter, object: *mut TESBoundObject) {
        let refr = this.cast::<TESObjectREFR>();
        let inv_changes = unsafe { (*refr).get_inventory_changes(false) };
        let Some(inv_changes) = (unsafe { inv_changes.as_mut() }) else {
            return;
        };
        let Some(entries) = (unsafe { inv_changes.entry_list.as_ref() }) else {
            return;
        };

        for &entry in entries.iter() {
            let Some(entry) = (unsafe { entry.as_mut() }) else {
                continue;
            };
            if entry.object != object {
                continue;
            }

            let mut raw_count = entry.count_delta;
            if let Some(lists) = unsafe { entry.extra_lists.as_ref() } {
                for &x_list in lists.iter() {
                    if let Some(list) = unsafe { x_list.as_ref() } {
                        raw_count -= list.get_count();
                        let mut x_list = x_list;
                        unique_id::ensure_xlist_uniqueness(&mut x_list);
                    }
                }
            }

            while raw_count > 0 {
                raw_count -= 1;
                let mut x_list: *mut ExtraDataList = core::ptr::null_mut();
                unique_id::ensure_xlist_uniqueness(&mut x_list);
                entry.add_extra_list(x_list);
            }
        }
    }

    extern "C" fn hook_add_object_to_container(
        this: *mut PlayerCharacter,
        object: *mut TESBoundObject,
        extra_list: *mut ExtraDataList,
        count: i32,
        from_refr: *mut TESObjectREFR,
    ) {
        let original = original!(ADD_OBJECT_TO_CONTAINER, AddObjectToContainer);
        if count <= 0 || object.is_null() || !unique_id::is_mutable_item_type(object) {
            return original(this, object, extra_list, count, from_refr);
        }

        let mut count_left = count;
        if let Some(list) = unsafe { extra_list.as_ref() } {
            let list_count = list.get_count();
            count_left -= list_count;
            let mut extra_list = extra_list;
            unique_id::ensure_xlist_uniqueness(&mut extra_list);
            original(this, object, extra_list, list_count, from_refr);
        }

        while count_left > 0 {
            count_left -= 1;
            original(this, object, core::ptr::null_mut(), 1, from_refr);
        }

        fix_inventory_entry(this, object);
    }

    extern "C" fn hook_pick_up_object(
        this: *mut PlayerCharacter,
        object: *mut TESObjectREFR,
        count: u32,
        arg3: bool,
        play_sound: bool,
    ) {
        let original = original!(PICK_UP_OBJECT, PickUpObject);
        let Some(refr) = (unsafe { object.as_mut() }) else {
            return original(this, object, count, arg3, play_sound);
        };
        let base = refr.get_base_object();
        if base.is_null() || count == 0 || !unique_id::is_mutable_item_type(base) {
            return original(this, object, count, arg3, play_sound);
        }

        original(this, object, count, arg3, play_sound);

        let mut count_left = count as i32 - refr.extra_list.get_count();
        let mut list_ptr: *mut ExtraDataList = &mut refr.extra_list;
        unique_id::ensure_xlist_uniqueness(&mut list_ptr);

        let add = original!(ADD_OBJECT_TO_CONTAINER, AddObjectToContainer);
        while count_left > 0 {
            // Kept from the original ("why are we calling this?").
            count_left -= 1;
            add(this, base, core::ptr::null_mut(), 1, core::ptr::null_mut());
        }

        fix_inventory_entry(this, base);
    }
}

// ---------------------------------------------------------------------------
// OnInputEventDispatch

/// Hooks the input event dispatcher; [`input::process_and_filter`] edits the
/// linked list before the game's handlers see it.
mod on_input_event_dispatch {
    use super::*;

    type DispatchInputEvent =
        extern "C" fn(*mut BSTEventSource<*mut InputEvent>, *mut *mut InputEvent);
    static DISPATCH_INPUT_EVENT: AtomicUsize = AtomicUsize::new(0);

    pub(super) fn install() {
        let caller = RelocationID::new(67315, 68617).address();
        // RELOCATION_OFFSET(0x7B, 0x7B)
        // TODO: verify the call site on AE 1.7.99+ (`write_thunk_call` refuses
        // to patch when it is not a `call`).
        if caller != 0 {
            write_thunk_call(
                "OnInputEventDispatch",
                caller + 0x7B,
                dispatch_input_event as *const () as usize,
                &DISPATCH_INPUT_EVENT,
            );
        }
    }

    extern "C" fn dispatch_input_event(
        dispatcher: *mut BSTEventSource<*mut InputEvent>,
        events: *mut *mut InputEvent,
    ) {
        let original = original!(DISPATCH_INPUT_EVENT, DispatchInputEvent);
        if !events.is_null() {
            unsafe { input::process_and_filter(events) };
        }
        original(dispatcher, events);
    }
}

/// C++ `Hooks::Install`
pub fn install() {
    on_change_player_inventory::install();
    on_input_event_dispatch::install();
    skse_message!("Installed all hooks");
}
