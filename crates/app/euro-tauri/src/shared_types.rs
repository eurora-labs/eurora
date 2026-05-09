use std::collections::HashMap;
use std::sync::Arc;

use euro_endpoint::EndpointManager;
use euro_settings::AppSettings;
use euro_thread::ThreadManager;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub type SharedAppSettings = Mutex<AppSettings>;
/// `ThreadManager` is `Clone` and stateless across calls, so we share it via
/// a plain `Arc` instead of wrapping it in a mutex — handlers concurrently
/// hit the HTTP API and the WebSocket without any contention on this state.
pub type SharedThreadManager = Arc<ThreadManager>;
pub type SharedEndpointManager = Arc<EndpointManager>;
pub type SharedUserController = Mutex<euro_user::UserController>;
pub type ActiveStreamTokens = Mutex<HashMap<Uuid, CancellationToken>>;

/// Process-wide HTTP client used by every backend-touching procedure
/// (`payment_*`, `system_test_backend_url`, `system_get_llm_info`, …).
/// `reqwest::Client` is internally an `Arc` over its connection pool, so
/// cloning the state out of `tauri::State` is free — do that rather than
/// constructing a fresh `Client` per call, which would defeat connection
/// reuse and re-build TLS state every time.
pub type SharedHttpClient = reqwest::Client;
