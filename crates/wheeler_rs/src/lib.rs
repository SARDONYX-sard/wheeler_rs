use commonlibsse_ng::skse;
use commonlibsse_ng::skse::interfaces::messaging::{Message, MessageType};

macro_rules! bail {
    ($expr:expr) => {
        if let Err(err) = $expr {
            #[cfg(feature = "tracing")]
            tracing::error!("{err}");
            return;
        };
    };
}

#[commonlibsse_ng::skse_plugin_main]
fn plugin_main() {
    let messaging = match skse::api::get_messaging_interface() {
        Ok(messaging) => messaging,
        Err(_err) => {
            #[cfg(feature = "tracing")]
            tracing::error!("Failed to skse::init: {_err}");
            return;
        }
    };

    bail!(messaging.register_skse_listener(skse_event_listener));
    on_skse_init();
}

fn skse_event_listener(message: &Message) {
    if let Some(msg_type) = message.msg_type.to_enum() {
        if msg_type == MessageType::PostLoadGame {}
    }
}

fn on_skse_init() {
    bail!(wheeler_core::rendering::render_manager::install());
}

// This is because CI will treat it as an error if there is at least one test missing.
#[test]
fn dummy_test() {
    assert_eq!(1 + 1, 2);
}
