use core::{ptr::NonNull, str::FromStr};
use std::{
    path::{Path, PathBuf},
    sync::LazyLock,
};

use commonlibsse_ng::re::TESForm::TESForm;
use commonlibsse_ng::{re::BSCoreTypes::FormID, rel::relocation::RelocationError};
use commonlibsse_ng_re_ext::re::{
    BSRenderManager::{BSRenderManager, RUNTIME_DATA},
    TESDataHandler::TESDataHandler,
};
use dashmap::DashMap;
use snafu::ResultExt as _;
use windows::Win32::Graphics::{
    Direct3D::D3D11_SRV_DIMENSION_TEXTURE2D,
    Direct3D11::{
        D3D11_BIND_SHADER_RESOURCE, D3D11_SHADER_RESOURCE_VIEW_DESC,
        D3D11_SHADER_RESOURCE_VIEW_DESC_0, D3D11_SUBRESOURCE_DATA, D3D11_TEX2D_SRV,
        D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, ID3D11ShaderResourceView,
    },
    Dxgi::Common::{DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SAMPLE_DESC},
};

use super::render_manager::DEVICE;

const ICON_DIRECTORY: &str = "Data/SKSE/Plugins/Wheeler/resources/icons";
const ICON_CUSTOM_DIRECTORY: &str = "Data/SKSE/Plugins/Wheeler/resources/icons_custom";

#[derive(Debug, snafu::Snafu)]
enum TextureError {
    /// Not found `BSRenderManager. AddressLibrary error.`
    NotFoundRenderManager,
    /// Not found runtime fields of `BSRenderManager`. AddressLibrary error.: {source}`
    FailedToGetRuntimeData { source: RelocationError },

    /// Not found svg. path: {path:?}, error: {source}
    NotFoundSvg {
        path: PathBuf,
        source: std::io::Error,
    },

    /// Failed to parse svg. path: {path:?}, error: {source}
    FailedToParseSvg { path: PathBuf, source: usvg::Error },

    /// Failed to create 2D Texture. path: {path:?}, error: {source}
    FailedToCreate2DTexture {
        path: PathBuf,
        source: windows::core::Error,
    },

    /// Failed to create shader resource view. path: {path:?}, error: {source}
    FailedToCreateShaderResourceView {
        path: PathBuf,
        source: windows::core::Error,
    },
}

pub struct Texture;

impl Texture {
    pub fn init() {
        Self::load_custom_icon_images();
        Self::load_images(ICON_DIRECTORY);
    }

    /// Return width & height
    fn load_texture_from_file(file_name: impl AsRef<Path>) -> Result<Image, TextureError> {
        let render_manager =
            BSRenderManager::get_singleton().ok_or(TextureError::NotFoundRenderManager)?;

        let RUNTIME_DATA { forwarder, .. } = render_manager
            .get_runtime_data()
            .with_context(|_| FailedToGetRuntimeDataSnafu)?;

        let image_data = std::fs::read(file_name.as_ref()).with_context(|_| NotFoundSvgSnafu {
            path: file_name.as_ref().to_path_buf(),
        })?;
        // https://github.com/linebender/resvg/blob/main/crates/usvg/tests/parser.rs#L193
        let image_size = usvg::Tree::from_data(&image_data, &usvg::Options::default())
            .with_context(|_| FailedToParseSvgSnafu {
                path: file_name.as_ref().to_path_buf(),
            })?
            .size();
        let image_width = image_size.width() as u32;
        let image_height = image_size.height() as u32;

        let desc = D3D11_TEXTURE2D_DESC {
            Width: image_width,
            Height: image_height,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
            CPUAccessFlags: 0,
            MiscFlags: 0,
        };

        let sub_resource = D3D11_SUBRESOURCE_DATA {
            pSysMem: image_data.as_ptr().cast(),
            SysMemPitch: desc.Width * 4,
            SysMemSlicePitch: 0,
        };

        let mut p_texture = None;
        let device = DEVICE.get().ok_or(TextureError::NotFoundRenderManager)?;
        unsafe { device.CreateTexture2D(&desc, Some(&sub_resource), Some(&mut p_texture)) }
            .with_context(|_| FailedToCreate2DTextureSnafu {
                path: file_name.as_ref().to_path_buf(),
            })?;

        let srv_desc = D3D11_SHADER_RESOURCE_VIEW_DESC {
            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
            ViewDimension: D3D11_SRV_DIMENSION_TEXTURE2D,
            Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                Texture2D: D3D11_TEX2D_SRV {
                    MostDetailedMip: desc.MipLevels,
                    MipLevels: 0,
                },
            },
        };

        let mut out_srv = None;
        unsafe {
            let forwarder = forwarder
                .as_ref()
                .ok_or(TextureError::NotFoundRenderManager)?;

            if let Some(p_texture) = p_texture {
                forwarder
                    .CreateShaderResourceView(&p_texture, Some(&srv_desc), Some(&mut out_srv))
                    .with_context(|_| FailedToCreateShaderResourceViewSnafu {
                        path: file_name.as_ref().to_path_buf(),
                    })?;
            }
        }

