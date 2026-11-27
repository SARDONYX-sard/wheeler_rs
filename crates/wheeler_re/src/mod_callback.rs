//! Port of `ModCallbackEventHandler.cpp`: reacts to dMenu (in-game settings
//! menu) callbacks.

use libskyrim::sdk::events::skse::dispatchers;
use libskyrim::skse::ModCallbackEvent;
use libskyrim::{skse_error, skse_message};

fn on_event(event: &ModCallbackEvent) {
    let arg = event.str_arg.as_str();
    match event.event_name.as_str() {
        "dmenu_updateSettings" if arg == "Wheeler Styles" || arg == "Wheeler Controls" => {
            crate::reload_settings();
        }
        "dmenu_buttonCallback" if arg == "wheeler_reset_all_wheels" => {
            crate::wheeler::setup_default_wheels();
        }
        _ => {}
    }
}

/// C++ `ModCallbackEventHandler::Register`
pub fn register() {
    match dispatchers::subscribe::<ModCallbackEvent, _, _>(|e| {
        if let Some(e) = e {
            on_event(e);
        }
    }) {
        Ok(subscription) => {
            // Stays registered for the whole process.
            core::mem::forget(subscription);
            skse_message!("Registered ModCallbackEvent sink");
        }
        Err(e) => skse_error!("Failed to register ModCallbackEvent sink: {e:?}"),
    }
}
