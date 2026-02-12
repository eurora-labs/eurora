use async_trait::async_trait;
use enum_dispatch::enum_dispatch;
use euro_auth::AuthedChannel;
use log::{debug, error};
use prost_types::Timestamp;
use proto_gen::activity::{
    ActivityResponse, InsertActivityRequest,
    proto_activity_service_client::ProtoActivityServiceClient,
};
use proto_gen::asset::{CreateAssetRequest, proto_asset_service_client::ProtoAssetServiceClient};
use serde::{Deserialize, Serialize};
use std::{io::Cursor, path::PathBuf};
use tonic::Status;

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
    // async fn load(bytes: &[u8]) -> ActivityResult<Self>
    // where
    //     Self: Sized,
    // {
    // }

    fn get_asset_type(&self) -> &'static str;

    async fn serialize_content(&self) -> ActivityResult<Vec<u8>>;

    fn get_unique_id(&self) -> String;

    fn get_display_name(&self) -> String;
}

pub struct ActivityStorage {
    activity_client: ProtoActivityServiceClient<AuthedChannel>,
    asset_client: ProtoAssetServiceClient<AuthedChannel>,
}

impl ActivityStorage {
    pub async fn new() -> Self {
        let channel = euro_auth::get_authed_channel().await;
        let asset_client = ProtoAssetServiceClient::new(channel);

        let channel = euro_auth::get_authed_channel().await;
        let activity_client = ProtoActivityServiceClient::new(channel);

        Self {
            activity_client,
            asset_client,
        }
    }

    pub async fn save_activity_to_service(
        &self,
        activity: &Activity,
    ) -> ActivityResult<ActivityResponse> {
        let mut client = self.activity_client.clone();
        let icon = match &activity.icon {
            Some(icon) => {
                let mut bytes: Vec<u8> = Vec::new();
                icon.write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
                    .map_err(ActivityError::Image)?;
                Some(bytes)
            }
            None => None,
        };
        // let icon = activity.icon.as_ref().map(|icon| {
        //     let mut bytes: Vec<u8> = Vec::new();
        //     icon.write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
        //         .map_err(|e| ActivityError::Image(e)).;
        //     bytes
        // });
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
                error!("Failed to insert activity: {}", e);
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
            // if ids.contains(&asset.get_id().to_string()) {
            let saved_info = self.save_asset_to_service(asset).await?;
            saved_assets.push(saved_info);
            // }
        }

        Ok(saved_assets)
    }

    pub async fn save_asset_to_service(
        &self,
        asset: &ActivityAsset,
    ) -> ActivityResult<SavedAssetInfo> {
        // let service_endpoint = self.config.service_endpoint.as_ref().ok_or_else(|| {
        //     ActivityError::Configuration("service_endpoint not configured".to_string())
        // })?;

        let bytes = serde_json::to_vec(asset)?;
        let file_size = bytes.len() as u64;

        let mut client = self.asset_client.clone();

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

        debug!("Asset saved with ID: {}", created_asset.id);

        Ok(SavedAssetInfo {
            file_path: PathBuf::from(&created_asset.storage_uri),
            absolute_path: PathBuf::from(&created_asset.storage_uri),
            content_hash: created_asset.checksum_sha256,
            file_size,
            saved_at: chrono::Utc::now(),
        })
    }
}
