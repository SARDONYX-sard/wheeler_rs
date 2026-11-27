//! Port of `Texts.cpp`: user-facing strings overridable from `Texts.ini`.

use std::sync::RwLock;

use crate::ini::IniSource;

pub const TEXTS_PATH: &str = "Data\\SKSE\\Plugins\\wheeler\\Texts.ini";

/// C++ `Texts::TextType`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextType {
    AlchemyDynamicIDConsumptionWarning,
    NoWheelPresent,
}

impl TextType {
    const ALL: [Self; 2] = [
        Self::AlchemyDynamicIDConsumptionWarning,
        Self::NoWheelPresent,
    ];

    /// The INI key, which is also the default text (as in C++).
    const fn key(self) -> &'static str {
        match self {
            Self::AlchemyDynamicIDConsumptionWarning => "AlchemyDynamicIDConsumptionWarning",
            Self::NoWheelPresent => "NoWheelPresent",
        }
    }
}

static TEXTS: RwLock<Vec<String>> = RwLock::new(Vec::new());

/// C++ `Texts::LoadTranslations`: `[Texts]` of [`TEXTS_PATH`]; missing keys
/// fall back to the key name.
pub fn load_translations(ini: Option<&dyn IniSource>) {
    let texts = TextType::ALL
        .iter()
        .map(|t| {
            ini.and_then(|ini| ini.value("Texts", t.key()).map(str::to_owned))
                .unwrap_or_else(|| t.key().to_owned())
        })
        .collect();
    *TEXTS.write().unwrap_or_else(|e| e.into_inner()) = texts;
}

/// C++ `Texts::GetText`
pub fn get_text(text: TextType) -> String {
    let texts = TEXTS.read().unwrap_or_else(|e| e.into_inner());
    let index = TextType::ALL.iter().position(|t| *t == text).unwrap_or(0);
    texts
        .get(index)
        .cloned()
        .unwrap_or_else(|| text.key().to_owned())
}

/// C++ `stripMagicItemDescriptionFormatCode`: removes `<...>` tags (font,
/// colour) from a game description. An unterminated `<` keeps the rest.
pub fn strip_markup(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(lhs) = rest.find('<') {
        let Some(len) = rest[lhs..].find('>') else {
            break;
        };
        out.push_str(&rest[..lhs]);
        rest = &rest[lhs + len + 1..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_font_tags() {
        let s = "Restore <font color='#FFFFFF'>25</font> points of Health.";
        assert_eq!(strip_markup(s), "Restore 25 points of Health.");
        assert_eq!(strip_markup("a < b"), "a < b");
        assert_eq!(strip_markup(""), "");
    }
}
