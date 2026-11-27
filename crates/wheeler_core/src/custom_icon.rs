//! File-name rules of `resources/icons_custom` (C++
//! `Texture::load_custom_icon_images`):
//!
//! - `FID_<plugin>_0x<local form id>.svg` overrides the icon of one form;
//! - `KWD_<keyword editor id>.svg` overrides the icon of forms with a keyword.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CustomIcon {
    Form { plugin: String, local_form_id: u32 },
    Keyword(String),
}

/// Parses a custom icon file name; `None` for anything else.
pub fn parse_custom_icon_name(file_name: &str) -> Option<CustomIcon> {
    let stem = file_name.strip_suffix(".svg")?;
    if let Some(rest) = stem.strip_prefix("FID_") {
        // The plugin name ends at the first `_0x` / `_0X`.
        let end = rest.find("_0x").or_else(|| rest.find("_0X"))?;
        let (plugin, id) = (&rest[..end], &rest[end + 3..]);
        let local_form_id = u32::from_str_radix(id, 16).ok()?;
        (!plugin.is_empty()).then(|| CustomIcon::Form {
            plugin: plugin.to_owned(),
            local_form_id,
        })
    } else {
        let keyword = stem.strip_prefix("KWD_")?;
        (!keyword.is_empty()).then(|| CustomIcon::Keyword(keyword.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn parses_names() {
        assert_eq!(
            parse_custom_icon_name("FID_Skyrim.esm_0x12EB7.svg"),
            Some(CustomIcon::Form {
                plugin: "Skyrim.esm".into(),
                local_form_id: 0x12EB7
            })
        );
        assert_eq!(
            parse_custom_icon_name("FID_My_Mod.esp_0X800.svg"),
            Some(CustomIcon::Form {
                plugin: "My_Mod.esp".into(),
                local_form_id: 0x800
            })
        );
        assert_eq!(
            parse_custom_icon_name("KWD_WeapTypeSword.svg"),
            Some(CustomIcon::Keyword("WeapTypeSword".into()))
        );
        assert_eq!(parse_custom_icon_name("FID_Skyrim.esm_0xZZ.svg"), None);
        assert_eq!(parse_custom_icon_name("FID_Skyrim.esm.svg"), None);
        assert_eq!(parse_custom_icon_name("KWD_.svg"), None);
        assert_eq!(parse_custom_icon_name("sword.svg"), None);
        assert_eq!(parse_custom_icon_name("KWD_Foo.png"), None);
    }
}
