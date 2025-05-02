use core::ptr;

use commonlibsse_ng::re::Actor::Actor;
use commonlibsse_ng::re::ActorEquipManager::ActorEquipManager;
use commonlibsse_ng::re::BGSEquipSlot::BGSEquipSlot;
use commonlibsse_ng::re::BSCoreTypes::FormID;
use commonlibsse_ng::re::PlayerCharacter::PlayerCharacter;
use commonlibsse_ng::re::SpellItem::SpellItem;
use commonlibsse_ng::re::TESBoundObject::TESBoundObject;
use commonlibsse_ng::re::TESForm::TESForm;
use commonlibsse_ng::re::TESShout::TESShout;

#[commonlibsse_ng::relocate_fn(se_id = 23150, ae_id = 23607)]
#[inline]
pub fn get_left_hand_slot() -> *mut BGSEquipSlot {}

#[commonlibsse_ng::relocate_fn(se_id = 23151, ae_id = 23608)]
#[inline]
pub fn get_right_hand_slot() -> *mut BGSEquipSlot {}

#[commonlibsse_ng::relocate_fn(se_id = 23153, ae_id = 23610)]
#[inline]
pub fn get_voice_slot() -> *mut BGSEquipSlot {}

pub trait ActorEquipManagerExt {
    /// # Safety
    /// valid pointer of PlayerCharacter
    unsafe fn clean_slot(&mut self, pc: *mut PlayerCharacter, slot: *mut BGSEquipSlot);
    fn unequip_spell(&mut self, pc: *mut PlayerCharacter, spell: *mut SpellItem, hand: i32);
    fn unequip_shout(&mut self, actor: *mut Actor, spell: *mut TESShout);
}

impl ActorEquipManagerExt for ActorEquipManager {
    #[inline]
    unsafe fn clean_slot(&mut self, pc: *mut PlayerCharacter, slot: *mut BGSEquipSlot) {
        let Some(dummy) = TESForm::lookup_by_id(FormID::new(0x00020163)) else {
            return;
        };
        let Some(pc) = (unsafe { pc.as_mut() }) else {
            return;
        };
        let actor = &mut pc.__base.__base;
        let dummy = dummy.cast::<TESBoundObject>(); // unsafe downcast

        self.equip_object(
            actor,
            dummy.as_ptr(),
            ptr::null_mut(),
            1,
            slot,
            false,
            true,
            false,
            false,
        );
    }

    #[commonlibsse_ng::relocate_fn(se_id = 37947, ae_id = 38903)]
    #[inline]
    fn unequip_spell(&mut self, pc: *mut PlayerCharacter, spell: *mut SpellItem, hand: i32) {}

    #[commonlibsse_ng::relocate_fn(se_id = 37948, ae_id = 38904)]
    #[inline]
    fn unequip_shout(&mut self, actor: *mut Actor, spell: *mut TESShout) {}
}
