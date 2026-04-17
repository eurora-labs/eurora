use agent_chain_core::messages::ContentBlocks;
use chrono::{DateTime, Utc};
use enum_dispatch::enum_dispatch;
use euro_native_messaging::NativeMessage;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use url::Url;
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
    pub name: String,
    pub icon: Option<String>,
    pub domain: Option<String>,
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

#[enum_dispatch]
pub trait AssetFunctionality {
    fn get_id(&self) -> &str;
    fn get_name(&self) -> &str;
    fn get_icon(&self) -> Option<&str>;
    fn construct_messages(&self) -> ContentBlocks;
}

#[enum_dispatch]
pub trait SnapshotFunctionality {
    fn get_id(&self) -> &str;
    fn construct_messages(&self) -> ContentBlocks;
    fn get_updated_at(&self) -> u64;
    fn get_created_at(&self) -> u64;
}

#[enum_dispatch(SnapshotFunctionality)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivitySnapshot {
    YoutubeSnapshot,
    ArticleSnapshot,
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
            _ => Err(anyhow::anyhow!("Invalid snapshot type")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    pub id: String,
    pub name: String,
    pub title: Option<String>,
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
        title: Option<String>,
        icon: Option<Arc<image::RgbaImage>>,
        process_name: String,
        assets: Vec<ActivityAsset>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            title,
            icon,
            process_name,
            start: Utc::now(),
            end: None,
            assets,
            snapshots: Vec::new(),
        }
    }

    pub fn get_context_chip(&self) -> ContextChip {
        ContextChip {
            id: self.id.clone(),
            name: self.title.clone().unwrap_or_else(|| self.name.clone()),
            icon: None,
            domain: extract_domain(&self.name),
        }
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

fn extract_domain(value: &str) -> Option<String> {
    let host = Url::parse(value).ok()?.host_str()?.to_ascii_lowercase();
    Some(host.strip_prefix("www.").unwrap_or(&host).to_owned())
}

#[cfg(test)]
mod tests {
    use super::extract_domain;

    #[test]
    fn extracts_bare_host() {
        assert_eq!(extract_domain("https://x.com/some/path"), Some("x.com".into()));
    }

    #[test]
    fn strips_www_and_lowercases() {
        assert_eq!(
            extract_domain("https://WWW.Example.COM/"),
            Some("example.com".into())
        );
    }

    #[test]
    fn preserves_subdomains() {
        assert_eq!(
            extract_domain("https://m.youtube.com/watch?v=1"),
            Some("m.youtube.com".into())
        );
    }

    #[test]
    fn returns_none_for_non_urls() {
        assert_eq!(extract_domain("Some Window Title"), None);
        assert_eq!(extract_domain(""), None);
    }
}
