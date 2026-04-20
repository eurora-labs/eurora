use std::collections::HashMap;

use euro_settings::AppSettings;
use euro_thread::ThreadManager;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

pub type SharedThreadManager = Mutex<ThreadManager>;
pub type SharedUserController = Mutex<euro_user::UserController>;
pub type SharedAppSettings = Mutex<AppSettings>;
pub type ActiveStreamTokens = Mutex<HashMap<String, CancellationToken>>;
