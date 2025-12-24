//! Data types and structures for the update service

use std::collections::HashMap;

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

/// Path parameters for the release info endpoint
#[derive(Deserialize, Debug)]
pub struct ReleaseParams {
    pub channel: String, // "nightly", "release", or "beta"
}

/// Platform-specific download information
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlatformInfo {
    pub url: String,
    pub signature: String,
}

/// Response for the releases/{channel} endpoint
/// Contains the latest version info with all available platforms
#[derive(Serialize, Deserialize, Debug)]
pub struct ReleaseInfoResponse {
    pub version: String,
    pub pub_date: String,
    /// Map of platform identifiers (e.g., "windows-x86_64", "linux-x86_64") to their download info
    pub platforms: HashMap<String, PlatformInfo>,
}
