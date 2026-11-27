//! Port of `WheelItemMutable` / `WheelItemMutableManager`.
//!
//! Weapons and armors are tracked by the unique ID tagged on their
//! `ExtraDataList`, since tempered/enchanted/poisoned copies of one form are
//! different items. The game may re-assign unique IDs; a
//! `TESUniqueIDChangeEvent` sink keeps the tracked IDs in sync.
//!
//! Instead of raw pointers to live items (C++), the registry holds weak
//! references to the shared [`MutableId`] of each item.

use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex, OnceLock, Weak};

use libskyrim::re::{
    EnchantmentItem, ExtraDataList, ExtraDataType, ExtraEnchantment, TESBoundObject, TESForm,
    TESUniqueIDChangeEvent,
};
use libskyrim::sdk::events::game;
use libskyrim::{skse_error, skse_message};

use super::equip::{extra_lists, is_clean, unique_id_of};
use super::{Form, ItemCtx};

/// Form ID + (mutable) unique ID of one tracked item.
#[derive(Debug)]
pub struct MutableId {
    form_id: u32,
    unique_id: AtomicU16,
}

impl MutableId {
    pub fn get(&self) -> u16 {
        self.unique_id.load(Ordering::Relaxed)
    }
}

static TRACKED: Mutex<Vec<Weak<MutableId>>> = Mutex::new(Vec::new());

fn track(form_id: u32, unique_id: u16) -> Arc<MutableId> {
    let id = Arc::new(MutableId {
        form_id,
        unique_id: AtomicU16::new(unique_id),
    });
    let mut tracked = TRACKED.lock().unwrap_or_else(|e| e.into_inner());
    tracked.retain(|w| w.strong_count() > 0);
    tracked.push(Arc::downgrade(&id));
    id
}

/// C++ `WheelItemMutableManager::ProcessEvent`
fn on_unique_id_change(event: &TESUniqueIDChangeEvent) {
    const PLAYER_REF_ID: u32 = 0x14;
    if (event.old_base_id != PLAYER_REF_ID && event.new_base_id != PLAYER_REF_ID)
        || event.old_unique_id == event.new_unique_id
        || event.new_unique_id == 0
    {
        return;
    }
    let Some(form) = TESForm::lookup_by_id(event.object_id).and_then(|f| unsafe { f.as_ref() })
    else {
        return;
    };
    let form_id = form.get_form_id();
    let tracked = TRACKED.lock().unwrap_or_else(|e| e.into_inner());
    for id in tracked.iter().filter_map(Weak::upgrade) {
        if id.form_id == form_id && id.get() == event.old_unique_id {
            skse_message!(
                "{}'s unique id changed from {} to {} due to external changes.",
                form.get_name_as_str(),
                event.old_unique_id,
                event.new_unique_id
            );
            id.unique_id.store(event.new_unique_id, Ordering::Relaxed);
        }
    }
}

/// C++ `WheelItemMutableManager::Register`
pub fn register() {
    static SUBSCRIBED: OnceLock<()> = OnceLock::new();
    SUBSCRIBED.get_or_init(|| {
        match game::subscribe::<TESUniqueIDChangeEvent, _, _>(|e| {
            if let Some(e) = e {
                on_unique_id_change(e);
            }
        }) {
            // The sink must stay registered for the whole process.
            Ok(subscription) => core::mem::forget(subscription),
            Err(e) => skse_error!("Failed to register TESUniqueIDChangeEvent sink: {e:?}"),
        }
    });
}

/// The shared state of `WheelItemWeapon` / `WheelItemArmor`.
pub struct Mutable {
    pub obj: Form<TESBoundObject>,
    id: Arc<MutableId>,
}

impl Mutable {
    /// C++ `WheelItemMutable::CreateWheelItemMutable` (tracking part).
    pub fn new(obj: Form<TESBoundObject>, unique_id: u16) -> Self {
        Self {
            id: track(obj.form_id(), unique_id),
            obj,
        }
    }

    pub fn unique_id(&self) -> u16 {
        self.id.get()
    }

    /// C++ `GetItemExtraDataAndCount`: how many functionally identical copies
    /// of this item the inventory holds, and the extra data list of this one.
    ///
    /// A modified (tempered/enchanted/poisoned) item counts as 1; a clean one
    /// counts every clean copy of the same form.
    pub fn extra_data_and_count(&self, ctx: &ItemCtx) -> (i32, *mut ExtraDataList) {
        let Some(entry) = ctx.entry(self.obj.ptr()) else {
            return (0, core::ptr::null_mut());
        };
        let unique_id = self.unique_id();
        let mut clean_count = 0;
        let mut target_clean = false;
        let mut target = core::ptr::null_mut();
        for list in extra_lists(entry) {
            // NOTE: the C++ fork tests `HasType(kEnchantment) && !HasType(kHealth)`
            // here, contradicting its own comment and `GetWeaponEquippedHand`;
            // the documented meaning (unmodified) is used instead.
            let clean = is_clean(list);
            if clean {
                clean_count += 1;
            }
            if unique_id_of(list) == Some(unique_id) {
                target = core::ptr::from_ref(list).cast_mut();
                if clean {
                    target_clean = true;
                } else {
                    return (1, target);
                }
            }
        }
        (if target_clean { clean_count } else { 0 }, target)
    }

    /// C++ `GetItemEnchantment(...)[0]`: the form's own enchantment, else the
    /// one on this instance's extra data.
    pub fn enchantment(&self, base: *mut EnchantmentItem, ctx: &ItemCtx) -> *mut EnchantmentItem {
        if !base.is_null() {
            return base;
        }
        let Some(entry) = ctx.entry(self.obj.ptr()) else {
            return core::ptr::null_mut();
        };
        let unique_id = self.unique_id();
        extra_lists(entry)
            .find(|list| unique_id_of(list) == Some(unique_id))
            .filter(|list| list.has_type(ExtraDataType::Enchantment))
            .and_then(|list| unsafe { list.get_by_type_typed::<ExtraEnchantment>().as_ref() })
            .map_or(core::ptr::null_mut(), |x| x.enchantment)
    }
}
