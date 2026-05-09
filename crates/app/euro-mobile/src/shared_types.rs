use std::collections::HashMap;
use std::sync::Arc;

use euro_settings::AppSettings;
use euro_thread::ThreadManager;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// `ThreadManager` is `Clone` and stateless across calls; share via `Arc`
/// so concurrent procedures don't serialize on a mutex they don't need.
pub type SharedThreadManager = Arc<ThreadManager>;
pub type SharedUserController = Mutex<euro_user::UserController>;
pub type SharedAppSettings = Mutex<AppSettings>;
pub type ActiveStreamTokens = Mutex<HashMap<Uuid, CancellationToken>>;
