mod error;

pub use error::{AssetError, AssetResult};

use std::sync::Arc;

use be_storage::StorageService;
use chrono::{DateTime, Utc};
use euro_remote_db::{
    CreateAssetRequest as DbCreateAssetRequest, DatabaseManager,
    UpdateAssetRequest as DbUpdateAssetRequest,
};
use prost_types::Timestamp;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use asset_models::proto::{
    ActivityAsset, ActivityAssetResponse, Asset, AssetResponse, CreateAssetRequest,
    DeleteAssetRequest, FindAssetBySha256Request, GetAssetRequest, GetAssetsByActivityIdRequest,
    GetAssetsByMessageIdRequest, LinkAssetToActivityRequest, LinkAssetToMessageRequest,
    ListAssetsRequest, ListAssetsResponse, MessageAsset, MessageAssetResponse,
    UnlinkAssetFromActivityRequest, UnlinkAssetFromMessageRequest, UpdateAssetRequest,
};

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
    ///
    /// # Errors
    ///
    /// Returns [`AssetError::StorageConfig`] if the storage service
    /// cannot be configured from environment variables.
    pub fn from_env(db: Arc<DatabaseManager>) -> AssetResult<Self> {
        let storage = StorageService::from_env().map_err(AssetError::StorageConfig)?;
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
            checksum_sha256: asset
                .checksum_sha256
                .as_ref()
                .map(|h| general_purpose::STANDARD.encode(h)),
            size_bytes: asset.size_bytes,
            storage_uri: asset.storage_uri.clone(),
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
fn decode_sha256(base64_hash: &str) -> AssetResult<Vec<u8>> {
    use base64::{Engine as _, engine::general_purpose};
    general_purpose::STANDARD
        .decode(base64_hash)
        .map_err(AssetError::InvalidSha256)
}

impl AssetService {
    pub async fn create_asset(
        &self,
        req: CreateAssetRequest,
        user_id: Uuid,
    ) -> AssetResult<AssetResponse> {
        info!("CreateAsset request received");

        // Validate request
        if req.content.is_empty() {
            return Err(AssetError::EmptyContent);
        }

        if req.mime_type.is_empty() {
            return Err(AssetError::MissingMimeType);
        }

        // Calculate SHA256 hash and byte size
        let checksum_sha256 = StorageService::calculate_sha256(&req.content);
        let size_bytes = req.content.len() as i64;

        debug!(
            "Processing asset: {} bytes, SHA256: {}",
            size_bytes,
            hex::encode(&checksum_sha256)
        );

        // // Check for deduplication - if we already have this exact content, return existing asset
        // if let Ok(Some(existing_asset)) = self
        //     .db
        //     .find_asset_by_sha256(user_id, &checksum_sha256)
        //     .await
        // {
        //     info!(
        //         "Found existing asset {} with same SHA256 hash, reusing",
        //         existing_asset.id
        //     );

        //     // If activity_id is provided, link the existing asset to it
        //     if let Some(activity_id_str) = &req.activity_id {
        //         let activity_id =
        //             Uuid::parse_str(activity_id_str).map_err(AssetError::InvalidActivityId)?;

        //         // Try to link, ignore if already linked
        //         let _ = self
        //             .db
        //             .link_asset_to_activity(activity_id, existing_asset.id)
        //             .await;
        //     }

        //     return Ok(AssetResponse {
        //         asset: Some(Self::db_asset_to_proto(&existing_asset)),
        //     });
        // }

        // Generate new asset ID
        let asset_id = Uuid::now_v7();

        // Upload content to storage
        let storage_uri = self
            .storage
            .upload(&user_id, &asset_id, &req.content, &req.mime_type)
            .await
            .map_err(|e| {
                error!("Failed to upload asset to storage: {}", e);
                AssetError::StorageUpload(e)
            })?;

        // Parse metadata if provided
        let metadata = req
            .metadata
            .as_ref()
            .map(|m| serde_json::from_str(m))
            .transpose()
            .map_err(AssetError::InvalidMetadata)?;

        // Create database record
        let db_request = DbCreateAssetRequest {
            id: asset_id,
            name: req.name,
            checksum_sha256: Some(checksum_sha256),
            size_bytes: Some(size_bytes),
            storage_uri,
            mime_type: req.mime_type,
            metadata,
        };

        let asset = self
            .db
            .create_asset(user_id, db_request)
            .await
            .map_err(|e| {
                error!("Failed to create asset in database: {}", e);
                AssetError::DatabaseCreate(e)
            })?;

        // Link to activity if activity_id provided
        if let Some(activity_id_str) = &req.activity_id {
            let activity_id =
                Uuid::parse_str(activity_id_str).map_err(AssetError::InvalidActivityId)?;

            self.db
                .link_asset_to_activity(activity_id, asset.id)
                .await
                .map_err(|e| {
                    error!("Failed to link asset to activity: {}", e);
                    AssetError::DatabaseLinkActivity(e)
                })?;
        }

        debug!("Created asset {} for user {}", asset.id, user_id);

        Ok(AssetResponse {
            asset: Some(Self::db_asset_to_proto(&asset)),
        })
    }

    pub async fn get_asset(
        &self,
        req: GetAssetRequest,
        user_id: Uuid,
    ) -> AssetResult<AssetResponse> {
        info!("GetAsset request received");

        let asset_id = Uuid::parse_str(&req.id).map_err(AssetError::InvalidAssetId)?;

        let asset = self
            .db
            .get_asset_for_user(asset_id, user_id)
            .await
            .map_err(|e| {
                warn!("Asset not found: {}", e);
                AssetError::AssetNotFound
            })?;

        debug!("Retrieved asset {} for user {}", asset_id, user_id);

        Ok(AssetResponse {
            asset: Some(Self::db_asset_to_proto(&asset)),
        })
    }

    pub async fn list_assets(
        &self,
        req: ListAssetsRequest,
        user_id: Uuid,
    ) -> AssetResult<ListAssetsResponse> {
        info!("ListAssets request received");

        let limit = if req.limit == 0 { 50 } else { req.limit };

        let (assets, total_count) = self
            .db
            .list_assets(user_id, limit, req.offset)
            .await
            .map_err(|e| {
                error!("Failed to list assets: {}", e);
                AssetError::DatabaseList(e)
            })?;

        let proto_assets: Vec<Asset> = assets.iter().map(Self::db_asset_to_proto).collect();

        debug!("Listed {} assets for user {}", proto_assets.len(), user_id);

        Ok(ListAssetsResponse {
            assets: proto_assets,
            total_count,
        })
    }

    pub async fn update_asset(
        &self,
        req: UpdateAssetRequest,
        user_id: Uuid,
    ) -> AssetResult<AssetResponse> {
        info!("UpdateAsset request received");

        let asset_id = Uuid::parse_str(&req.id).map_err(AssetError::InvalidAssetId)?;

        let checksum_sha256 = req
            .checksum_sha256
            .as_ref()
            .map(|h| decode_sha256(h))
            .transpose()?;

        let metadata = req
            .metadata
            .as_ref()
            .map(|m| serde_json::from_str(m))
            .transpose()
            .map_err(AssetError::InvalidMetadata)?;

        let db_request = DbUpdateAssetRequest {
            checksum_sha256,
            size_bytes: req.size_bytes,
            storage_uri: req.storage_uri,
            mime_type: req.mime_type,
            metadata,
        };

        let asset = self
            .db
            .update_asset(asset_id, user_id, db_request)
            .await
            .map_err(|e| {
                error!("Failed to update asset: {}", e);
                AssetError::DatabaseUpdate(e)
            })?;

        debug!("Updated asset {} for user {}", asset_id, user_id);

        Ok(AssetResponse {
            asset: Some(Self::db_asset_to_proto(&asset)),
        })
    }

    pub async fn delete_asset(&self, req: DeleteAssetRequest, user_id: Uuid) -> AssetResult<()> {
        info!("DeleteAsset request received");

        let asset_id = Uuid::parse_str(&req.id).map_err(AssetError::InvalidAssetId)?;

        // Get the asset first to get the file path for storage deletion
        let asset = self
            .db
            .get_asset_for_user(asset_id, user_id)
            .await
            .map_err(|e| {
                warn!("Asset not found for deletion: {}", e);
                AssetError::AssetNotFound
            })?;

        // Delete from storage
        if let Err(e) = self.storage.delete(&asset.storage_uri).await {
            warn!(
                "Failed to delete asset from storage (continuing with DB deletion): {}",
                e
            );
            // Continue with database deletion even if storage deletion fails
        }

        // Delete from database
        self.db.delete_asset(asset_id, user_id).await.map_err(|e| {
            error!("Failed to delete asset: {}", e);
            AssetError::DatabaseDelete(e)
        })?;

        debug!("Deleted asset {} for user {}", asset_id, user_id);

        Ok(())
    }

    pub async fn find_asset_by_sha256(
        &self,
        req: FindAssetBySha256Request,
        user_id: Uuid,
    ) -> AssetResult<AssetResponse> {
        info!("FindAssetBySha256 request received");

        let checksum_sha256 = decode_sha256(&req.checksum_sha256)?;

        let asset = self
            .db
            .find_asset_by_sha256(user_id, &checksum_sha256)
            .await
            .map_err(|e| {
                error!("Failed to find asset by SHA256: {}", e);
                AssetError::DatabaseFindBySha256(e)
            })?;

        debug!(
            "Find asset by SHA256 for user {}: found={}",
            user_id,
            asset.is_some()
        );

        Ok(AssetResponse {
            asset: asset.as_ref().map(Self::db_asset_to_proto),
        })
    }

    pub async fn get_assets_by_message_id(
        &self,
        req: GetAssetsByMessageIdRequest,
        user_id: Uuid,
    ) -> AssetResult<ListAssetsResponse> {
        info!("GetAssetsByMessageId request received");

        let message_id = Uuid::parse_str(&req.message_id).map_err(AssetError::InvalidMessageId)?;

        let assets = self
            .db
            .get_assets_by_message_id(message_id, user_id)
            .await
            .map_err(|e| {
                error!("Failed to get assets by message ID: {}", e);
                AssetError::DatabaseGetByMessageId(e)
            })?;

        let proto_assets: Vec<Asset> = assets.iter().map(Self::db_asset_to_proto).collect();
        let total_count = proto_assets.len() as u64;

        debug!(
            "Retrieved {} assets for message {} for user {}",
            total_count, message_id, user_id
        );

        Ok(ListAssetsResponse {
            assets: proto_assets,
            total_count,
        })
    }

    pub async fn get_assets_by_activity_id(
        &self,
        req: GetAssetsByActivityIdRequest,
        user_id: Uuid,
    ) -> AssetResult<ListAssetsResponse> {
        info!("GetAssetsByActivityId request received");

        let activity_id =
            Uuid::parse_str(&req.activity_id).map_err(AssetError::InvalidActivityId)?;

        let assets = self
            .db
            .get_assets_by_activity_id(activity_id, user_id)
            .await
            .map_err(|e| {
                error!("Failed to get assets by activity ID: {}", e);
                AssetError::DatabaseGetByActivityId(e)
            })?;

        let proto_assets: Vec<Asset> = assets.iter().map(Self::db_asset_to_proto).collect();
        let total_count = proto_assets.len() as u64;

        debug!(
            "Retrieved {} assets for activity {} for user {}",
            total_count, activity_id, user_id
        );

        Ok(ListAssetsResponse {
            assets: proto_assets,
            total_count,
        })
    }

    pub async fn link_asset_to_message(
        &self,
        req: LinkAssetToMessageRequest,
        user_id: Uuid,
    ) -> AssetResult<MessageAssetResponse> {
        info!("LinkAssetToMessage request received");

        let message_id = Uuid::parse_str(&req.message_id).map_err(AssetError::InvalidMessageId)?;

        let asset_id = Uuid::parse_str(&req.asset_id).map_err(AssetError::InvalidAssetId)?;

        // Verify the asset belongs to the user
        self.db
            .get_asset_for_user(asset_id, user_id)
            .await
            .map_err(|_| AssetError::AssetNotOwned)?;

        let message_asset = self
            .db
            .link_asset_to_message(message_id, asset_id)
            .await
            .map_err(|e| {
                error!("Failed to link asset to message: {}", e);
                AssetError::DatabaseLinkMessage(e)
            })?;

        debug!(
            "Linked asset {} to message {} for user {}",
            asset_id, message_id, user_id
        );

        Ok(MessageAssetResponse {
            message_asset: Some(Self::db_message_asset_to_proto(&message_asset)),
        })
    }

    pub async fn unlink_asset_from_message(
        &self,
        req: UnlinkAssetFromMessageRequest,
        user_id: Uuid,
    ) -> AssetResult<()> {
        info!("UnlinkAssetFromMessage request received");

        let message_id = Uuid::parse_str(&req.message_id).map_err(AssetError::InvalidMessageId)?;

        let asset_id = Uuid::parse_str(&req.asset_id).map_err(AssetError::InvalidAssetId)?;

        // Verify the asset belongs to the user
        self.db
            .get_asset_for_user(asset_id, user_id)
            .await
            .map_err(|_| AssetError::AssetNotOwned)?;

        self.db
            .unlink_asset_from_message(message_id, asset_id)
            .await
            .map_err(|e| {
                error!("Failed to unlink asset from message: {}", e);
                AssetError::DatabaseUnlinkMessage(e)
            })?;

        debug!(
            "Unlinked asset {} from message {} for user {}",
            asset_id, message_id, user_id
        );

        Ok(())
    }

    pub async fn link_asset_to_activity(
        &self,
        req: LinkAssetToActivityRequest,
        user_id: Uuid,
    ) -> AssetResult<ActivityAssetResponse> {
        info!("LinkAssetToActivity request received");

        let activity_id =
            Uuid::parse_str(&req.activity_id).map_err(AssetError::InvalidActivityId)?;

        let asset_id = Uuid::parse_str(&req.asset_id).map_err(AssetError::InvalidAssetId)?;

        // Verify the asset belongs to the user
        self.db
            .get_asset_for_user(asset_id, user_id)
            .await
            .map_err(|_| AssetError::AssetNotOwned)?;

        let activity_asset = self
            .db
            .link_asset_to_activity(activity_id, asset_id)
            .await
            .map_err(|e| {
                error!("Failed to link asset to activity: {}", e);
                AssetError::DatabaseLinkActivity(e)
            })?;

        debug!(
            "Linked asset {} to activity {} for user {}",
            asset_id, activity_id, user_id
        );

        Ok(ActivityAssetResponse {
            activity_asset: Some(Self::db_activity_asset_to_proto(&activity_asset)),
        })
    }

    pub async fn unlink_asset_from_activity(
        &self,
        req: UnlinkAssetFromActivityRequest,
        user_id: Uuid,
    ) -> AssetResult<()> {
        info!("UnlinkAssetFromActivity request received");

        let activity_id =
            Uuid::parse_str(&req.activity_id).map_err(AssetError::InvalidActivityId)?;

        let asset_id = Uuid::parse_str(&req.asset_id).map_err(AssetError::InvalidAssetId)?;

        // Verify the asset belongs to the user
        self.db
            .get_asset_for_user(asset_id, user_id)
            .await
            .map_err(|_| AssetError::AssetNotOwned)?;

        self.db
            .unlink_asset_from_activity(activity_id, asset_id)
            .await
            .map_err(|e| {
                error!("Failed to unlink asset from activity: {}", e);
                AssetError::DatabaseUnlinkActivity(e)
            })?;

        debug!(
            "Unlinked asset {} from activity {} for user {}",
            asset_id, activity_id, user_id
        );

        Ok(())
    }
}
