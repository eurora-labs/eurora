use async_trait::async_trait;
use enum_dispatch::enum_dispatch;
use euro_auth::{AuthManager, AuthedChannel, build_authed_channel};
use prost_types::Timestamp;
use proto_gen::activity::{
    ActivityResponse, InsertActivityRequest,
    proto_activity_service_client::ProtoActivityServiceClient,
};
use proto_gen::asset::{CreateAssetRequest, proto_asset_service_client::ProtoAssetServiceClient};
use serde::{Deserialize, Serialize};
use std::{io::Cursor, path::PathBuf};
use tokio::sync::watch;
use tonic::Status;
use tonic::transport::Channel;

use crate::{Activity, ActivityAsset, ActivityError, error::ActivityResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedAssetInfo {
    pub file_path: PathBuf,
    pub absolute_path: PathBuf,
    pub content_hash: Option<String>,
    pub file_size: u64,
    pub saved_at: chrono::DateTime<chrono::Utc>,
}

#[async_trait]
#[enum_dispatch]
pub trait SaveableAsset {
    fn get_asset_type(&self) -> &'static str;

    async fn serialize_content(&self) -> ActivityResult<Vec<u8>>;

    fn get_unique_id(&self) -> String;

    fn get_display_name(&self) -> String;
}

pub struct ActivityStorage {
    channel_rx: watch::Receiver<Channel>,
    auth_manager: AuthManager,
}

impl ActivityStorage {
    pub fn new(channel_rx: watch::Receiver<Channel>) -> Self {
        let auth_manager = AuthManager::new(channel_rx.clone());
        Self {
            channel_rx,
            auth_manager,
        }
    }

    fn activity_client(&self) -> ProtoActivityServiceClient<AuthedChannel> {
        let channel = self.channel_rx.borrow().clone();
        let authed = build_authed_channel(channel, self.auth_manager.clone());
        ProtoActivityServiceClient::new(authed)
    }

    fn asset_client(&self) -> ProtoAssetServiceClient<AuthedChannel> {
        let channel = self.channel_rx.borrow().clone();
        let authed = build_authed_channel(channel, self.auth_manager.clone());
        ProtoAssetServiceClient::new(authed)
    }

    pub async fn save_activity_to_service(
        &self,
        activity: &Activity,
    ) -> ActivityResult<ActivityResponse> {
        let mut client = self.activity_client();
        let icon = match &activity.icon {
            Some(icon) => {
                let mut bytes: Vec<u8> = Vec::new();
                icon.write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
                    .map_err(ActivityError::Image)?;
                Some(bytes)
            }
            None => None,
        };
        let response = client
            .insert_activity(InsertActivityRequest {
                id: None,
                name: activity.name.clone(),
                process_name: activity.process_name.clone(),
                window_title: activity.process_name.clone(),
                icon,
                started_at: Some(Timestamp {
                    seconds: activity.start.timestamp(),
                    nanos: activity.start.timestamp_subsec_nanos() as i32,
                }),
                ended_at: None,
            })
            .await
            .map_err(|e| {
                tracing::error!("Failed to insert activity: {}", e);
                Status::internal("Failed to insert activity")
            })
            .expect("Failed to insert activity");

        Ok(response.into_inner())
    }

    pub async fn save_assets_to_service_by_ids(
        &self,
        activity: &Activity,
        _ids: &[String],
    ) -> ActivityResult<Vec<SavedAssetInfo>> {
        let mut saved_assets = Vec::new();

        for asset in &activity.assets {
            let saved_info = self.save_asset_to_service(asset).await?;
            saved_assets.push(saved_info);
        }

        Ok(saved_assets)
    }

    pub async fn save_asset_to_service(
        &self,
        asset: &ActivityAsset,
    ) -> ActivityResult<SavedAssetInfo> {
        let mut client = self.asset_client();

        let bytes = serde_json::to_vec(asset)?;
        let file_size = bytes.len() as u64;

        let metadata = serde_json::json!({
            "asset_type": asset.get_asset_type(),
            "unique_id": asset.get_unique_id(),
            "display_name": asset.get_display_name(),
        });

        let request = tonic::Request::new(CreateAssetRequest {
            name: asset.get_display_name(),
            content: bytes,
            mime_type: "application/json".to_string(),
            metadata: Some(metadata.to_string()),
            activity_id: None,
        });

        let response = client
            .create_asset(request)
            .await
            .map_err(|e| ActivityError::Network(format!("gRPC call failed: {}", e)))?;

        let asset_response = response.into_inner();
        let created_asset = asset_response
            .asset
            .ok_or_else(|| ActivityError::Network("No asset returned from service".to_string()))?;

        tracing::debug!("Asset saved with ID: {}", created_asset.id);

        Ok(SavedAssetInfo {
            file_path: PathBuf::from(&created_asset.storage_uri),
            absolute_path: PathBuf::from(&created_asset.storage_uri),
            content_hash: created_asset.checksum_sha256,
            file_size,
            saved_at: chrono::Utc::now(),
        })
    }
}
