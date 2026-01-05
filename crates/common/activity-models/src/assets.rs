use agent_chain_core::{BaseMessage, HumanMessage};
use async_trait::async_trait;
use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};
use serde_json::Value;

mod video_frame;
mod video_transcript;

/// Enum containing all possible activity assets
#[enum_dispatch(SaveableAsset, AssetFunctionality)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivityAsset {
    VideoTranscriptAsset,
}

/// Trait for assets that can be saved to disk
#[async_trait]
#[enum_dispatch]
pub trait SaveableAsset {
    // async fn load(bytes: &[u8]) -> ActivityResult<Self>
    // where
    //     Self: Sized,
    // {
    // }

    /// Get the asset type for organizing files
    fn get_asset_type(&self) -> &'static str;

    /// Serialize the asset content for saving
    async fn serialize_content(&self) -> ActivityResult<Vec<u8>>;

    /// Get a unique identifier for the asset (used for filename)
    fn get_unique_id(&self) -> String;

    /// Get a human-readable name for the asset
    fn get_display_name(&self) -> String;
}

#[enum_dispatch]
pub trait AssetFunctionality {
    fn construct_messages(&self) -> Vec<BaseMessage>;
    fn get_context_card(&self) -> Value;
}