        Ok(Image {
            texture: out_srv,
            width: image_width as i32,   // FIXME: valid cast?
            height: image_height as i32, // FIXME: valid cast?
        })
    }

    fn load_images(file_path: impl AsRef<Path>) {
        let walk_dir = jwalk::WalkDir::new(file_path)
            .into_iter()
            .filter_map(Result::ok);

        for entry in walk_dir {
            let path = entry.path();
            let is_svg = path
                .extension()
                .map(|e| e.eq_ignore_ascii_case("svg"))
                .unwrap_or_default();
            if !is_svg {
                continue;
            }

            let index = match IconImageType::from_str(&path.to_string_lossy()) {
                Ok(index) => index,
                Err(err) => {
                    tracing::error!("{err}");
                    continue;
                }
            };

            match Self::load_texture_from_file(&path) {
                Ok(image) => ICON_STRUCT.insert(index, image),
                Err(err) => {
                    tracing::error!("{err}");
                    continue;
                }
            };
        }
    }

    fn load_custom_icon_images() {
        let Some(handler) = TESDataHandler::get_singleton() else {
            tracing::error!("Failed to get `TESDataHandler`");
            return;
        };

        for entry in jwalk::WalkDir::new(ICON_CUSTOM_DIRECTORY)
            .into_iter()
            .filter_map(Result::ok)
        {
            let path = entry.path();

            if !is_svg_file(&path) {
                continue;
            }

            let file_name = path.file_name().unwrap().to_string_lossy();
            let image = match Self::load_texture_from_file(path.as_path()) {
                Ok(img) => img,
                Err(err) => {
                    tracing::error!("{err}");
                    continue;
                }
            };

            if let Some((plugin, form_id)) = parse_form_id_from_filename(&file_name) {
                if let Some(form) = handler
                    .lookup_form(FormID::new(form_id), plugin)
                    .map(|form| unsafe { form.as_ref() })
                {
                    ICON_STRUCT_FORM_ID.insert(form.formID, image);
                }
            } else if let Some(keyword) = parse_keyword_from_filename(&file_name) {
                ICON_STRUCT_KEYWORD.insert(keyword, image);
            }
        }
    }
}

fn is_svg_file(path: &std::path::Path) -> bool {
    path.extension()
        .map(|ext| ext.eq_ignore_ascii_case("svg"))
        .unwrap_or(false)
}

fn parse_form_id_from_filename(file_name: &str) -> Option<(&str, u32)> {
    let rest = file_name.strip_prefix("FID_")?;
    let (plugin, hex_id) = rest.split_once("_0x").or_else(|| rest.split_once("_0X"))?;
    let form_id = u32::from_str_radix(hex_id.trim_end_matches(".svg"), 16).ok()?;
    Some((plugin, form_id))
}

fn parse_keyword_from_filename(file_name: &str) -> Option<String> {
    file_name
        .strip_prefix("KWD_")
        .map(|kw| kw.trim_end_matches(".svg").to_string())
}

static ICON_STRUCT: LazyLock<DashMap<IconImageType, Image>> = LazyLock::new(DashMap::new);
static ICON_STRUCT_FORM_ID: LazyLock<DashMap<FormID, Image>> = LazyLock::new(DashMap::new);
static ICON_STRUCT_KEYWORD: LazyLock<DashMap<String, Image>> = LazyLock::new(DashMap::new);

pub struct Image {
    pub texture: Option<ID3D11ShaderResourceView>,
    pub width: i32,
    pub height: i32,
}

pub fn get_icon_image<'a>(
    image_type: IconImageType,
    form: Option<NonNull<TESForm>>,
) -> Option<&'a Image> {
    let img = form.and_then(|form| {
        if let Some(pair) = ICON_STRUCT_FORM_ID.get(unsafe { &form.as_ref().formID }) {
            return Some(pair);
        }
        None
    });

    if let Some(pair) = ICON_STRUCT.get(&image_type) {
        // return Some(pair);
    }

    None
}

