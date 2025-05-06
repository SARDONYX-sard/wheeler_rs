mod render_manager;

use core::ptr::NonNull;
use std::sync::{LazyLock, OnceLock};

use commonlibsse_ng::{
    re::{BSCoreTypes::FormID, TESForm::TESForm},
    rel::relocation::RelocationError,
};
use commonlibsse_ng_re_ext::re::BSRenderManager::{BSRenderManager, RUNTIME_DATA};
use dashmap::DashMap;
use snafu::ResultExt as _;
use windows::Win32::Graphics::{
    Direct3D::D3D11_SRV_DIMENSION_TEXTURE2D,
    Direct3D11::{
        D3D11_BIND_SHADER_RESOURCE, D3D11_SHADER_RESOURCE_VIEW_DESC,
        D3D11_SHADER_RESOURCE_VIEW_DESC_0, D3D11_SUBRESOURCE_DATA, D3D11_TEX2D_SRV,
        D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, ID3D11Device, ID3D11ShaderResourceView,
    },
    Dxgi::Common::{DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SAMPLE_DESC},
};

static DEVICE: OnceLock<ID3D11Device> = OnceLock::new();

#[derive(Debug, snafu::Snafu)]
enum TextureError {
    /// Not found `BSRenderManager. AddressLibrary error.`
    NotFoundRenderManager,
    /// Not found runtime fields of `BSRenderManager`. AddressLibrary error.: {source}`
    FailedToGetRuntimeData { source: RelocationError },
}

struct Texture {}

impl Texture {
    pub fn new() {}

    /// Return width & height
    pub fn load_texture_from_file() -> Result<Image, TextureError> {
        let render_manager =
            BSRenderManager::get_singleton().ok_or(TextureError::NotFoundRenderManager)?;

        let RUNTIME_DATA { forwarder, .. } = render_manager
            .get_runtime_data()
            .with_context(|_| FailedToGetRuntimeDataSnafu)?;

        // TODOs
        let image_data = &[0_u8; 0];
        let image_width = 0;
        let image_height = 0;

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
        unsafe { device.CreateTexture2D(&desc, Some(&sub_resource), Some(&mut p_texture)) };

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
                forwarder.CreateShaderResourceView(&p_texture, Some(&srv_desc), Some(&mut out_srv));
            }
        }

        Ok(Image {
            texture: out_srv,
            width: image_width as i32,   // FIXME: valid cast?
            height: image_height as i32, // FIXME: valid cast?
        })
    }
}

struct Image {
    texture: Option<ID3D11ShaderResourceView>,
    width: i32,
    height: i32,
}

static ICON_STRUCT: LazyLock<DashMap<u32, Image>> = LazyLock::new(DashMap::new);
static ICON_STRUCT_FORM_ID: LazyLock<DashMap<FormID, Image>> = LazyLock::new(DashMap::new);
static ICON_STRUCT_KEYWORD: LazyLock<DashMap<String, Image>> = LazyLock::new(DashMap::new);

const ICON_DIRECTORY: &str = "Data/SKSE/Plugins/Wheeler/resources/icons";
const IMG_DIRECTORY: &str = "Data/SKSE/Plugins/Wheeler/resources/img";
const ICON_CUSTOM_DIRECTORY: &str = "Data/SKSE/Plugins/Wheeler/resources/icons_custom";

fn get_icon_image<'a>(
    image_type: IconImageType,
    form: Option<NonNull<TESForm>>,
) -> Option<&'a Image> {
    let img = form.and_then(|form| {
        if let Some(pair) = ICON_STRUCT_FORM_ID.get(unsafe { &form.as_ref().formID }) {
            return Some(pair);
        }
        None
    });

    if let Some(pair) = ICON_STRUCT.get(&(image_type as u32)) {
        // return Some(pair);
    }

    None
}

enum ImageType {
    Hud,
    Round,
    Key,
    Total,
}

enum IconImageType {
    PotionHealth,
    PotionDefault,
    SwordOneHanded,
    AxeOneHanded,
    Mace,
    Dagger,
    SwordTwoHanded,
    AxeTwoHanded,
    WarHammerTwoHanded,
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
