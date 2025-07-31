//! Data types and structures for the update service

use serde::{Deserialize, Serialize};

/// Tauri updater response format (dynamic server)
#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateResponse {
    pub version: String,
    pub pub_date: String,
    pub url: String,
    pub signature: String,
    pub notes: String,
}

/// Path parameters for the update endpoint
#[derive(Deserialize, Debug)]
pub struct UpdateParams {
    pub channel: String,     // "nightly" or "release"
    pub target_arch: String, // e.g., "linux-x86_64", "darwin-aarch64"
    pub current_version: String,
}
