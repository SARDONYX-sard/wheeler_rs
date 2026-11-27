//! Port of `WheelItemFactory::MakeWheelItemFromMenuHovered`.

use libskyrim::re::{
    AlchemyItem, FormType, GFxValue, InventoryMenu, MagicMenu, ScrollItem, SpellItem, TESAmmo,
    TESForm, TESObjectARMO, TESObjectLIGH, TESObjectMISC, TESObjectWEAP, TESShout, UI,
};
use libskyrim::skse_error;
use wheeler_core::save::ItemData;
use wheeler_imgui::wheel::WheelItem;

use super::equip::{self, extra_lists, unique_id_of};
use super::{
    ItemCtx, alchemy::WheelItemAlchemy, ammo::WheelItemAmmo, armor::WheelItemArmor, form_as,
    light::WheelItemLight, misc::WheelItemMisc, scroll::WheelItemScroll, shout::WheelItemShout,
    spell::WheelItemSpell, weapon::WheelItemWeapon,
};

pub type BoxedItem = Box<dyn WheelItem<ItemCtx>>;

/// Builds the wheel item for an inventory object; weapons and armors need
/// their unique ID.
pub fn from_bound_object(form: &TESForm, unique_id: u16) -> Option<BoxedItem> {
    let ty = form.get_form_type();
    let item: BoxedItem = match ty {
        FormType::Weapon if unique_id != 0 => Box::new(WheelItemWeapon::new(
            form_as::<TESObjectWEAP>(form)?,
            unique_id,
        )),
        FormType::Armor if unique_id != 0 => Box::new(WheelItemArmor::new(
            form_as::<TESObjectARMO>(form)?,
            unique_id,
        )),
        FormType::Light => Box::new(WheelItemLight::new(form_as::<TESObjectLIGH>(form)?)?),
        FormType::Ammo => Box::new(WheelItemAmmo::new(form_as::<TESAmmo>(form)?)),
        FormType::AlchemyItem => Box::new(WheelItemAlchemy::new(form_as::<AlchemyItem>(form)?)),
        FormType::Scroll => Box::new(WheelItemScroll::new(form_as::<ScrollItem>(form)?)),
        FormType::Misc => Box::new(WheelItemMisc::new(form_as::<TESObjectMISC>(form)?)),
        FormType::Spell => Box::new(WheelItemSpell::new(form_as::<SpellItem>(form)?)),
        FormType::Shout => Box::new(WheelItemShout::new(form_as::<TESShout>(form)?)),
        _ => return None,
    };
    Some(item)
}

/// C++ `Utils::Inventory::GetSelectedItemIninventory` + item creation.
fn from_inventory_menu(ui: &UI) -> Option<BoxedItem> {
    let menu = ui.get_menu(InventoryMenu::MENU_NAME);
    let menu = unsafe { menu.as_ptr().cast::<InventoryMenu>().as_ref() }?;
    let item_list = unsafe { menu.get_runtime_data().item_list.as_ref() }?;
    let selected = unsafe { item_list.get_selected_item().as_ref() }?;
    let entry = unsafe { selected.data.obj_desc.as_ref() }?;
    let obj = unsafe { entry.object.as_ref() }?;

    let form: &TESForm = obj;
    let unique_id = match obj.get_form_type() {
        FormType::Weapon | FormType::Armor => {
            if entry.extra_lists.is_null() {
                skse_error!("ExtraDataList for {} is nullptr", obj.get_name_as_str());
                return None;
            }
            // Items without a unique ID can't be tracked.
            extra_lists(entry)
                .find_map(unique_id_of)
                .filter(|&id| id != 0)?
        }
        _ => 0,
    };
    from_bound_object(form, unique_id)
}

/// C++ `Utils::Inventory::GetSelectedFormInMagicMenu` + item creation.
fn from_magic_menu(ui: &UI) -> Option<BoxedItem> {
    let menu = ui.get_menu(MagicMenu::MENU_NAME);
    let menu = unsafe { menu.as_ptr().as_ref() }?;
    let movie = unsafe { menu.ui_movie.as_ptr().as_ref() }?;
    let mut result = GFxValue::new();
    let path = c"_root.Menu_mc.inventoryLists.itemList.selectedEntry.formId";
    if !movie.get_variable(&mut result, path.as_ptr().cast()) || !result.is_number() {
        return None;
    }
    let form = unsafe { TESForm::lookup_by_id(result.get_number() as u32)?.as_ref() }?;
    match form.get_form_type() {
        FormType::Spell | FormType::Shout => from_bound_object(form, 0),
        _ => None,
    }
}

/// C++ `WheelItemFactory::MakeWheelItemFromMenuHovered`
pub fn from_menu_hovered() -> Option<BoxedItem> {
    if !equip::player_3d_loaded() {
        return None;
    }
    let ui = unsafe { UI::get_singleton().as_ref() }?;
    if ui.is_menu_open(InventoryMenu::MENU_NAME) {
        from_inventory_menu(ui)
    } else if ui.is_menu_open(MagicMenu::MENU_NAME) {
        from_magic_menu(ui)
    } else {
        None
    }
}

/// C++ `WheelItemFactory::MakeWheelItemFromJsonObject`. `resolve` maps the
/// saved form ID to the current load order (`ResolveFormID`).
pub fn from_save_data(data: &ItemData, resolve: impl Fn(u32) -> Option<u32>) -> Option<BoxedItem> {
    let form_id = resolve(data.form_id)?;
    let form = unsafe { TESForm::lookup_by_id(form_id)?.as_ref() }?;
    let item: BoxedItem = match data.item_type.as_str() {
        super::weapon::ITEM_TYPE_STR => {
            Box::new(WheelItemWeapon::new(form_as(form)?, data.unique_id?))
        }
        super::armor::ITEM_TYPE_STR => {
            Box::new(WheelItemArmor::new(form_as(form)?, data.unique_id?))
        }
        super::spell::ITEM_TYPE_STR => Box::new(WheelItemSpell::new(form_as(form)?)),
        super::shout::ITEM_TYPE_STR => Box::new(WheelItemShout::new(form_as(form)?)),
        super::light::ITEM_TYPE_STR => Box::new(WheelItemLight::new(form_as(form)?)?),
        super::ammo::ITEM_TYPE_STR => Box::new(WheelItemAmmo::new(form_as(form)?)),
        super::alchemy::ITEM_TYPE_STR => Box::new(WheelItemAlchemy::new(form_as(form)?)),
        super::scroll::ITEM_TYPE_STR => Box::new(WheelItemScroll::new(form_as(form)?)),
        super::misc::ITEM_TYPE_STR => Box::new(WheelItemMisc::new(form_as(form)?)),
        other => {
            skse_error!("Unknown wheel item type in save: {other}");
            return None;
        }
    };
    Some(item)
}
