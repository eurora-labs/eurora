//! Server-side implementation for the Assets Service.
//!
//! This module contains the gRPC server implementation and is only
//! available when the `server` feature is enabled.

use std::sync::Arc;

use anyhow::Result;
use be_auth_grpc::Claims;
use chrono::{DateTime, Utc};
use euro_remote_db::{
    CreateAssetRequest as DbCreateAssetRequest, DatabaseManager,
    UpdateAssetRequest as DbUpdateAssetRequest,
};
use prost_types::Timestamp;
use tonic::{Request, Response, Status};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::proto::{
    ActivityAsset, ActivityAssetResponse, Asset, AssetResponse, CreateAssetRequest,
    DeleteAssetRequest, FindAssetBySha256Request, GetAssetRequest, GetAssetsByActivityIdRequest,
    GetAssetsByMessageIdRequest, LinkAssetToActivityRequest, LinkAssetToMessageRequest,
    ListAssetsRequest, ListAssetsResponse, MessageAsset, MessageAssetResponse,
    UnlinkAssetFromActivityRequest, UnlinkAssetFromMessageRequest, UpdateAssetRequest,
};
use crate::storage::StorageService;

pub use crate::proto::proto_asset_service_server::{ProtoAssetService, ProtoAssetServiceServer};

/// The main assets service
#[derive(Debug)]
pub struct AssetService {
    db: Arc<DatabaseManager>,
    storage: Arc<StorageService>,
}

impl AssetService {
    /// Create a new AssetsService instance
    pub fn new(db: Arc<DatabaseManager>, storage: Arc<StorageService>) -> Self {
        info!("Creating new AssetsService instance");
        Self { db, storage }
    }

    /// Create a new AssetsService with storage configured from environment
    pub fn from_env(db: Arc<DatabaseManager>) -> Result<Self> {
        let storage = StorageService::from_env()?;
        Ok(Self::new(db, Arc::new(storage)))
    }

    /// Get the storage service reference
    pub fn storage(&self) -> &StorageService {
        &self.storage
    }

    /// Convert a database Asset to a proto Asset
    fn db_asset_to_proto(asset: &euro_remote_db::Asset) -> Asset {
        use base64::{Engine as _, engine::general_purpose};

        Asset {
            id: asset.id.to_string(),
            content_sha256: asset
                .content_sha256
                .as_ref()
                .map(|h| general_purpose::STANDARD.encode(h)),
            byte_size: asset.byte_size,
            file_path: asset.file_path.clone(),
            mime_type: asset.mime_type.clone(),
            metadata: asset.metadata.to_string(),
            created_at: Some(datetime_to_timestamp(asset.created_at)),
            updated_at: Some(datetime_to_timestamp(asset.updated_at)),
        }
    }

    /// Convert a database MessageAsset to a proto MessageAsset
    fn db_message_asset_to_proto(ma: &euro_remote_db::MessageAsset) -> MessageAsset {
        MessageAsset {
            message_id: ma.message_id.to_string(),
            asset_id: ma.asset_id.to_string(),
            created_at: Some(datetime_to_timestamp(ma.created_at)),
        }
    }

    /// Convert a database ActivityAsset to a proto ActivityAsset
    fn db_activity_asset_to_proto(aa: &euro_remote_db::ActivityAsset) -> ActivityAsset {
        ActivityAsset {
            activity_id: aa.activity_id.to_string(),
            asset_id: aa.asset_id.to_string(),
            created_at: Some(datetime_to_timestamp(aa.created_at)),
        }
    }
}

/// Convert DateTime<Utc> to prost_types::Timestamp
fn datetime_to_timestamp(dt: DateTime<Utc>) -> Timestamp {
    Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    }
}

/// Decode base64 SHA256 hash
fn decode_sha256(base64_hash: &str) -> Result<Vec<u8>, Status> {
    use base64::{Engine as _, engine::general_purpose};
    general_purpose::STANDARD
        .decode(base64_hash)
        .map_err(|e| Status::invalid_argument(format!("Invalid base64 SHA256 hash: {}", e)))
}

#[tonic::async_trait]
impl ProtoAssetService for AssetService {
    async fn create_asset(
        &self,
        request: Request<CreateAssetRequest>,
    ) -> Result<Response<AssetResponse>, Status> {
        info!("CreateAsset request received");

        let claims = request.extensions().get::<Claims>().ok_or_else(|| {
            error!("Missing claims in request");
            Status::unauthenticated("Missing claims")
        })?;

        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|e| Status::internal(format!("Invalid user ID: {}", e)))?;

