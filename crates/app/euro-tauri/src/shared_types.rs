use std::sync::Arc;

use euro_endpoint::EndpointManager;
use euro_settings::AppSettings;
use tokio::sync::Mutex;

pub use euro_thread::commands::{ActiveStreamTokens, SharedThreadManager};

pub type SharedAppSettings = Mutex<AppSettings>;
pub type SharedEndpointManager = Arc<EndpointManager>;
pub type SharedUserController = Mutex<euro_user::UserController>;

/// Process-wide HTTP client used by every backend-touching procedure
/// (`payment_*`, `system_test_backend_url`, `system_get_llm_info`, …).
/// `reqwest::Client` is internally an `Arc` over its connection pool, so
/// cloning the state out of `tauri::State` is free — do that rather than
/// constructing a fresh `Client` per call, which would defeat connection
/// reuse and re-build TLS state every time.
pub type SharedHttpClient = reqwest::Client;
