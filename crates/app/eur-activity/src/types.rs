//! Core type definitions for the refactored activity system
//!
//! This module contains the enum-based replacements for the previous trait object system,
//! providing better performance, type safety, and cloneable activities.

use crate::assets::*;
use crate::snapshots::*;
use crate::storage::{AssetStorage, SaveableAsset, SavedAssetInfo};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivityAsset {
    Youtube(YoutubeAsset),
    Article(ArticleAsset),
    Twitter(TwitterAsset),
    Default(DefaultAsset),
}

impl ActivityAsset {
    /// Get the name of the asset
    pub fn get_name(&self) -> &str {
        match self {
            ActivityAsset::Youtube(asset) => &asset.title,
            ActivityAsset::Article(asset) => &asset.title,
            ActivityAsset::Twitter(asset) => &asset.title,
            ActivityAsset::Default(asset) => &asset.name,
        }
    }

    /// Get the icon of the asset
    pub fn get_icon(&self) -> Option<&str> {
        match self {
            ActivityAsset::Youtube(_) => Some("youtube-icon"),
            ActivityAsset::Article(_) => Some("article-icon"),
            ActivityAsset::Twitter(_) => Some("twitter-icon"),
            ActivityAsset::Default(asset) => asset.icon.as_deref(),
        }
    }

    /// Construct a message for LLM interaction
    pub fn construct_message(&self) -> Message {
        match self {
            ActivityAsset::Youtube(asset) => asset.construct_message(),
            ActivityAsset::Article(asset) => asset.construct_message(),
            ActivityAsset::Twitter(asset) => asset.construct_message(),
            ActivityAsset::Default(asset) => asset.construct_message(),
        }
    }

    /// Get context chip for UI integration
    pub fn get_context_chip(&self) -> Option<ContextChip> {
        match self {
            ActivityAsset::Youtube(asset) => asset.get_context_chip(),
            ActivityAsset::Article(asset) => asset.get_context_chip(),
            ActivityAsset::Twitter(asset) => asset.get_context_chip(),
            ActivityAsset::Default(_) => None,
        }
    }

    /// Save this asset to disk using the provided storage
    pub async fn save_to_disk(
        &self,
        storage: &AssetStorage,
    ) -> crate::error::Result<SavedAssetInfo> {
        match self {
            ActivityAsset::Youtube(asset) => storage.save_asset(asset).await,
            ActivityAsset::Article(asset) => storage.save_asset(asset).await,
            ActivityAsset::Twitter(asset) => storage.save_asset(asset).await,
            ActivityAsset::Default(asset) => storage.save_asset(asset).await,
        }
    }
}

#[async_trait]
impl SaveableAsset for ActivityAsset {
    fn get_asset_type(&self) -> &'static str {
        match self {
            ActivityAsset::Youtube(asset) => asset.get_asset_type(),
            ActivityAsset::Article(asset) => asset.get_asset_type(),
            ActivityAsset::Twitter(asset) => asset.get_asset_type(),
            ActivityAsset::Default(asset) => asset.get_asset_type(),
        }
    }

    fn get_file_extension(&self) -> &'static str {
        match self {
            ActivityAsset::Youtube(asset) => asset.get_file_extension(),
            ActivityAsset::Article(asset) => asset.get_file_extension(),
            ActivityAsset::Twitter(asset) => asset.get_file_extension(),
            ActivityAsset::Default(asset) => asset.get_file_extension(),
        }
    }

    fn get_mime_type(&self) -> &'static str {
        match self {
            ActivityAsset::Youtube(asset) => asset.get_mime_type(),
            ActivityAsset::Article(asset) => asset.get_mime_type(),
            ActivityAsset::Twitter(asset) => asset.get_mime_type(),
            ActivityAsset::Default(asset) => asset.get_mime_type(),
        }
    }

    async fn serialize_content(&self) -> crate::error::Result<Vec<u8>> {
        match self {
            ActivityAsset::Youtube(asset) => asset.serialize_content().await,
            ActivityAsset::Article(asset) => asset.serialize_content().await,
            ActivityAsset::Twitter(asset) => asset.serialize_content().await,
            ActivityAsset::Default(asset) => asset.serialize_content().await,
        }
    }

    fn get_unique_id(&self) -> String {
        match self {
            ActivityAsset::Youtube(asset) => asset.get_unique_id(),
            ActivityAsset::Article(asset) => asset.get_unique_id(),
            ActivityAsset::Twitter(asset) => asset.get_unique_id(),
            ActivityAsset::Default(asset) => asset.get_unique_id(),
        }
    }

    fn get_display_name(&self) -> String {
        match self {
            ActivityAsset::Youtube(asset) => asset.get_display_name(),
            ActivityAsset::Article(asset) => asset.get_display_name(),
            ActivityAsset::Twitter(asset) => asset.get_display_name(),
            ActivityAsset::Default(asset) => asset.get_display_name(),
        }
    }
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
