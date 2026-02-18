mod error;
mod provider;

pub use error::{Error, Result};
pub use proto_gen::local_settings as proto;
pub use provider::*;

pub type SettingsSender = tokio::sync::watch::Sender<Option<ProviderSettings>>;
pub type SettingsReceiver = tokio::sync::watch::Receiver<Option<ProviderSettings>>;

/// Create a settings channel.
///
/// Returns a `(Sender, Receiver)` pair. The settings service owns the sender
/// and calls `send()` when provider configuration changes. Other services
/// (e.g. conversation) hold a receiver and are notified via `changed()`.
pub fn settings_channel() -> (SettingsSender, SettingsReceiver) {
    tokio::sync::watch::channel(None)
}
