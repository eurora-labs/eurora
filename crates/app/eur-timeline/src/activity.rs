//! Activity module for the timeline
//!
//! This module defines the Activity, ActivitySnapshot, and ActivityAsset types
//! that are used to organize system state over time.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Represents a session of the application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSession {}

/// Types of activities
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityType {
    Article,
    Application,
    Browser,
    Document,
    Video,
    Custom,
}

/// Types of assets
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetType {
    Youtube,
    Article,
    Pdf,
    Screenshot,
    Custom,
}

/// Represents an activity in the timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    /// Name of the activity
    pub name: String,

    /// Icon representing the activity
    pub icon: String,

    /// Type of activity
    #[serde(rename = "type")]
    pub activity_type: ActivityType,

    /// Start time (Unix timestamp)
    pub start: DateTime<Utc>,

    /// End time (Unix timestamp)
    pub end: Option<DateTime<Utc>>,

    /// Snapshots of the activity
    pub snapshots: Vec<ActivitySnapshot>,

    /// Assets associated with the activity
    pub assets: Vec<ActivityAsset>,
}

impl Activity {
    /// Create a new activity
    pub fn new(name: String, icon: String, activity_type: ActivityType) -> Self {
        let now = chrono::Utc::now();

        Self {
            name,
            icon,
            activity_type,
            start: now,
            end: None, // Will be set when the activity ends
            snapshots: Vec::new(),
            assets: Vec::new(),
        }
    }

    /// Add a snapshot to the activity
    pub fn add_snapshot(&mut self, snapshot: ActivitySnapshot) {
        self.snapshots.push(snapshot);
    }

    /// Add an asset to the activity
    pub fn add_asset(&mut self, asset: ActivityAsset) {
        self.assets.push(asset);
    }
}

/// Represents a snapshot of an activity at a specific point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivitySnapshot {
    /// The application session
    #[serde(skip_serializing)]
    pub session: AppSession,

    /// Screenshot data
    pub screenshot: Option<Vec<u8>>,

    /// When this snapshot was last updated
    pub updated_at: u64,

    /// When this snapshot was created
    pub created_at: u64,
}

impl ActivitySnapshot {
    /// Get assets associated with this snapshot
    pub fn get_assets(&self) -> Vec<ActivityAsset> {
        // In a real implementation, this would retrieve assets
        // associated with this specific snapshot
        Vec::new()
    }
}

/// Represents an asset associated with an activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityAsset {
    /// The data for this asset
    pub data: JsonValue,

    /// Type of asset
    #[serde(rename = "type")]
    pub asset_type: AssetType,

    /// When this asset was last updated
    pub updated_at: u64,

    /// When this asset was created
    pub created_at: u64,
}