        let req = request.into_inner();

        // Validate request
        if req.content.is_empty() {
            return Err(Status::invalid_argument("Content cannot be empty"));
        }

        if req.mime_type.is_empty() {
            return Err(Status::invalid_argument("MIME type is required"));
        }

        // Calculate SHA256 hash and byte size
        let content_sha256 = StorageService::calculate_sha256(&req.content);
        let byte_size = req.content.len() as i64;

        debug!(
            "Processing asset: {} bytes, SHA256: {}",
            byte_size,
            hex::encode(&content_sha256)
        );

        // Check for deduplication - if we already have this exact content, return existing asset
        if let Ok(Some(existing_asset)) =
            self.db.find_asset_by_sha256(user_id, &content_sha256).await
        {
            info!(
                "Found existing asset {} with same SHA256 hash, reusing",
                existing_asset.id
            );

            // If activity_id is provided, link the existing asset to it
            if let Some(activity_id_str) = &req.activity_id {
                let activity_id = Uuid::parse_str(activity_id_str)
                    .map_err(|e| Status::invalid_argument(format!("Invalid activity ID: {}", e)))?;

                // Try to link, ignore if already linked
                let _ = self
                    .db
                    .link_asset_to_activity(activity_id, existing_asset.id)
                    .await;
            }

            return Ok(Response::new(AssetResponse {
                asset: Some(Self::db_asset_to_proto(&existing_asset)),
            }));
        }

        // Generate new asset ID
        let asset_id = Uuid::now_v7();

        // Upload content to storage
        let file_path = self
            .storage
            .upload(&user_id, &asset_id, &req.content, &req.mime_type)
            .await
            .map_err(|e| {
                error!("Failed to upload asset to storage: {}", e);
                Status::internal("Failed to upload asset to storage")
            })?;

        // Parse metadata if provided
        let metadata = req
            .metadata
            .as_ref()
            .map(|m| serde_json::from_str(m))
            .transpose()
            .map_err(|e| Status::invalid_argument(format!("Invalid metadata JSON: {}", e)))?;

        // Create database record
        let db_request = DbCreateAssetRequest {
            id: asset_id,
            content_sha256: Some(content_sha256),
            byte_size: Some(byte_size),
            file_path,
            mime_type: req.mime_type,
            metadata,
        };

        let asset = self
            .db
            .create_asset(user_id, db_request)
            .await
            .map_err(|e| {
                error!("Failed to create asset in database: {}", e);
                Status::internal("Failed to create asset")
            })?;

        // Link to activity if activity_id provided
        if let Some(activity_id_str) = &req.activity_id {
            let activity_id = Uuid::parse_str(activity_id_str)
                .map_err(|e| Status::invalid_argument(format!("Invalid activity ID: {}", e)))?;

            self.db
                .link_asset_to_activity(activity_id, asset.id)
                .await
                .map_err(|e| {
                    error!("Failed to link asset to activity: {}", e);
                    Status::internal("Failed to link asset to activity")
                })?;
        }

        debug!("Created asset {} for user {}", asset.id, user_id);

