mod error;
mod provider;

pub use error::{Error, Result};
pub use proto_gen::local_settings as proto;
pub use provider::*;

pub type SettingsSender = tokio::sync::watch::Sender<Option<ProviderSettings>>;
pub type SettingsReceiver = tokio::sync::watch::Receiver<Option<ProviderSettings>>;

pub fn settings_channel() -> (SettingsSender, SettingsReceiver) {
    tokio::sync::watch::channel(None)
}
