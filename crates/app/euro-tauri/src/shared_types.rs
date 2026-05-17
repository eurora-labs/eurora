use std::sync::Arc;

use euro_endpoint::EndpointManager;
use euro_settings::SettingsState;
use tokio::sync::Mutex;

pub use euro_thread::commands::{ActiveStreamTokens, SharedThreadManager};

/// Shared owner of the on-disk settings split. Held in an `Arc` so the
/// sync engine and the Tauri IPC handlers can lock the *same* mutex —
/// the engine needs to replace `cache` after a pull or a 409 reconcile,
/// and the IPC handlers need to see the result without re-reading from
/// disk.
pub type SharedSettingsState = Arc<Mutex<SettingsState>>;
pub type SharedEndpointManager = Arc<EndpointManager>;

/// Process-wide HTTP client used by every backend-touching procedure
/// (`payment_*`, `system_test_backend_url`, `system_get_llm_info`, …).
/// `reqwest::Client` is internally an `Arc` over its connection pool, so
/// cloning the state out of `tauri::State` is free — do that rather than
/// constructing a fresh `Client` per call, which would defeat connection
/// reuse and re-build TLS state every time.
pub type SharedHttpClient = reqwest::Client;
