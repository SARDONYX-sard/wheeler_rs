//! Rust port of [Wheeler](https://github.com/D7ry/wheeler) on top of libskyrim.
//!
//! Module map (C++ original in parentheses):
//! - [`hooks`] (`Hooks.cpp`): inventory and input-dispatch hooks
//! - [`render`] (`Rendering/RenderManager.cpp`): D3D11 init/present hooks and the ImGui backend
//! - [`input`] / [`controls`] (`UserInput/*`): input filtering and key bindings
//! - [`unique_id`] (`Utilities/UniqueIDHandler.cpp`)
//! - [`wheeler`] (`Wheeler/Wheeler.cpp`)
//! - [`serialization`] (`Serialization/SerializationEntry.cpp`)
//! - [`mod_callback`] (`ModCallbackEventHandler.cpp`)
//! - [`items`] (`Wheeler/WheelItems/*`)
//! - [`settings`] (`Config.cpp` file loading; the values live in `wheeler_core`)
//! - [`textures`] (`Rendering/TextureManager.cpp`)
//! - [`host`]: game callbacks for `wheeler_imgui::wheel`
//!
//! Game-independent logic is in `wheeler_core` (settings, geometry,
//! animation), `wheeler_imgui` (wheel drawing) and `wheeler_dx11` (renderer).

pub mod controls;
pub mod hooks;
pub mod host;
pub mod input;
pub mod items;
pub mod mod_callback;
pub mod render;
pub mod serialization;
pub mod settings;
pub mod textures;
pub mod unique_id;
pub mod wheeler;

use libskyrim::{
    sdk::plugin::{self, LoadInterface, LogLevel},
    skse::{RuntimeCompatibility, StructCompatibility, VersionIndependence},
    skse_message,
    version::Version,
};

pub const PLUGIN_NAME: &str = "wheeler_re";

libskyrim::plugin_declaration! {
    version: Version::new(0, 1, 0, 0),
    name: "wheeler_re",
    author: "SARDONYX",
    support_email: "",
    struct_compatibility: StructCompatibility::Independent,
    runtime_compatibility: RuntimeCompatibility::from_version_independence(
        VersionIndependence::AddressLibrary,
        false,
    ),
    minimum_skse_version: Version::new(0, 0, 0, 0),
}

/// Trampoline bytes: 4 `write_call<5>` hooks x 14-byte stubs, rounded up.
const TRAMPOLINE_SIZE: usize = 1 << 6;

/// # Safety
/// Called once by the `plugin_declaration!` load stub with SKSE's interface.
#[allow(clippy::result_unit_err)] // signature fixed by the load stub
#[unsafe(no_mangle)]
pub unsafe fn skse_plugin_rust_entry(skse: &LoadInterface) -> Result<(), ()> {
    plugin::init(skse);
    plugin::set_level(LogLevel::Debug);
    skse_message!("{PLUGIN_NAME} v{}", env!("CARGO_PKG_VERSION"));

    plugin::alloc_trampoline(TRAMPOLINE_SIZE);

    // C++ `onSKSEInit`
    render::install();
    wheeler::init();
    hooks::install();
    serialization::install();

    register_messages();
    Ok(())
}

/// Reads `Styles.ini` / `Controls.ini`, scales them to the viewport and
/// rebinds the keys (data loaded, and dMenu's "update settings").
pub fn reload_settings() {
    settings::read_style_config();
    settings::read_control_config();
    match render::viewport_height() {
        Some(height) => wheeler_core::style::offset_sizing_to_viewport(height),
        None => {
            libskyrim::skse_error!("Renderer not initialized; sizes are not scaled to the viewport")
        }
    }
    controls::bind_all_inputs_from_config();
}

/// C++ `MessageHandler`
fn register_messages() {
    plugin::on_data_loaded(|_| {
        items::mutable::register();
        reload_settings();
        render::on_data_loaded(); // Texture::Init
        settings::load_translations();
        mod_callback::register();
    });

    plugin::on_new_game(|_| {
        wheeler::setup_default_wheels();
        unique_id::ensure_xlist_uniqueness_in_pc_inventory();
    });

    plugin::on_post_load_game(|_| {
        unique_id::ensure_xlist_uniqueness_in_pc_inventory();
    });
}