#[derive(Debug)]
enum ImageType {
    Hud,
    Round,
    Key,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IconImageType {
    PotionHealth,
    PotionDefault,
    SwordOneHanded,
    AxeOneHanded,
    Mace,
    Dagger,
    SwordTwoHanded,
    AxeTwoHanded,
    WarhammerTwoHanded,
    Staff,
    Bow,
    Crossbow,
    SpellDefault,
    Destruction,
    Shout,
    Power,
    Food,
    Shield,
    IconDefault,
    DestructionFire,
    DestructionFrost,
    DestructionShock,
    Restoration,
    PoisonDefault,
    ArmorHeavyShield,
    ArmorLightShield,
    ArmorLightChest,
    ArmorHeavyChest,
    ArmorLightArm,
    ArmorHeavyArm,
    ArmorLightFoot,
    ArmorHeavyFoot,
    ArmorLightHead,
    ArmorHeavyHead,
    ArmorClothingHead,
    ArmorClothingChest,
    ArmorClothingFoot,
    ArmorClothingArm,
    ArmorNecklace,
    ArmorCirclet,
    ArmorRing,
    ArmorDefault,
    Scroll,
    Arrow,
    HandToHand,
    PotionStamina,
    PotionMagicka,
    PotionFireResist,
    PotionShockResist,
    PotionFrostResist,
    PotionMagicResist,
    Alteration,
    Conjuration,
    Illusion,
    Torch,
    Lantern,
    Mask,
    ArmorRating,
    WeaponDamage,
    SlotBackground,
    SlotHighlightedBackground,
    SlotActiveBackground,
    WheelBackground,
    WheelIndicatorActive,
    WheelIndicatorInactive,
}

impl core::str::FromStr for IconImageType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "potion_health.svg" => Self::PotionHealth,
            "potion_default.svg" => Self::PotionDefault,
            "sword_one_handed.svg" => Self::SwordOneHanded,
            "axe_one_handed.svg" => Self::AxeOneHanded,
            "mace.svg" => Self::Mace,
            "dagger.svg" => Self::Dagger,
            "sword_two_handed.svg" => Self::SwordTwoHanded,
            "axe_two_handed.svg" => Self::AxeTwoHanded,
            "warhammer_two_handed.svg" => Self::WarhammerTwoHanded,
            "staff.svg" => Self::Staff,
            "bow.svg" => Self::Bow,
            "crossbow.svg" => Self::Crossbow,
            "spell_default.svg" => Self::SpellDefault,
            "destruction.svg" => Self::Destruction,
            "shout.svg" => Self::Shout,
            "power.svg" => Self::Power,
            "food.svg" => Self::Food,
            "shield.svg" => Self::Shield,
            "icon_default.svg" => Self::IconDefault,
            "destruction_fire.svg" => Self::DestructionFire,
            "destruction_frost.svg" => Self::DestructionFrost,
            "destruction_shock.svg" => Self::DestructionShock,
            "restoration.svg" => Self::Restoration,
            "poison_default.svg" => Self::PoisonDefault,
            "armor_light_chest.svg" => Self::ArmorLightChest,
            "armor_heavy_chest.svg" => Self::ArmorHeavyChest,
            "armor_light_arm.svg" => Self::ArmorLightArm,
            "armor_heavy_arm.svg" => Self::ArmorHeavyArm,
            "armor_light_foot.svg" => Self::ArmorLightFoot,
            "armor_heavy_foot.svg" => Self::ArmorHeavyFoot,
            "armor_light_head.svg" => Self::ArmorLightHead,
            "armor_heavy_head.svg" => Self::ArmorHeavyHead,
            "armor_heavy_shield.svg" => Self::ArmorHeavyShield,
            "armor_light_shield.svg" => Self::ArmorLightShield,
            "armor_clothing_head.svg" => Self::ArmorClothingHead,
            "armor_clothing_chest.svg" => Self::ArmorClothingChest,
            "armor_clothing_arm.svg" => Self::ArmorClothingArm,
            "armor_clothing_foot.svg" => Self::ArmorClothingFoot,
            "armor_circlet.svg" => Self::ArmorCirclet,
            "armor_necklace.svg" => Self::ArmorNecklace,
            "armor_ring.svg" => Self::ArmorRing,
            "armor_default.svg" => Self::ArmorDefault,
            "scroll.svg" => Self::Scroll,
            "arrow.svg" => Self::Arrow,
            "hand_to_hand.svg" => Self::HandToHand,
            "potion_stamina.svg" => Self::PotionStamina,
            "potion_magicka.svg" => Self::PotionMagicka,
            "potion_fire_resist.svg" => Self::PotionFireResist,
            "potion_shock_resist.svg" => Self::PotionShockResist,
            "potion_frost_resist.svg" => Self::PotionFrostResist,
            "potion_magic_resist.svg" => Self::PotionMagicResist,
            "alteration.svg" => Self::Alteration,
            "conjuration.svg" => Self::Conjuration,
            "illusion.svg" => Self::Illusion,
            "torch.svg" => Self::Torch,
            "lantern.svg" => Self::Lantern,
            "mask.svg" => Self::Mask,
            "armor_rating.svg" => Self::ArmorRating,
            "weapon_damage.svg" => Self::WeaponDamage,
            "slot_background.svg" => Self::SlotBackground,
            "slot_highlighted_background.svg" => Self::SlotHighlightedBackground,
            "slot_active_background.svg" => Self::SlotActiveBackground,
            "wheel_background.svg" => Self::WheelBackground,
            "wheel_indicator_active.svg" => Self::WheelIndicatorActive,
            "wheel_indicator_inactive.svg" => Self::WheelIndicatorInactive,
            unknown => {
                return Err(format!(
                    "There is no variant corresponding to that svg filename: {unknown}."
                ));
            }
        })
    }
}
