use serde::{Deserialize, Serialize};
use specta::Type;

/// The backend URL the binary was compiled against.
///
/// Baked at compile time from `BACKEND_URL` (sourced from the workspace
/// `.env` by `build.rs`). For dev builds that's typically
/// `http://localhost:3000`; for release binaries it's whatever the
/// shipping organisation set in their CI. Override the value at build
/// time to ship a fork pointing at a different organisation's
/// infrastructure — there is no in-source default.
pub const DEFAULT_API_URL: &str = env!("BACKEND_URL");

/// Where the desktop app should send authenticated requests.
///
/// `Default` carries no parameters because its URL is baked in, which
/// keeps "I want to talk to the URL this binary was built against"
/// expressible as a stable enum value rather than a magic string.
/// `Custom` covers everything else (self-hosted homelab, a colleague's
/// tunnel, a different organisation's infrastructure).
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ConnectionMode {
    Default,
    Custom { url: String },
}

impl ConnectionMode {
    /// Resolve the mode to a concrete URL string.
    pub fn endpoint(&self) -> &str {
        match self {
            ConnectionMode::Default => DEFAULT_API_URL,
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
    fn default() -> Self {
        Self {
            mode: ConnectionMode::Default,
        }
    }
}
