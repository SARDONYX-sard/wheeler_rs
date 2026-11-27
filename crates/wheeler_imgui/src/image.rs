//! `Texture::Image` and `Texture::icon_image_type` from `TextureManager.h`.
//! Loading (SVG rasterisation + GPU upload) is the renderer's job.

/// A GPU texture usable as an ImGui texture id, with its native size.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Image {
    /// `ID3D11ShaderResourceView*` (or any backend texture id); `0` = none.
    pub texture: usize,
    pub width: i32,
    pub height: i32,
}

impl Image {
    pub const NONE: Self = Self {
        texture: 0,
        width: 0,
        height: 0,
    };

    pub fn texture_ptr(&self) -> *mut core::ffi::c_void {
        self.texture as *mut core::ffi::c_void
    }

    /// `[width * scale, height * scale]`
    pub fn scaled(&self, scale: f32) -> [f32; 2] {
        [self.width as f32 * scale, self.height as f32 * scale]
    }
}

macro_rules! icon_types {
    ($($variant:ident),* $(,)?) => {
        /// C++ `Texture::icon_image_type`; the file is `<snake_name>.svg`.
        #[allow(non_camel_case_types)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum IconImageType {
            $($variant),*
        }

        impl IconImageType {
            pub const ALL: &'static [Self] = &[$(Self::$variant),*];

            /// File name in `resources/icons` (C++ `icon_type_name_map`).
            pub const fn file_name(self) -> &'static str {
                match self {
                    $(Self::$variant => concat!(stringify!($variant), ".svg")),*
                }
            }
        }
    };
}

icon_types!(
    potion_health,
    potion_default,
    sword_one_handed,
    axe_one_handed,
    mace,
    dagger,
    sword_two_handed,
    axe_two_handed,
    warhammer_two_handed,
    staff,
    bow,
    crossbow,
    spell_default,
    destruction,
    shout,
    power,
    food,
    shield,
    icon_default,
    destruction_fire,
    destruction_frost,
    destruction_shock,
    restoration,
    poison_default,
    armor_heavy_shield,
    armor_light_shield,
    armor_light_chest,
    armor_heavy_chest,
    armor_light_arm,
    armor_heavy_arm,
    armor_light_foot,
    armor_heavy_foot,
    armor_light_head,
    armor_heavy_head,
    armor_clothing_head,
    armor_clothing_chest,
    armor_clothing_foot,
    armor_clothing_arm,
    armor_necklace,
    armor_circlet,
    armor_ring,
    armor_default,
    scroll,
    arrow,
    hand_to_hand,
    potion_stamina,
    potion_magicka,
    potion_fire_resist,
    potion_shock_resist,
    potion_frost_resist,
    potion_magic_resist,
    alteration,
    conjuration,
    illusion,
    torch,
    lantern,
    mask,
    armor_rating,
    weapon_damage,
    slot_background,
    slot_highlighted_background,
    slot_active_background,
    wheel_background,
    wheel_indicator_active,
    wheel_indicator_inactive,
);
