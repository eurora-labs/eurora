use activity_core::{
    Activity as WireActivity, ActivityErrorResponse, InsertActivityRequest, InsertActivityResponse,
    ListActivitiesResponse, UpdateActivityRequest, UpdateActivityResponse,
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use chrono::{DateTime, Utc};
use euro_auth::AuthManager;
use euro_endpoint::EndpointManager;
use reqwest::StatusCode;
use secrecy::ExposeSecret;
use std::{io::Cursor, sync::Arc};
use uuid::Uuid;

use crate::{Activity, ActivityError, error::ActivityResult};

/// HTTP client wrapper used to persist activities.
///
/// The activity write path (`POST /activities`) speaks JSON over HTTPS.
/// A single [`AuthManager`] drives token refresh and an [`EndpointManager`]
/// provides the base URL so config changes flip endpoints in lock-step.
///
/// Asset persistence was removed alongside the bundled-context channel —
/// the LLM pulls page contents through granular tools per turn, so there
/// is no longer a server-side asset store fed by this client.
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

    /// Create the activity on the backend.
    ///
    /// The client-supplied `id` and `ended_at` are sent in the same request so
    /// that a subsequent PATCH targets the same row (idempotent retries / heartbeat),
    /// and so an unexpected crash before the first heartbeat still leaves a
    /// row with a bounded `ended_at` instead of `NULL`.
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
            id: Some(activity.id),
            name: activity.name.clone(),
            process_name: activity.process_name.clone(),
            window_title: activity.window_title(),
            icon_png_base64,
            started_at: activity.start,
            ended_at: Some(activity.end.unwrap_or_else(Utc::now)),
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

    /// Fetch the most recent persisted activities for the authenticated user.
    ///
    /// The server returns rows in `started_at DESC` order, capped at the
    /// service-side maximum (`activity_core::MAX_LIST_LIMIT`). Pagination
    /// parameters are forwarded verbatim — invalid values surface as a
    /// network error carrying the server's typed `ActivityErrorResponse`
    /// body.
    pub async fn list_activities(
        &self,
        limit: u32,
        offset: u32,
    ) -> ActivityResult<Vec<WireActivity>> {
        let bearer = self.bearer().await?;
        let response = self
            .http
            .get(self.url("/activities"))
            .header("Authorization", bearer)
            .query(&[("limit", limit), ("offset", offset)])
            .send()
            .await
            .map_err(|e| ActivityError::network(format!("activity list request failed: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            return Err(map_http_error_response(status, response).await);
        }

        let body: ListActivitiesResponse = response.json().await.map_err(|e| {
            ActivityError::network(format!("Failed to decode activity list response: {e}"))
        })?;
        Ok(body.activities)
    }

    /// Fetch the raw bytes for an asset by id.
    ///
    /// `None` indicates a clean 404 (the asset does not exist, or is not
    /// owned by the authenticated user — the backend deliberately conflates
    /// the two so foreign ids can't be probed). Any other non-success
    /// status surfaces as [`ActivityError::Network`]. The returned MIME
    /// type mirrors the value recorded at upload time.
    ///
    /// Kept around for historical assets that were uploaded prior to the
    /// bundled-context channel's removal; the desktop no longer writes new
    /// assets through this client.
    pub async fn fetch_asset_bytes(
        &self,
        asset_id: Uuid,
    ) -> ActivityResult<Option<(Vec<u8>, String)>> {
        let bearer = self.bearer().await?;
        let response = self
            .http
            .get(self.url(&format!("/v1/assets/{asset_id}")))
            .header("Authorization", bearer)
            .send()
            .await
            .map_err(|e| ActivityError::network(format!("asset fetch request failed: {e}")))?;

        let status = response.status();
        if status == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !status.is_success() {
            return Err(map_http_error_response(status, response).await);
        }

        let mime_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_owned())
            .unwrap_or_else(|| "application/octet-stream".to_string());

        let bytes = response
            .bytes()
            .await
            .map_err(|e| ActivityError::network(format!("Failed to read asset bytes: {e}")))?;

        Ok(Some((bytes.to_vec(), mime_type)))
    }

    /// PATCH the activity's `ended_at` on the backend.
    ///
    /// Used both for heartbeat ticks (best-known end so far) and for real
    /// transitions (`Stopping` / `NewActivity` overrides the previous). The
    /// server-side update is idempotent and partial: only `ended_at` is set.
    pub async fn update_activity_end(
        &self,
        id: Uuid,
        ended_at: DateTime<Utc>,
    ) -> ActivityResult<UpdateActivityResponse> {
        let request = UpdateActivityRequest {
            name: None,
            window_title: None,
            ended_at: Some(ended_at),
        };
        self.patch_activity(id, &request).await
    }

    /// PATCH the activity's `window_title` on the backend.
    ///
    /// Fired when a browser strategy reports a title-only update without a
    /// new activity (e.g. SPA route change inside the same domain).
    pub async fn update_activity_title(
        &self,
        id: Uuid,
        window_title: String,
    ) -> ActivityResult<UpdateActivityResponse> {
        let request = UpdateActivityRequest {
            name: None,
            window_title: Some(window_title),
            ended_at: None,
        };
        self.patch_activity(id, &request).await
    }

    async fn patch_activity(
        &self,
        id: Uuid,
        request: &UpdateActivityRequest,
    ) -> ActivityResult<UpdateActivityResponse> {
        let bearer = self.bearer().await?;
        let response = self
            .http
            .patch(self.url(&format!("/activities/{id}")))
            .header("Authorization", bearer)
            .json(request)
            .send()
            .await
            .map_err(|e| ActivityError::network(format!("activity patch failed: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            return Err(map_http_error_response(status, response).await);
        }

        let body: UpdateActivityResponse = response.json().await.map_err(|e| {
            ActivityError::network(format!("Failed to decode activity patch response: {e}"))
        })?;
        Ok(body)
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
