//! Ports of `Wheeler/WheelItems/*`: the things that can sit in a wheel entry.
//!
//! Every item implements [`wheeler_imgui::wheel::WheelItem`] over [`ItemCtx`]
//! (the player's inventory, like the C++ `InventoryItemMap&` argument).

pub mod alchemy;
pub mod ammo;
pub mod armor;
pub mod equip;
pub mod factory;
pub mod light;
pub mod misc;
pub mod mutable;
pub mod scroll;
pub mod shout;
pub mod spell;
pub mod weapon;

use core::ptr::NonNull;

use libskyrim::re::{
    BSString, FormCastable, InventoryEntryData, ItemCard, MagicItem, PlayerCharacter,
    TESBoundObject, TESDescription, TESForm, TESObjectREFR, TESObjectREFRInventoryItemMap,
};
use wheeler_imgui::image::{IconImageType, Image};

/// Per-frame item context: the player's inventory
/// (C++ `RE::TESObjectREFR::InventoryItemMap`).
#[derive(Default)]
pub struct ItemCtx {
    pub inv: TESObjectREFRInventoryItemMap,
}

impl ItemCtx {
    /// C++ `PlayerCharacter::GetSingleton()->GetInventory()`
    pub fn current() -> Self {
        let pc = PlayerCharacter::get_singleton();
        match unsafe { pc.cast::<TESObjectREFR>().as_mut() } {
            Some(refr) => Self {
                inv: refr.get_inventory(),
            },
            None => Self::default(),
        }
    }

    /// The inventory entry of `obj`.
    pub fn entry(&self, obj: *mut TESBoundObject) -> Option<&InventoryEntryData> {
        self.inv.get(&obj).map(|(_, entry)| &**entry)
    }

    /// Raw pointer to the entry, for engine calls that take `InventoryEntryData*`.
    pub fn entry_ptr(&self, obj: *mut TESBoundObject) -> *mut InventoryEntryData {
        self.entry(obj)
            .map_or(core::ptr::null_mut(), |e| core::ptr::from_ref(e).cast_mut())
    }

    pub fn contains(&self, obj: *mut TESBoundObject) -> bool {
        self.inv.contains_key(&obj)
    }

    /// Item count, 0 when absent.
    pub fn count(&self, obj: *mut TESBoundObject) -> i32 {
        self.inv.get(&obj).map_or(0, |(count, _)| *count)
    }
}

/// A non-null pointer to an engine-owned form. Forms live for the whole game
/// session, so items may hold and send them freely.
pub struct Form<T>(NonNull<T>);

// SAFETY: forms are engine-owned, never freed while a save is loaded, and
// only dereferenced on game/render threads like in the C++ original.
unsafe impl<T> Send for Form<T> {}
unsafe impl<T> Sync for Form<T> {}

impl<T> Clone for Form<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Form<T> {}

impl<T> Form<T> {
    pub fn new(ptr: *mut T) -> Option<Self> {
        NonNull::new(ptr).map(Self)
    }

    pub fn ptr(self) -> *mut T {
        self.0.as_ptr()
    }

    /// Reinterprets the pointer (C++ `static_cast` along the inheritance chain).
    pub fn cast<U>(self) -> Form<U> {
        Form(self.0.cast())
    }

    pub fn form(self) -> &'static TESForm {
        unsafe { &*self.0.as_ptr().cast::<TESForm>() }
    }

    pub fn form_id(self) -> u32 {
        self.form().get_form_id()
    }

    pub fn name(self) -> String {
        self.form().get_name_as_str().to_owned()
    }
}

impl<T> core::ops::Deref for Form<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { self.0.as_ref() }
    }
}

/// C++ `form->As<T>()`: checked downcast by form type.
pub fn form_as<T: FormCastable>(form: &TESForm) -> Option<Form<T>> {
    form.is(T::TARGET_FORM_TYPE)
        .then(|| Form(NonNull::from(form).cast()))
}

/// `'DESC'`, the default `TESDescription::GetDescription` field type.
const DESC: u32 = u32::from_be_bytes(*b"DESC");

/// C++ `x->GetDescription(buf, nullptr)`
pub fn description(desc: &TESDescription) -> String {
    let mut buf = BSString::new();
    TESDescription::get_description(desc, &mut buf, core::ptr::null_mut(), DESC);
    buf.as_str().to_owned()
}

/// C++ `Utils::Magic::GetMagicItemDescription`: the item-card effect text
/// without formatting tags.
pub fn magic_item_description(item: *mut MagicItem) -> String {
    if item.is_null() {
        return String::new();
    }
    let mut card = ItemCard::detached();
    let mut buf = BSString::new();
    card.get_magic_item_description(item, &mut buf);
    wheeler_core::texts::strip_markup(buf.as_str())
}

/// C++ `Texture::GetIconImage(type, form)`: custom `icons_custom` icons of
/// `form` win over the built-in `icon`.
pub fn icon<T>(icon: IconImageType, form: Form<T>) -> Image {
    crate::textures::icon_for(icon, Some(form.form()))
}
