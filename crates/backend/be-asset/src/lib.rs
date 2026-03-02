mod error;

pub use error::{AssetError, AssetResult};

use std::sync::Arc;

use be_remote_db::DatabaseManager;
use be_storage::StorageService;
use chrono::{DateTime, Utc};
use prost_types::Timestamp;
use uuid::Uuid;

use proto_gen::asset::{Asset, AssetResponse, CreateAssetRequest};

const ALLOWED_MIME_TYPES: &[&str] = &[
    "image/png",
    "image/jpeg",
    "image/gif",
    "image/webp",
    "image/svg+xml",
    "application/pdf",
    "text/plain",
    "application/json",
    "application/octet-stream",
];

fn validate_content_matches_mime(content: &[u8], declared_mime: &str) -> bool {
    match declared_mime {
        "image/png" => content.starts_with(&[0x89, 0x50, 0x4E, 0x47]),
        "image/jpeg" => content.starts_with(&[0xFF, 0xD8, 0xFF]),
        "image/gif" => content.starts_with(b"GIF8"),
        "image/webp" => {
            content.len() >= 12 && &content[..4] == b"RIFF" && &content[8..12] == b"WEBP"
        }
        "image/svg+xml" => {
            let bytes = content.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(content);
            std::str::from_utf8(bytes)
                .map(|s| {
                    let t = s.trim_start();
                    t.starts_with("<svg") || t.starts_with("<?xml") || t.starts_with("<!DOCTYPE")
                })
                .unwrap_or(false)
        }
        "application/pdf" => content.starts_with(b"%PDF"),
        "text/plain" => std::str::from_utf8(content).is_ok(),
        "application/json" => serde_json::from_slice::<serde_json::Value>(content).is_ok(),
        "application/octet-stream" => true,
        _ => false,
    }
}

#[derive(Debug)]
pub struct AssetService {
    db: Arc<DatabaseManager>,
    storage: Arc<StorageService>,
}

impl AssetService {
    pub fn new(db: Arc<DatabaseManager>, storage: Arc<StorageService>) -> Self {
        tracing::info!("Creating new AssetsService instance");
        Self { db, storage }
    }

    pub fn from_env(db: Arc<DatabaseManager>) -> AssetResult<Self> {
        let storage = StorageService::from_env().map_err(AssetError::StorageConfig)?;
        Ok(Self::new(db, Arc::new(storage)))
    }

    pub fn storage(&self) -> &StorageService {
        &self.storage
    }

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
        tracing::info!("CreateAsset request received");

        if req.content.is_empty() {
            return Err(AssetError::EmptyContent);
        }

        if req.mime_type.is_empty() {
            return Err(AssetError::MissingMimeType);
        }

        let mime_base = req
            .mime_type
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();

        if !ALLOWED_MIME_TYPES.contains(&mime_base.as_str()) {
            return Err(AssetError::UnsupportedMimeType(req.mime_type));
        }

        if !validate_content_matches_mime(&req.content, &mime_base) {
            return Err(AssetError::MimeTypeMismatch);
        }

        let checksum_sha256 = StorageService::calculate_sha256(&req.content);
        let size_bytes = req.content.len() as i64;

        tracing::debug!(
            "Processing asset: {} bytes, SHA256: {}",
            size_bytes,
            hex::encode(&checksum_sha256)
        );

        let asset_id = Uuid::now_v7();

        let storage_uri = self
            .storage
            .upload(&user_id, &asset_id, &req.content, &req.mime_type)
            .await
            .map_err(|e| {
                tracing::error!("Failed to upload asset to storage: {}", e);
                AssetError::StorageUpload(e)
            })?;

        let metadata: Option<serde_json::Value> = req
            .metadata
            .as_ref()
            .map(|m| serde_json::from_str(m))
            .transpose()
            .map_err(AssetError::InvalidMetadata)?;

        let asset = self
            .db
            .create_asset()
            .id(asset_id)
            .user_id(user_id)
            .name(req.name)
            .checksum_sha256(checksum_sha256)
            .size_bytes(size_bytes)
            .storage_uri(storage_uri)
            .storage_backend(self.storage.get_backend_name().to_string())
            .mime_type(req.mime_type)
            .maybe_metadata(metadata)
            .call()
            .await
            .map_err(|e| {
                tracing::error!("Failed to create asset in database: {}", e);
                AssetError::DatabaseCreate(e)
            })?;

        if let Some(activity_id_str) = &req.activity_id {
            let activity_id =
                Uuid::parse_str(activity_id_str).map_err(AssetError::InvalidActivityId)?;

            self.db
                .link_asset_to_activity()
                .activity_id(activity_id)
                .asset_id(asset.id)
                .user_id(user_id)
                .call()
                .await
                .map_err(|e| {
                    tracing::error!("Failed to link asset to activity: {}", e);
                    AssetError::DatabaseLinkActivity(e)
                })?;
        }

        tracing::debug!("Created asset {} for user {}", asset.id, user_id);

        Ok(AssetResponse {
            asset: Some(Self::db_asset_to_proto(&asset)),
        })
    }
}
