use std::sync::Arc;

use euro_endpoint::EndpointManager;
use euro_settings::AppSettings;
use euro_thread::ThreadManager;
use tokio::sync::Mutex;

pub type SharedAppSettings = Mutex<AppSettings>;
pub type SharedThreadManager = Mutex<ThreadManager>;
pub type SharedEndpointManager = Arc<EndpointManager>;
pub type SharedUserController = Mutex<euro_user::Controller>;
