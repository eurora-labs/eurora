use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
#[cfg(feature = "specta")]
use specta::Type;
#[cfg(feature = "specta")]
use specta_typescript::{BigInt, Unknown};
use uuid::Uuid;

/// Canonical asset record returned from the asset service.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct Asset {
    pub id: Uuid,
    pub name: String,
    pub mime_type: String,
    #[cfg_attr(feature = "specta", specta(type = Option<BigInt>))]
    pub size_bytes: Option<i64>,
    /// SHA-256 of the asset content, base64 (standard alphabet)-encoded.
    pub checksum_sha256: Option<String>,
    pub storage_uri: String,
    #[cfg_attr(feature = "specta", specta(type = Unknown))]
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request body for `POST /v1/assets`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct CreateAssetRequest {
    pub name: String,
    /// Asset bytes, base64 (standard alphabet)-encoded.
    pub content: String,
    pub mime_type: String,
    #[serde(default)]
    #[cfg_attr(feature = "specta", specta(type = Option<Unknown>))]
    pub metadata: Option<serde_json::Value>,
    #[serde(default)]
    pub activity_id: Option<Uuid>,
}
