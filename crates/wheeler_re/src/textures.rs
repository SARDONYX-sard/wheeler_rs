//! Port of `Rendering/TextureManager.cpp`: loads the icon SVGs into D3D11
//! textures once the renderer exists.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::RwLock;

use libskyrim::re::{BGSKeywordForm, TESDataHandler, TESForm};
use libskyrim::relocation::skyrim_cast_const;
use libskyrim::rex::ID3D11Device;
use libskyrim::{skse_error, skse_message};
use wheeler_core::custom_icon::{CustomIcon, parse_custom_icon_name};
use wheeler_dx11::texture;
use wheeler_imgui::image::{IconImageType, Image};

/// C++ `Texture::icon_directory`
const ICON_DIR: &str = "Data\\SKSE\\Plugins\\Wheeler\\resources\\icons";
/// C++ `Texture::icon_custom_directory`
const ICON_CUSTOM_DIR: &str = "Data\\SKSE\\Plugins\\Wheeler\\resources\\icons_custom";

#[derive(Default)]
struct Icons {
    builtin: HashMap<IconImageType, Image>,
    /// C++ `icon_struct_formID`
    by_form: HashMap<u32, Image>,
    /// C++ `icon_struct_keyword` (a `std::map`: first match in key order wins).
    by_keyword: BTreeMap<String, Image>,
}

static ICONS: RwLock<Option<Icons>> = RwLock::new(None);

unsafe fn load(device: *mut ID3D11Device, path: &Path) -> Option<Image> {
    let loaded = texture::rasterize_svg_file(path)
        .and_then(|rgba| unsafe { texture::upload(device.cast(), &rgba) });
    match loaded {
        Ok(t) => Some(Image {
            texture: t.srv,
            width: t.width,
            height: t.height,
        }),
        Err(e) => {
            skse_error!("Failed to load icon: {e}");
            None
        }
    }
}

/// C++ `Texture::load_custom_icon_images`
unsafe fn load_custom(device: *mut ID3D11Device, icons: &mut Icons) {
    let Ok(dir) = std::fs::read_dir(ICON_CUSTOM_DIR) else {
        return; // optional directory
    };
    let data_handler = unsafe { TESDataHandler::get_singleton(false).as_ref() };
    for path in dir.filter_map(Result::ok).map(|e| e.path()) {
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        match parse_custom_icon_name(name) {
            Some(CustomIcon::Form {
                plugin,
                local_form_id,
            }) => {
                let form = data_handler.map_or(core::ptr::null_mut(), |dh| {
                    dh.lookup_form(local_form_id, &plugin)
                });
                let Some(form) = (unsafe { form.as_ref() }) else {
                    continue; // plugin not loaded
                };
                if let Some(image) = unsafe { load(device, &path) } {
                    icons.by_form.insert(form.get_form_id(), image);
                }
            }
            Some(CustomIcon::Keyword(keyword)) => {
                if let Some(image) = unsafe { load(device, &path) } {
                    icons.by_keyword.insert(keyword, image);
                }
            }
            None => {}
        }
    }
}

/// C++ `Texture::Init`
///
/// # Safety
/// `device` must be the game's live D3D11 device.
pub unsafe fn init(device: *mut ID3D11Device) {
    let mut icons = Icons::default();
    for &icon in IconImageType::ALL {
        if let Some(image) = unsafe { load(device, &Path::new(ICON_DIR).join(icon.file_name())) } {
            icons.builtin.insert(icon, image);
        }
    }
    unsafe { load_custom(device, &mut icons) };
    skse_message!(
        "Loaded {} icons, {} form icons, {} keyword icons",
        icons.builtin.len(),
        icons.by_form.len(),
        icons.by_keyword.len()
    );
    *ICONS.write().unwrap_or_else(|e| e.into_inner()) = Some(icons);
}

/// C++ `Texture::GetIconImage`: a custom icon for `form` (by form ID, then
/// keyword) wins over the built-in `icon`.
pub fn icon_for(icon: IconImageType, form: Option<&TESForm>) -> Image {
    let guard = ICONS.read().unwrap_or_else(|e| e.into_inner());
    let Some(icons) = guard.as_ref() else {
        return Image::NONE;
    };
    if let Some(form) = form {
        if let Some(&image) = icons.by_form.get(&form.get_form_id()) {
            return image;
        }
        let keywords = unsafe { skyrim_cast_const::<TESForm, BGSKeywordForm>(form).as_ref() };
        if let Some(keywords) = keywords
            && let Some((_, &image)) = icons
                .by_keyword
                .iter()
                .find(|(k, _)| keywords.has_keyword_string(k))
        {
            return image;
        }
    }
    icons.builtin.get(&icon).copied().unwrap_or(Image::NONE)
}

/// Built-in icon only (`GetIconImage(type, nullptr)`).
pub fn icon(icon: IconImageType) -> Image {
    icon_for(icon, None)
}
