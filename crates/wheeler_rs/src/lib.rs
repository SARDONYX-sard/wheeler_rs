use commonlibsse_ng::skse;
use commonlibsse_ng::skse::interfaces::messaging::{Message, MessageType};

#[commonlibsse_ng::skse_plugin_main]
fn plugin_main() {
    match skse::api::get_messaging_interface() {
        Ok(messaging) => {
            if let Err(err) = messaging.register_skse_listener(skse_event_listener) {
                #[cfg(feature = "tracing")]
                tracing::error!("{err}");
            };
        }
        Err(err) => {
            #[cfg(feature = "tracing")]
            tracing::error!("Failed to skse::init: {err}");
        }
    }
}

fn skse_event_listener(message: &Message) {
    if let Some(msg_type) = message.msg_type.to_enum() {
        if msg_type == MessageType::PostLoadGame {}
    }
}

// This is because CI will treat it as an error if there is at least one test missing.
#[test]
fn dummy_test() {
    assert_eq!(1 + 1, 2);
}
