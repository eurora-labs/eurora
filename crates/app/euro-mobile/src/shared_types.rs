use euro_settings::SettingsState;
use tokio::sync::Mutex;

pub use euro_thread::commands::{ActiveStreamTokens, SharedThreadManager};

pub type SharedSettingsState = Mutex<SettingsState>;
