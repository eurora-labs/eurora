use std::collections::HashMap;
use std::sync::Arc;

use agent_chain_core::BaseMessage;
use chrono::{DateTime, Utc};
use enum_dispatch::enum_dispatch;
use euro_native_messaging::NativeMessage;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    assets::{ArticleAsset, DefaultAsset, TwitterAsset, YoutubeAsset},
    error::ActivityResult,
    snapshots::*,
    storage::SaveableAsset,
};

#[taurpc::ipc_type]
pub struct ContextChip {
    pub id: String,
    pub extension_id: String,
    pub name: String,
    pub attrs: HashMap<String, String>,
    pub icon: Option<String>,
    pub position: Option<u32>,
}

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

#[enum_dispatch(SaveableAsset, AssetFunctionality)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivityAsset {
    YoutubeAsset,
    ArticleAsset,
    TwitterAsset,
    DefaultAsset,
}

impl TryFrom<NativeMessage> for ActivityAsset {
    type Error = anyhow::Error;

    fn try_from(value: NativeMessage) -> Result<Self, Self::Error> {
        match value {
            NativeMessage::NativeYoutubeAsset(asset) => {
                Ok(ActivityAsset::YoutubeAsset(YoutubeAsset::from(asset)))
            }
            NativeMessage::NativeArticleAsset(asset) => {
                Ok(ActivityAsset::ArticleAsset(ArticleAsset::from(asset)))
            }
            NativeMessage::NativeTwitterAsset(asset) => {
                Ok(ActivityAsset::TwitterAsset(TwitterAsset::from(asset)))
            }
            _ => Err(anyhow::anyhow!("Invalid asset type")),
        }
    }
}

// impl From<NativeMessage> for ActivityAsset {
//     fn from(asset: NativeMessage) -> Self {
//         match asset {
//             NativeMessage::NativeYoutubeAsset(asset) => {
//                 ActivityAsset::YoutubeAsset(YoutubeAsset::from(asset))
//             }
//             NativeMessage::NativeArticleAsset(asset) => {
//                 ActivityAsset::ArticleAsset(ArticleAsset::from(asset))
//             }
//             NativeMessage::NativeTwitterAsset(asset) => {
//                 ActivityAsset::TwitterAsset(TwitterAsset::from(asset))
//             }
//         }
//     }
// }

#[enum_dispatch]
pub trait AssetFunctionality {
    fn get_id(&self) -> &str;
    fn get_name(&self) -> &str;
    fn get_icon(&self) -> Option<&str>;
    fn construct_messages(&self) -> Vec<BaseMessage>;
    fn get_context_chip(&self) -> Option<ContextChip>;
}

#[enum_dispatch]
pub trait SnapshotFunctionality {
    fn get_id(&self) -> &str;
    fn construct_messages(&self) -> Vec<BaseMessage>;
    fn get_updated_at(&self) -> u64;
    fn get_created_at(&self) -> u64;
}

#[enum_dispatch(SnapshotFunctionality)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivitySnapshot {
    YoutubeSnapshot,
    ArticleSnapshot,
    TwitterSnapshot,
    DefaultSnapshot,
}

impl TryFrom<NativeMessage> for ActivitySnapshot {
    type Error = anyhow::Error;

    fn try_from(value: NativeMessage) -> Result<Self, Self::Error> {
        match value {
            NativeMessage::NativeYoutubeSnapshot(snapshot) => Ok(
                ActivitySnapshot::YoutubeSnapshot(YoutubeSnapshot::from(snapshot)),
            ),
            NativeMessage::NativeArticleSnapshot(snapshot) => Ok(
                ActivitySnapshot::ArticleSnapshot(ArticleSnapshot::from(snapshot)),
            ),
            NativeMessage::NativeTwitterSnapshot(snapshot) => Ok(
                ActivitySnapshot::TwitterSnapshot(TwitterSnapshot::from(snapshot)),
            ),
            _ => Err(anyhow::anyhow!("Invalid snapshot type")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    pub id: String,
    pub name: String,
    #[serde(skip)]
    pub icon: Option<Arc<image::RgbaImage>>,
    pub process_name: String,
    pub start: DateTime<Utc>,
    pub end: Option<DateTime<Utc>>,
    pub assets: Vec<ActivityAsset>,
    pub snapshots: Vec<ActivitySnapshot>,
}

impl Activity {
    pub fn new(
        name: String,
        icon: Option<Arc<image::RgbaImage>>,
        process_name: String,
        assets: Vec<ActivityAsset>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            icon,
            process_name,
            start: Utc::now(),
            end: None,
            assets,
            snapshots: Vec::new(),
        }
    }

    pub fn get_display_assets(&self) -> Vec<DisplayAsset> {
        self.assets
            .iter()
            .map(|asset| {
                if let Some(icon) = asset.get_icon() {
                    DisplayAsset::new(asset.get_name().to_string(), icon.to_string())
                } else {
                    DisplayAsset::new(asset.get_name().to_string(), "".to_string())
                }
            })
            .collect()
    }

    pub fn get_context_chips(&self) -> Vec<ContextChip> {
        self.assets
            .iter()
            .filter_map(|asset| asset.get_context_chip())
            .collect()
    }

    pub fn add_asset(&mut self, asset: ActivityAsset) {
        self.assets.push(asset);
    }

    pub fn add_snapshot(&mut self, snapshot: ActivitySnapshot) {
        self.snapshots.push(snapshot);
    }

    pub fn end_activity(&mut self) {
        self.end = Some(Utc::now());
    }
}
