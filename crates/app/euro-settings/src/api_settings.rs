use serde::{Deserialize, Serialize};
use specta::Type;

/// Canonical Eurora cloud backend.
///
/// Baked at compile time from `EURORA_CLOUD_API_URL` (sourced from
/// the workspace `.env` by `build.rs`). Override the value at build
/// time to ship a fork pointing at a different organisation's
/// infrastructure — there is no in-source default.
pub const CLOUD_API_URL: &str = env!("EURORA_CLOUD_API_URL");

/// Canonical local-development backend served by
/// `cargo run -p be-monolith`. Baked at compile time from
/// `EURORA_LOCAL_API_URL` (workspace `.env`); override at build
/// time if your local backend listens elsewhere.
pub const LOCAL_API_URL: &str = env!("EURORA_LOCAL_API_URL");

/// Where the desktop app should send authenticated requests.
///
/// `Cloud` and `Local` carry no parameters because their URLs are baked in,
/// which keeps "I want to talk to the production server" expressible as a
/// stable enum value rather than a magic string. `Custom` covers everything
/// else (self-hosted homelab, a colleague's tunnel, etc.).
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ConnectionMode {
    Cloud,
    Local,
    Custom { url: String },
}

impl ConnectionMode {
    /// Resolve the mode to a concrete URL string.
    pub fn endpoint(&self) -> &str {
        match self {
            ConnectionMode::Cloud => CLOUD_API_URL,
            ConnectionMode::Local => LOCAL_API_URL,
            ConnectionMode::Custom { url } => url.as_str(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Type)]
#[serde(rename_all = "camelCase")]
pub struct APISettings {
    pub mode: ConnectionMode,
}

impl APISettings {
    /// Resolve the active connection mode to a URL string. Convenience
    /// wrapper around [`ConnectionMode::endpoint`] so callers don't have to
    /// reach into the enum directly.
    pub fn endpoint(&self) -> &str {
        self.mode.endpoint()
    }
}

impl Default for APISettings {
    /// Debug builds default to talking to a backend on `localhost:3000` —
    /// the `just dev` flow stands one up there. Release builds default to
    /// the public Eurora cloud.
    fn default() -> Self {
        let mode = if cfg!(debug_assertions) {
            ConnectionMode::Local
        } else {
            ConnectionMode::Cloud
        };
        Self { mode }
    }
}
