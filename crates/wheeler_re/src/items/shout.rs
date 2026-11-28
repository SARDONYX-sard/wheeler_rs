//! Port of `WheelItemShout`.

use libskyrim::re::TESShout;
use wheeler_core::save::ItemData;
use wheeler_imgui::drawer::DrawArgs;
use wheeler_imgui::image::{IconImageType, Image};
use wheeler_imgui::wheel::{self, WheelItem};

use super::equip;
use super::{Form, ItemCtx, description, icon};

pub const ITEM_TYPE_STR: &str = "WheelItemShout";

pub struct WheelItemShout {
    shout: Form<TESShout>,
    texture: Image,
    description: String,
}

impl WheelItemShout {
    pub fn new(shout: Form<TESShout>) -> Self {
        Self {
            shout,
            texture: icon(IconImageType::shout, shout),
            description: description((*shout).as_ref()),
        }
    }

    pub fn form_id(&self) -> u32 {
        self.shout.form_id()
    }

    fn selected(&self) -> bool {
        equip::player().is_some_and(|pc| {
            unsafe { pc.get_actor_runtime_data().selected_power.as_ref() }
                .is_some_and(|p| p.get_form_id() == self.form_id())
        })
    }

    /// C++ `ActivateItemPrimary` / `Secondary` (identical).
    // TODO (C++): block equipping shouts that are not unlocked.
    fn toggle(&self) {
        let (Some(pc), Some(aem)) = (equip::player_actor(), equip::equip_manager()) else {
            return;
        };
        if self.selected() {
            aem.unequip_shout(pc, self.shout.ptr());
        } else {
            aem.equip_shout(pc, self.shout.ptr());
        }
    }
}

impl WheelItem<ItemCtx> for WheelItemShout {
    fn save_data(&self) -> Option<ItemData> {
        Some(ItemData::new(ITEM_TYPE_STR, self.form_id(), None))
    }

    fn draw_slot(&self, center: [f32; 2], _hovered: bool, _ctx: &ItemCtx, args: DrawArgs) {
        wheel::draw_slot_text(center, &self.shout.name(), args);
        wheel::draw_slot_texture(self.texture, center, args);
    }

    fn draw_highlight(&self, center: [f32; 2], _ctx: &ItemCtx, args: DrawArgs) {
        wheel::draw_highlight_text(center, &self.shout.name(), args);
        wheel::draw_highlight_texture(self.texture, center, args);
        if !self.description.is_empty() {
            wheel::draw_highlight_description(center, &self.description, args);
        }
    }

    fn is_active(&self, _ctx: &ItemCtx) -> bool {
        self.selected()
    }

    fn is_available(&self, _ctx: &ItemCtx) -> bool {
        true
    }

    fn activate_primary(&mut self, _ctx: &ItemCtx) {
        self.toggle();
    }

    fn activate_secondary(&mut self, _ctx: &ItemCtx) {
        self.toggle();
    }
}