        Ok(Response::new(AssetResponse {
            asset: Some(Self::db_asset_to_proto(&asset)),
        }))
    }

    async fn get_asset(
        &self,
        request: Request<GetAssetRequest>,
    ) -> Result<Response<AssetResponse>, Status> {
        info!("GetAsset request received");

        let claims = request.extensions().get::<Claims>().ok_or_else(|| {
            error!("Missing claims in request");
            Status::unauthenticated("Missing claims")
        })?;

        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|e| Status::internal(format!("Invalid user ID: {}", e)))?;

        let req = request.into_inner();
        let asset_id = Uuid::parse_str(&req.id)
            .map_err(|e| Status::invalid_argument(format!("Invalid asset ID: {}", e)))?;

        let asset = self
            .db
            .get_asset_for_user(asset_id, user_id)
            .await
            .map_err(|e| {
                warn!("Asset not found: {}", e);
                Status::not_found("Asset not found")
            })?;

        debug!("Retrieved asset {} for user {}", asset_id, user_id);

        Ok(Response::new(AssetResponse {
            asset: Some(Self::db_asset_to_proto(&asset)),
        }))
    }

    async fn list_assets(
        &self,
        request: Request<ListAssetsRequest>,
    ) -> Result<Response<ListAssetsResponse>, Status> {
        info!("ListAssets request received");

        let claims = request.extensions().get::<Claims>().ok_or_else(|| {
            error!("Missing claims in request");
            Status::unauthenticated("Missing claims")
        })?;

        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|e| Status::internal(format!("Invalid user ID: {}", e)))?;

        let req = request.into_inner();
        let limit = if req.limit == 0 { 50 } else { req.limit };

        let (assets, total_count) = self
            .db
            .list_assets(user_id, limit, req.offset)
            .await
            .map_err(|e| {
                error!("Failed to list assets: {}", e);
                Status::internal("Failed to list assets")
            })?;

        let proto_assets: Vec<Asset> = assets.iter().map(Self::db_asset_to_proto).collect();

        debug!("Listed {} assets for user {}", proto_assets.len(), user_id);

        Ok(Response::new(ListAssetsResponse {
            assets: proto_assets,
            total_count,
        }))
    }

    async fn update_asset(
        &self,
        request: Request<UpdateAssetRequest>,
    ) -> Result<Response<AssetResponse>, Status> {
        info!("UpdateAsset request received");

        let claims = request.extensions().get::<Claims>().ok_or_else(|| {
            error!("Missing claims in request");
            Status::unauthenticated("Missing claims")
        })?;

        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|e| Status::internal(format!("Invalid user ID: {}", e)))?;

        let req = request.into_inner();

        let asset_id = Uuid::parse_str(&req.id)
            .map_err(|e| Status::invalid_argument(format!("Invalid asset ID: {}", e)))?;

        let content_sha256 = req
            .content_sha256
            .as_ref()
            .map(|h| decode_sha256(h))
            .transpose()?;

        let metadata = req
            .metadata
            .as_ref()
            .map(|m| serde_json::from_str(m))
            .transpose()
            .map_err(|e| Status::invalid_argument(format!("Invalid metadata JSON: {}", e)))?;

        let db_request = DbUpdateAssetRequest {
            content_sha256,
            byte_size: req.byte_size,
            file_path: req.file_path,
            mime_type: req.mime_type,
            metadata,
        };

        let asset = self
            .db
            .update_asset(asset_id, user_id, db_request)
            .await
            .map_err(|e| {
                error!("Failed to update asset: {}", e);
                Status::internal("Failed to update asset")
            })?;

        debug!("Updated asset {} for user {}", asset_id, user_id);

        Ok(Response::new(AssetResponse {
            asset: Some(Self::db_asset_to_proto(&asset)),
        }))
    }

    async fn delete_asset(
        &self,
        request: Request<DeleteAssetRequest>,
    ) -> Result<Response<()>, Status> {
        info!("DeleteAsset request received");

        let claims = request.extensions().get::<Claims>().ok_or_else(|| {
            error!("Missing claims in request");
            Status::unauthenticated("Missing claims")
        })?;

        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|e| Status::internal(format!("Invalid user ID: {}", e)))?;

        let req = request.into_inner();

        let asset_id = Uuid::parse_str(&req.id)
            .map_err(|e| Status::invalid_argument(format!("Invalid asset ID: {}", e)))?;

        // Get the asset first to get the file path for storage deletion
        let asset = self
            .db
            .get_asset_for_user(asset_id, user_id)
            .await
            .map_err(|e| {
                warn!("Asset not found for deletion: {}", e);
                Status::not_found("Asset not found")
            })?;

        // Delete from storage
        if let Err(e) = self.storage.delete(&asset.file_path).await {
            warn!(
                "Failed to delete asset from storage (continuing with DB deletion): {}",
                e
            );
            // Continue with database deletion even if storage deletion fails
        }

        // Delete from database
        self.db.delete_asset(asset_id, user_id).await.map_err(|e| {
            error!("Failed to delete asset: {}", e);
            Status::internal("Failed to delete asset")
        })?;

        debug!("Deleted asset {} for user {}", asset_id, user_id);

        Ok(Response::new(()))
    }

    async fn find_asset_by_sha256(
        &self,
        request: Request<FindAssetBySha256Request>,
    ) -> Result<Response<AssetResponse>, Status> {
        info!("FindAssetBySha256 request received");

        let claims = request.extensions().get::<Claims>().ok_or_else(|| {
            error!("Missing claims in request");
            Status::unauthenticated("Missing claims")
        })?;

        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|e| Status::internal(format!("Invalid user ID: {}", e)))?;

        let req = request.into_inner();
        let content_sha256 = decode_sha256(&req.content_sha256)?;

        let asset = self
            .db
            .find_asset_by_sha256(user_id, &content_sha256)
            .await
            .map_err(|e| {
                error!("Failed to find asset by SHA256: {}", e);
                Status::internal("Failed to find asset by SHA256")
            })?;

        debug!(
            "Find asset by SHA256 for user {}: found={}",
            user_id,
            asset.is_some()
        );

        Ok(Response::new(AssetResponse {
            asset: asset.as_ref().map(Self::db_asset_to_proto),
        }))
    }

    async fn get_assets_by_message_id(
        &self,
        request: Request<GetAssetsByMessageIdRequest>,
    ) -> Result<Response<ListAssetsResponse>, Status> {
        info!("GetAssetsByMessageId request received");

        let claims = request.extensions().get::<Claims>().ok_or_else(|| {
            error!("Missing claims in request");
            Status::unauthenticated("Missing claims")
        })?;

        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|e| Status::internal(format!("Invalid user ID: {}", e)))?;

        let req = request.into_inner();
        let message_id = Uuid::parse_str(&req.message_id)
            .map_err(|e| Status::invalid_argument(format!("Invalid message ID: {}", e)))?;

        let assets = self
            .db
            .get_assets_by_message_id(message_id, user_id)
            .await
            .map_err(|e| {
                error!("Failed to get assets by message ID: {}", e);
                Status::internal("Failed to get assets by message ID")
            })?;

        let proto_assets: Vec<Asset> = assets.iter().map(Self::db_asset_to_proto).collect();
        let total_count = proto_assets.len() as u64;

        debug!(
            "Retrieved {} assets for message {} for user {}",
            total_count, message_id, user_id
        );

        Ok(Response::new(ListAssetsResponse {
            assets: proto_assets,
            total_count,
        }))
    }

    async fn get_assets_by_activity_id(
        &self,
        request: Request<GetAssetsByActivityIdRequest>,
    ) -> Result<Response<ListAssetsResponse>, Status> {
        info!("GetAssetsByActivityId request received");

        let claims = request.extensions().get::<Claims>().ok_or_else(|| {
            error!("Missing claims in request");
            Status::unauthenticated("Missing claims")
        })?;

        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|e| Status::internal(format!("Invalid user ID: {}", e)))?;

        let req = request.into_inner();
        let activity_id = Uuid::parse_str(&req.activity_id)
            .map_err(|e| Status::invalid_argument(format!("Invalid activity ID: {}", e)))?;

        let assets = self
            .db
            .get_assets_by_activity_id(activity_id, user_id)
            .await
            .map_err(|e| {
                error!("Failed to get assets by activity ID: {}", e);
                Status::internal("Failed to get assets by activity ID")
            })?;

        let proto_assets: Vec<Asset> = assets.iter().map(Self::db_asset_to_proto).collect();
        let total_count = proto_assets.len() as u64;

        debug!(
            "Retrieved {} assets for activity {} for user {}",
            total_count, activity_id, user_id
        );

        Ok(Response::new(ListAssetsResponse {
            assets: proto_assets,
            total_count,
        }))
    }

    async fn link_asset_to_message(
        &self,
        request: Request<LinkAssetToMessageRequest>,
    ) -> Result<Response<MessageAssetResponse>, Status> {
        info!("LinkAssetToMessage request received");

        let claims = request.extensions().get::<Claims>().ok_or_else(|| {
            error!("Missing claims in request");
            Status::unauthenticated("Missing claims")
        })?;

        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|e| Status::internal(format!("Invalid user ID: {}", e)))?;

        let req = request.into_inner();

        let message_id = Uuid::parse_str(&req.message_id)
            .map_err(|e| Status::invalid_argument(format!("Invalid message ID: {}", e)))?;

        let asset_id = Uuid::parse_str(&req.asset_id)
            .map_err(|e| Status::invalid_argument(format!("Invalid asset ID: {}", e)))?;

        // Verify the asset belongs to the user
        let _ = self
            .db
            .get_asset_for_user(asset_id, user_id)
            .await
            .map_err(|_| Status::not_found("Asset not found or not owned by user"))?;

        let message_asset = self
            .db
            .link_asset_to_message(message_id, asset_id)
            .await
            .map_err(|e| {
                error!("Failed to link asset to message: {}", e);
                Status::internal("Failed to link asset to message")
            })?;

        debug!(
            "Linked asset {} to message {} for user {}",
            asset_id, message_id, user_id
        );

        Ok(Response::new(MessageAssetResponse {
            message_asset: Some(Self::db_message_asset_to_proto(&message_asset)),
        }))
    }

    async fn unlink_asset_from_message(
        &self,
        request: Request<UnlinkAssetFromMessageRequest>,
    ) -> Result<Response<()>, Status> {
        info!("UnlinkAssetFromMessage request received");

        let claims = request.extensions().get::<Claims>().ok_or_else(|| {
            error!("Missing claims in request");
            Status::unauthenticated("Missing claims")
        })?;

        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|e| Status::internal(format!("Invalid user ID: {}", e)))?;

        let req = request.into_inner();

        let message_id = Uuid::parse_str(&req.message_id)
            .map_err(|e| Status::invalid_argument(format!("Invalid message ID: {}", e)))?;

        let asset_id = Uuid::parse_str(&req.asset_id)
            .map_err(|e| Status::invalid_argument(format!("Invalid asset ID: {}", e)))?;

        // Verify the asset belongs to the user
        let _ = self
            .db
            .get_asset_for_user(asset_id, user_id)
            .await
            .map_err(|_| Status::not_found("Asset not found or not owned by user"))?;

        self.db
            .unlink_asset_from_message(message_id, asset_id)
            .await
            .map_err(|e| {
                error!("Failed to unlink asset from message: {}", e);
                Status::internal("Failed to unlink asset from message")
            })?;

        debug!(
            "Unlinked asset {} from message {} for user {}",
            asset_id, message_id, user_id
        );

        Ok(Response::new(()))
    }

    async fn link_asset_to_activity(
        &self,
        request: Request<LinkAssetToActivityRequest>,
    ) -> Result<Response<ActivityAssetResponse>, Status> {
        info!("LinkAssetToActivity request received");

        let claims = request.extensions().get::<Claims>().ok_or_else(|| {
            error!("Missing claims in request");
            Status::unauthenticated("Missing claims")
        })?;

        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|e| Status::internal(format!("Invalid user ID: {}", e)))?;

        let req = request.into_inner();

        let activity_id = Uuid::parse_str(&req.activity_id)
            .map_err(|e| Status::invalid_argument(format!("Invalid activity ID: {}", e)))?;

        let asset_id = Uuid::parse_str(&req.asset_id)
            .map_err(|e| Status::invalid_argument(format!("Invalid asset ID: {}", e)))?;

        // Verify the asset belongs to the user
        let _ = self
            .db
            .get_asset_for_user(asset_id, user_id)
            .await
            .map_err(|_| Status::not_found("Asset not found or not owned by user"))?;

        let activity_asset = self
            .db
            .link_asset_to_activity(activity_id, asset_id)
            .await
            .map_err(|e| {
                error!("Failed to link asset to activity: {}", e);
                Status::internal("Failed to link asset to activity")
            })?;

        debug!(
            "Linked asset {} to activity {} for user {}",
            asset_id, activity_id, user_id
        );

        Ok(Response::new(ActivityAssetResponse {
            activity_asset: Some(Self::db_activity_asset_to_proto(&activity_asset)),
        }))
    }

    async fn unlink_asset_from_activity(
        &self,
        request: Request<UnlinkAssetFromActivityRequest>,
    ) -> Result<Response<()>, Status> {
        info!("UnlinkAssetFromActivity request received");

        let claims = request.extensions().get::<Claims>().ok_or_else(|| {
            error!("Missing claims in request");
            Status::unauthenticated("Missing claims")
        })?;

        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|e| Status::internal(format!("Invalid user ID: {}", e)))?;

        let req = request.into_inner();

        let activity_id = Uuid::parse_str(&req.activity_id)
            .map_err(|e| Status::invalid_argument(format!("Invalid activity ID: {}", e)))?;

        let asset_id = Uuid::parse_str(&req.asset_id)
            .map_err(|e| Status::invalid_argument(format!("Invalid asset ID: {}", e)))?;

        // Verify the asset belongs to the user
        let _ = self
            .db
            .get_asset_for_user(asset_id, user_id)
            .await
            .map_err(|_| Status::not_found("Asset not found or not owned by user"))?;

        self.db
            .unlink_asset_from_activity(activity_id, asset_id)
            .await
            .map_err(|e| {
                error!("Failed to unlink asset from activity: {}", e);
                Status::internal("Failed to unlink asset from activity")
            })?;

        debug!(
            "Unlinked asset {} from activity {} for user {}",
            asset_id, activity_id, user_id
        );

        Ok(Response::new(()))
    }
}
