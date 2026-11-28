//! Game implementation of the `wheeler_imgui` wheel callbacks.

use wheeler_core::sound::Sound;
use wheeler_imgui::image::{IconImageType, Image};
use wheeler_imgui::wheel::{WheelHost, WheelItem};

pub use crate::items::ItemCtx;

pub type Wheel = wheeler_imgui::wheel::Wheel<ItemCtx>;
pub type WheelEntry = wheeler_imgui::wheel::WheelEntry<ItemCtx>;

pub struct GameHost;

impl WheelHost<ItemCtx> for GameHost {
    fn play_sound(&self, sound: Sound) {
        libskyrim::re::play_sound(sound.editor_id().as_ptr());
    }

    fn icon(&self, icon: IconImageType) -> Image {
        crate::textures::icon(icon)
    }

    fn item_from_menu_hovered(&self) -> Option<Box<dyn WheelItem<ItemCtx>>> {
        crate::items::factory::from_menu_hovered()
    }
}
