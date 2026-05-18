//! HTTP error envelope returned by the thread service on non-2xx responses.

use serde::{Deserialize, Serialize};

#[cfg(feature = "specta")]
use specta::Type;

/// JSON error body returned by the thread service on non-2xx responses.
///
/// Mirrors the shape used by [`activity-core`](https://docs.rs/activity-core)
/// so the desktop client can decode failures uniformly across HTTP services.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct ThreadErrorResponse {
    pub error: String,
    pub message: String,
    #[serde(default)]
    pub details: Option<String>,
}
