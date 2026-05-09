use euro_settings::AppSettings;
use tokio::sync::Mutex;

pub use euro_thread::commands::{ActiveStreamTokens, SharedThreadManager};

pub type SharedUserController = Mutex<euro_user::UserController>;
pub type SharedAppSettings = Mutex<AppSettings>;
