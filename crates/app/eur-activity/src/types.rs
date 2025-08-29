//! Core type definitions for the refactored activity system
//!
//! This module contains the enum-based replacements for the previous trait object system,
//! providing better performance, type safety, and cloneable activities.

use crate::assets::{ArticleAsset, DefaultAsset, TwitterAsset, YoutubeAsset};
use crate::snapshots::*;
use crate::storage::{AssetStorage, SaveableAsset, SavedAssetInfo};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use enum_dispatch::enum_dispatch;
use ferrous_llm_core::Message;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Context chip for UI integration
#[taurpc::ipc_type]
pub struct ContextChip {
    pub id: String,
    pub extension_id: String,
    pub name: String,
    pub attrs: HashMap<String, String>,
    pub icon: Option<String>,
    pub position: Option<u32>,
}

/// Display asset for UI rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayAsset {
    pub name: String,
    pub icon: String,
}

impl DisplayAsset {
    pub fn new(name: String, icon: String) -> Self {
        Self { name, icon }
    }
}

/// Enum containing all possible activity assets
#[enum_dispatch(SaveableAsset, CommonFunctionality, SaveFunctionality)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivityAsset {
    YoutubeAsset,
    ArticleAsset,
    TwitterAsset,
    DefaultAsset,
}

#[enum_dispatch]
pub trait CommonFunctionality {
    fn get_name(&self) -> &str;
    fn get_icon(&self) -> Option<&str>;
    fn construct_message(&self) -> Message;
    fn get_context_chip(&self) -> Option<ContextChip>;
}

#[async_trait]
#[enum_dispatch]
pub trait SaveFunctionality {
    async fn save_to_disk(&self, storage: &AssetStorage) -> crate::error::Result<SavedAssetInfo>;
}

/// Enum containing all possible activity snapshots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivitySnapshot {
    Youtube(YoutubeSnapshot),
    Article(ArticleSnapshot),
    Twitter(TwitterSnapshot),
    Default(DefaultSnapshot),
}

impl ActivitySnapshot {
    /// Construct a message for LLM interaction
    pub fn construct_message(&self) -> Message {
        match self {
            ActivitySnapshot::Youtube(snapshot) => snapshot.construct_message(),
            ActivitySnapshot::Article(snapshot) => snapshot.construct_message(),
            ActivitySnapshot::Twitter(snapshot) => snapshot.construct_message(),
            ActivitySnapshot::Default(snapshot) => snapshot.construct_message(),
        }
    }

    /// Get the timestamp when this snapshot was last updated
    pub fn get_updated_at(&self) -> u64 {
        match self {
            ActivitySnapshot::Youtube(snapshot) => snapshot.updated_at,
            ActivitySnapshot::Article(snapshot) => snapshot.updated_at,
            ActivitySnapshot::Twitter(snapshot) => snapshot.updated_at,
            ActivitySnapshot::Default(snapshot) => snapshot.updated_at,
        }
    }

    /// Get the timestamp when this snapshot was created
    pub fn get_created_at(&self) -> u64 {
        match self {
            ActivitySnapshot::Youtube(snapshot) => snapshot.created_at,
            ActivitySnapshot::Article(snapshot) => snapshot.created_at,
            ActivitySnapshot::Twitter(snapshot) => snapshot.created_at,
            ActivitySnapshot::Default(snapshot) => snapshot.created_at,
        }
    }
}

/// Main activity structure - now fully cloneable and serializable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    /// Name of the activity
    pub name: String,
    /// Icon representing the activity
    pub icon: String,
    /// Process name of the activity
    pub process_name: String,
    /// Start time
    pub start: DateTime<Utc>,
    /// End time
    pub end: Option<DateTime<Utc>>,
    /// Assets associated with the activity
    pub assets: Vec<ActivityAsset>,
    /// Snapshots of the activity
    pub snapshots: Vec<ActivitySnapshot>,
}

impl Activity {
    /// Create a new activity
    pub fn new(
        name: String,
        icon: String,
        process_name: String,
        assets: Vec<ActivityAsset>,
    ) -> Self {
        Self {
            name,
            icon,
            process_name,
            start: Utc::now(),
            end: None,
            assets,
            snapshots: Vec::new(),
        }
    }

    /// Get display assets for UI rendering
    pub fn get_display_assets(&self) -> Vec<DisplayAsset> {
        self.assets
            .iter()
            .map(|asset| {
                if let Some(icon) = asset.get_icon() {
                    DisplayAsset::new(asset.get_name().to_string(), icon.to_string())
                } else {
                    DisplayAsset::new(asset.get_name().to_string(), self.icon.clone())
                }
            })
            .collect()
    }

    /// Get context chips for UI integration
    pub fn get_context_chips(&self) -> Vec<ContextChip> {
        self.assets
            .iter()
            .filter_map(|asset| asset.get_context_chip())
            .collect()
    }

    /// Add an asset to the activity
    pub fn add_asset(&mut self, asset: ActivityAsset) {
        self.assets.push(asset);
    }

    /// Add a snapshot to the activity
    pub fn add_snapshot(&mut self, snapshot: ActivitySnapshot) {
        self.snapshots.push(snapshot);
    }

    /// Mark the activity as ended
    pub fn end_activity(&mut self) {
        self.end = Some(Utc::now());
    }

    /// Save all assets in this activity to disk
    pub async fn save_assets_to_disk(
        &self,
        storage: &AssetStorage,
    ) -> crate::error::Result<Vec<SavedAssetInfo>> {
        let mut saved_assets = Vec::new();

        for asset in &self.assets {
            let saved_info = asset.save_to_disk(storage).await?;
            saved_assets.push(saved_info);
        }

        Ok(saved_assets)
    }

    /// Save a specific asset by index to disk
    pub async fn save_asset_by_index(
        &self,
        index: usize,
        storage: &AssetStorage,
    ) -> crate::error::Result<Option<SavedAssetInfo>> {
        if let Some(asset) = self.assets.get(index) {
            Ok(Some(asset.save_to_disk(storage).await?))
        } else {
            Ok(None)
        }
    }
}
