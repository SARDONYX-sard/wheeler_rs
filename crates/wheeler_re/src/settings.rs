//! Game-side loading of Wheeler's INI files into the `wheeler_core` settings
//! (C++ `Config::Read*`, `Texts::LoadTranslations`).

use libskyrim::sdk::plugin::{self, config::Ini};
use libskyrim::skse_message;
use wheeler_core::ini::IniSource;
use wheeler_core::{control, style, texts};

/// Adapts libskyrim's [`Ini`] to `wheeler_core`'s [`IniSource`].
pub struct GameIni(pub Ini);

impl IniSource for GameIni {
    fn value(&self, section: &str, key: &str) -> Option<&str> {
        self.0.get(section, key)
    }
}

/// Loads `path`, logging (not failing) when it is missing.
pub fn load(path: &str) -> Option<GameIni> {
    match plugin::load_ini(path) {
        Ok(ini) => Some(GameIni(ini)),
        Err(()) => {
            skse_message!("{path} not found, using defaults");
            None
        }
    }
}

/// C++ `Config::ReadControlConfig`
pub fn read_control_config() {
    if let Some(ini) = load(control::CONTROL_SETTINGS_PATH) {
        control::read_control_config(&ini);
    }
}

/// C++ `Config::ReadStyleConfig`
pub fn read_style_config() {
    if let Some(ini) = load(style::STYLE_SETTINGS_PATH) {
        style::read_style_config(&ini);
    }
}

/// C++ `Texts::LoadTranslations`
pub fn load_translations() {
    let ini = load(texts::TEXTS_PATH);
    texts::load_translations(ini.as_ref().map(|i| i as &dyn IniSource));
}
