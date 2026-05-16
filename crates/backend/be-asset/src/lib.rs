mod error;

pub use error::{AssetError, AssetResult};

use std::sync::Arc;

use asset_core::Asset;
use be_remote_db::DatabaseManager;
use be_storage::StorageService;
use uuid::Uuid;

/// Raw bytes for an asset paired with the MIME type recorded at upload.
///
/// Returned by [`AssetService::get_asset_bytes`] so callers can drive a
/// `Content-Type` header (HTTP surface) or pick an image decoder
/// (in-process consumers) without re-querying the database.
#[derive(Debug, Clone)]
pub struct AssetBytes {
    pub bytes: Vec<u8>,
    pub mime_type: String,
}

const ALLOWED_MIME_TYPES: &[&str] = &[
    "image/png",
    "image/jpeg",
    "image/gif",
    "image/webp",
    "image/svg+xml",
    "application/pdf",
    "text/plain",
    "text/markdown",
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
                    let mut t = s.trim_start();
                    if t.starts_with("<?xml") {
                        match t.find("?>") {
                            Some(end) => t = t[end + 2..].trim_start(),
                            None => return false,
                        }
                    }
                    if t.get(..9)
                        .is_some_and(|p| p.eq_ignore_ascii_case("<!doctype"))
                    {
                        match t.find('>') {
                            Some(end) => t = t[end + 1..].trim_start(),
                            None => return false,
                        }
                    }
                    while t.starts_with("<!--") {
                        match t.find("-->") {
                            Some(end) => t = t[end + 3..].trim_start(),
                            None => return false,
                        }
                    }
                    t.get(..4).is_some_and(|p| p.eq_ignore_ascii_case("<svg"))
                })
                .unwrap_or(false)
        }
        "application/pdf" => content.starts_with(b"%PDF"),
        "text/plain" | "text/markdown" => std::str::from_utf8(content).is_ok(),
        "application/json" => serde_json::from_slice::<serde_json::Value>(content).is_ok(),
        "application/octet-stream" => true,
        _ => false,
    }
}

/// Domain input for [`AssetService::create_asset`]. Speaks in raw bytes so the
/// transport layer (HTTP, gRPC, etc.) is free to choose its own encoding.
#[derive(Debug, Clone)]
pub struct CreateAssetInput {
    pub name: String,
    pub content: Vec<u8>,
    pub mime_type: String,
    pub metadata: Option<serde_json::Value>,
    pub activity_id: Option<Uuid>,
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

    fn db_asset_to_dto(asset: be_remote_db::Asset) -> Asset {
        use base64::{Engine as _, engine::general_purpose};

        Asset {
            id: asset.id,
            name: asset.name,
            mime_type: asset.mime_type,
            size_bytes: asset.size_bytes,
            checksum_sha256: asset
                .checksum_sha256
                .as_ref()
                .map(|h| general_purpose::STANDARD.encode(h)),
            storage_uri: asset.storage_uri,
            metadata: asset.metadata,
            created_at: asset.created_at,
            updated_at: asset.updated_at,
        }
    }

    pub async fn create_asset(&self, input: CreateAssetInput, user_id: Uuid) -> AssetResult<Asset> {
        tracing::info!("CreateAsset request received");

        let CreateAssetInput {
            name,
            content,
            mime_type,
            metadata,
            activity_id,
        } = input;

        if content.is_empty() {
            return Err(AssetError::EmptyContent);
        }

        if mime_type.is_empty() {
            return Err(AssetError::MissingMimeType);
        }

        let mime_base = mime_type
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();

        if !ALLOWED_MIME_TYPES.contains(&mime_base.as_str()) {
            return Err(AssetError::UnsupportedMimeType(mime_type));
        }

        if !validate_content_matches_mime(&content, &mime_base) {
            return Err(AssetError::MimeTypeMismatch);
        }

        let checksum_sha256 = StorageService::calculate_sha256(&content);
        let size_bytes = content.len() as i64;

        tracing::debug!(
            "Processing asset: {} bytes, SHA256: {}",
            size_bytes,
            hex::encode(&checksum_sha256)
        );

        let asset_id = Uuid::now_v7();

        let storage_uri = self
            .storage
            .upload(&user_id, &asset_id, &content, &mime_type)
            .await
            .map_err(|e| {
                tracing::error!("Failed to upload asset to storage: {}", e);
                AssetError::StorageUpload(e)
            })?;

        let asset = self
            .db
            .create_asset()
            .id(asset_id)
            .user_id(user_id)
            .name(name)
            .checksum_sha256(checksum_sha256)
            .size_bytes(size_bytes)
            .storage_uri(storage_uri)
            .storage_backend(self.storage.get_backend_name().to_string())
            .mime_type(mime_type)
            .maybe_metadata(metadata)
            .call()
            .await
            .map_err(|e| {
                tracing::error!("Failed to create asset in database: {}", e);
                AssetError::DatabaseCreate(e)
            })?;

        if let Some(activity_id) = activity_id {
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

        tracing::debug!("Created asset {}", asset.id);

        Ok(Self::db_asset_to_dto(asset))
    }

    /// Read an asset's raw bytes scoped to its owner.
    ///
    /// The `user_id` predicate is enforced inside the DB query
    /// (`get_asset_for_user`), so attempting to read another user's asset
    /// surfaces as [`AssetError::NotFound`] — never as a permission error
    /// that would leak the asset's existence. Bytes are pulled through the
    /// `StorageService`, which dispatches to the configured backend
    /// (filesystem in dev, S3 in prod) and transparently decrypts when the
    /// `encryption` feature is enabled.
    pub async fn get_asset_bytes(&self, asset_id: Uuid, user_id: Uuid) -> AssetResult<AssetBytes> {
        let asset = self
            .db
            .get_asset_for_user()
            .asset_id(asset_id)
            .user_id(user_id)
            .call()
            .await
            .map_err(|e| {
                if e.is_not_found() {
                    AssetError::NotFound
                } else {
                    AssetError::DatabaseRead(e)
                }
            })?;

        let bytes = self
            .storage
            .download(&asset.storage_uri)
            .await
            .map_err(|e| {
                if e.is_not_found() {
                    // The DB row points at storage that no longer holds
                    // the blob — surface as `NotFound` so HTTP callers
                    // render a clean 404 instead of a 500.
                    AssetError::NotFound
                } else {
                    AssetError::StorageDownload(e)
                }
            })?;

        Ok(AssetBytes {
            bytes,
            mime_type: asset.mime_type,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_markdown_is_allowed() {
        assert!(ALLOWED_MIME_TYPES.contains(&"text/markdown"));
    }

    #[test]
    fn text_markdown_accepts_utf8_content() {
        assert!(validate_content_matches_mime(
            b"# Heading\n\nBody with *emphasis*.",
            "text/markdown"
        ));
        assert!(validate_content_matches_mime(b"", "text/markdown"));
    }

    #[test]
    fn text_markdown_rejects_invalid_utf8() {
        // Lone continuation byte — never valid UTF-8.
        assert!(!validate_content_matches_mime(&[0x80], "text/markdown"));
    }
}
