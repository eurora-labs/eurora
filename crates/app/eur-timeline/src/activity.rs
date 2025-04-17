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

    /// Process name of the activity
    pub process_name: String,

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
    pub fn new(name: String, icon: String, process_name: String) -> Self {
        let now = chrono::Utc::now();

        // Create an AssetContext and set the strategy based on the process name
        let mut asset_context = crate::asset_strategy::AssetContext::new();
        asset_context.set_strategy_by_process_name(&process_name);

        // Try to retrieve assets using the strategy
        let mut assets = Vec::new();
        match asset_context.retrieve_assets() {
            Ok(asset) => {
                assets.push(asset);
            }
            Err(e) => {
                // Log the error but continue with empty assets
                eprintln!("Failed to retrieve assets: {}", e);
            }
        }

        Self {
            name,
            icon,
            process_name,
            start: now,
            end: None, // Will be set when the activity ends
            snapshots: Vec::new(),
            assets,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activity_with_asset_strategy() {
        // Create a new activity with browser process name
        let activity = Activity::new(
            "Test Activity".to_string(),
            "test-icon".to_string(),
            "browser".to_string(),
        );

        // Verify that assets were retrieved
        assert!(!activity.assets.is_empty());

        // Check the first asset
        let asset = &activity.assets[0];
        assert_eq!(asset.asset_type, AssetType::Custom);

        // Verify that the asset data contains expected fields
        if let JsonValue::Object(map) = &asset.data {
            assert!(map.contains_key("url"));
            assert!(map.contains_key("title"));
            assert!(map.contains_key("content"));
        } else {
            panic!("Asset data is not a JSON object");
        }
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

impl ActivityAsset {
    /// Create a new activity asset
    pub fn new(data: JsonValue, asset_type: AssetType) -> Self {
        let now = chrono::Utc::now().timestamp() as u64;

        Self {
            data,
            asset_type,
            updated_at: now,
            created_at: now,
        }
    }
}
