mod error;

pub use error::{AssetError, AssetResult};

use std::sync::Arc;

use be_remote_db::{DatabaseManager, NewAsset as DbCreateAssetRequest};
use be_storage::StorageService;
use chrono::{DateTime, Utc};
use prost_types::Timestamp;
use tracing::{debug, error, info};
use uuid::Uuid;

use proto_gen::asset::{Asset, AssetResponse, CreateAssetRequest};

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
    fn db_asset_to_proto(asset: &be_remote_db::Asset) -> Asset {
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
}

/// Convert DateTime<Utc> to prost_types::Timestamp
fn datetime_to_timestamp(dt: DateTime<Utc>) -> Timestamp {
    Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    }
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
                .link_asset_to_activity(activity_id, asset.id, user_id)
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
}
