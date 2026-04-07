use std::collections::HashMap;
use std::sync::Arc;

use euro_endpoint::EndpointManager;
use euro_settings::AppSettings;
use euro_thread::ThreadManager;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

pub type SharedAppSettings = Mutex<AppSettings>;
pub type SharedThreadManager = Mutex<ThreadManager>;
pub type SharedEndpointManager = Arc<EndpointManager>;
pub type SharedUserController = Mutex<euro_user::Controller>;
pub type ActiveStreamTokens = Mutex<HashMap<String, CancellationToken>>;
