use activity_core::{ActivityErrorResponse, InsertActivityRequest, InsertActivityResponse};
use asset_core::{Asset, CreateAssetRequest};
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use enum_dispatch::enum_dispatch;
use euro_auth::AuthManager;
use euro_endpoint::EndpointManager;
use reqwest::StatusCode;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use std::{io::Cursor, path::PathBuf, sync::Arc};

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

/// HTTP client wrapper used to persist activities and their assets.
///
/// Both the activity write path (`POST /activities`) and the asset write
/// path (`POST /v1/assets`) talk JSON over HTTPS. A single `AuthManager`
/// drives token refresh for both, and a single `EndpointManager` provides
/// the shared base URL so config changes flip both endpoints in lock-step.
pub struct ActivityStorage {
    endpoint_manager: Arc<EndpointManager>,
    auth_manager: AuthManager,
    http: reqwest::Client,
}

impl ActivityStorage {
    pub fn new(endpoint_manager: Arc<EndpointManager>, auth_manager: AuthManager) -> Self {
        let http = endpoint_manager.client();
        Self {
            endpoint_manager,
            auth_manager,
            http,
        }
    }

    fn url(&self, path: &str) -> reqwest::Url {
        self.endpoint_manager.url(path)
    }

    async fn bearer(&self) -> ActivityResult<String> {
        let token = self
            .auth_manager
            .get_or_refresh_access_token()
            .await
            .map_err(|e| ActivityError::network(format!("Failed to acquire access token: {e}")))?;
        Ok(format!("Bearer {}", token.expose_secret()))
    }

    pub async fn save_activity_to_service(
        &self,
        activity: &Activity,
    ) -> ActivityResult<InsertActivityResponse> {
        let icon_png_base64 = match activity.icon.as_ref() {
            Some(icon) => {
                let mut bytes: Vec<u8> = Vec::new();
                icon.write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
                    .map_err(ActivityError::Image)?;
                Some(BASE64_STANDARD.encode(&bytes))
            }
            None => None,
        };

        let request = InsertActivityRequest {
            id: None,
            name: activity.name.clone(),
            process_name: activity.process_name.clone(),
            window_title: activity.process_name.clone(),
            icon_png_base64,
            started_at: activity.start,
            ended_at: None,
        };

        let bearer = self.bearer().await?;
        let response = self
            .http
            .post(self.url("/activities"))
            .header("Authorization", bearer)
            .json(&request)
            .send()
            .await
            .map_err(|e| ActivityError::network(format!("activity request failed: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            return Err(map_http_error_response(status, response).await);
        }

        let body: InsertActivityResponse = response.json().await.map_err(|e| {
            ActivityError::network(format!("Failed to decode activity response: {e}"))
        })?;
        Ok(body)
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
        let bytes = serde_json::to_vec(asset)?;
        let file_size = bytes.len() as u64;

        let metadata = serde_json::json!({
            "asset_type": asset.get_asset_type(),
            "unique_id": asset.get_unique_id(),
            "display_name": asset.get_display_name(),
        });

        let request = CreateAssetRequest {
            name: asset.get_display_name(),
            content: BASE64_STANDARD.encode(&bytes),
            mime_type: "application/json".to_string(),
            metadata: Some(metadata),
            activity_id: None,
        };

        let bearer = self.bearer().await?;
        let response = self
            .http
            .post(self.url("/v1/assets"))
            .header("Authorization", bearer)
            .json(&request)
            .send()
            .await
            .map_err(|e| ActivityError::network(format!("asset request failed: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            return Err(map_http_error_response(status, response).await);
        }

        let created: Asset = response
            .json()
            .await
            .map_err(|e| ActivityError::network(format!("Failed to decode asset response: {e}")))?;

        tracing::debug!("Asset saved with ID: {}", created.id);

        Ok(SavedAssetInfo {
            file_path: PathBuf::from(&created.storage_uri),
            absolute_path: PathBuf::from(&created.storage_uri),
            content_hash: created.checksum_sha256,
            file_size,
            saved_at: chrono::Utc::now(),
        })
    }
}

async fn map_http_error_response(status: StatusCode, response: reqwest::Response) -> ActivityError {
    let body_text = response.text().await.unwrap_or_default();
    if let Ok(parsed) = serde_json::from_str::<ActivityErrorResponse>(&body_text) {
        ActivityError::network(format!(
            "service returned {status}: {} ({})",
            parsed.message, parsed.error
        ))
    } else {
        ActivityError::network(format!("service returned {status}: {body_text}"))
    }
}
