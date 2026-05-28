use activity_core::{
    ActivityErrorResponse, ActivityInsert, ActivityWithLatestSession, InsertActivitySessionRequest,
    InsertActivitySessionResponse, ListActivitiesResponse, UpdateActivitySessionRequest,
    UpdateActivitySessionResponse,
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use chrono::{DateTime, Utc};
use euro_auth::AuthManager;
use euro_endpoint::EndpointManager;
use reqwest::StatusCode;
use secrecy::ExposeSecret;
use std::{io::Cursor, sync::Arc};
use uuid::Uuid;

use crate::{ActivityError, ActivitySession, error::ActivityResult};

/// HTTP client wrapper used to persist activity sessions.
///
/// The session write path (`POST /activity-sessions`) speaks JSON over
/// HTTPS. The backend's transaction upserts the parent activity by
/// `(user_id, identity_key)` *and* inserts the child session in the same
/// round trip; the response carries both rows so the caller can update
/// the rail without a follow-up `GET /activities`.
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

    /// Insert one session against its parent activity.
    ///
    /// The client-supplied session id flows through unchanged so a
    /// subsequent PATCH (heartbeat-style end ratchet or final close)
    /// targets the same row even after a retry. The session is sent
    /// *without* `ended_at`: the parent's `ended_at IS NULL` invariant
    /// is the rail's live indicator; the backend bumps `last_used_at`
    /// when the session closes for real.
    pub async fn save_session_to_service(
        &self,
        session: &ActivitySession,
    ) -> ActivityResult<InsertActivitySessionResponse> {
        let icon_png_base64 = match session.icon.as_ref() {
            Some(icon) => {
                let mut bytes: Vec<u8> = Vec::new();
                icon.write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
                    .map_err(ActivityError::Image)?;
                Some(BASE64_STANDARD.encode(&bytes))
            }
            None => None,
        };

        let request = InsertActivitySessionRequest {
            session_id: Some(session.id),
            activity: ActivityInsert {
                identity_key: session.activity.key.clone(),
                display_name: session.activity.display_name.clone(),
                icon_png_base64,
            },
            process_name: session.process_name.clone(),
            process_id: Some(session.process_id as i32),
            window_title: session.window_title.clone(),
            url: session.url.as_ref().map(|u| u.to_string()),
            started_at: session.started_at,
            ended_at: session.ended_at,
        };

        let bearer = self.bearer().await?;
        let response = self
            .http
            .post(self.url("/activity-sessions"))
            .header("Authorization", bearer)
            .json(&request)
            .send()
            .await
            .map_err(|e| ActivityError::network(format!("activity session request failed: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            return Err(map_http_error_response(status, response).await);
        }

        let body: InsertActivitySessionResponse = response.json().await.map_err(|e| {
            ActivityError::network(format!("Failed to decode activity session response: {e}"))
        })?;
        Ok(body)
    }

    /// Fetch the most-recent persisted parent activities (and the latest
    /// session for each) for the authenticated user.
    ///
    /// The server returns rows ordered by `last_used_at DESC`, capped at
    /// the service-side maximum (`activity_core::MAX_LIST_LIMIT`).
    /// Pagination parameters are forwarded verbatim — invalid values
    /// surface as a network error carrying the server's typed
    /// [`ActivityErrorResponse`] body.
    pub async fn list_activities(
        &self,
        limit: u32,
        offset: u32,
    ) -> ActivityResult<Vec<ActivityWithLatestSession>> {
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
    /// `None` indicates a clean 404 (the asset does not exist, or is
    /// not owned by the authenticated user — the backend deliberately
    /// conflates the two so foreign ids can't be probed). Any other
    /// non-success status surfaces as [`ActivityError::Network`]. The
    /// returned MIME type mirrors the value recorded at upload time.
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

    /// PATCH a session's `ended_at` on the backend.
    ///
    /// Used by the collector when a new session arrives (closes the
    /// previous one with its real end time) and during graceful
    /// shutdown. The first NULL→set transition on the server also bumps
    /// the parent's `last_used_at`, so a long-lived session that's just
    /// closed keeps its parent fresh at the top of the rail.
    pub async fn update_session_end(
        &self,
        session_id: Uuid,
        ended_at: DateTime<Utc>,
    ) -> ActivityResult<UpdateActivitySessionResponse> {
        let request = UpdateActivitySessionRequest {
            window_title: None,
            url: None,
            ended_at: Some(ended_at),
        };
        self.patch_session(session_id, &request).await
    }

    /// PATCH a session's `window_title` (and optionally `url`) without
    /// closing it.
    ///
    /// Fired when a browser strategy reports a title-only update inside
    /// the same base domain (SPA route change) — the parent stays put
    /// and we keep the rail in sync with the live tab. `url` is only
    /// set when the strategy also passes the new URL alongside the
    /// title update.
    pub async fn update_session_title(
        &self,
        session_id: Uuid,
        window_title: String,
        url: Option<String>,
    ) -> ActivityResult<UpdateActivitySessionResponse> {
        let request = UpdateActivitySessionRequest {
            window_title: Some(window_title),
            url,
            ended_at: None,
        };
        self.patch_session(session_id, &request).await
    }

    async fn patch_session(
        &self,
        session_id: Uuid,
        request: &UpdateActivitySessionRequest,
    ) -> ActivityResult<UpdateActivitySessionResponse> {
        let bearer = self.bearer().await?;
        let response = self
            .http
            .patch(self.url(&format!("/activity-sessions/{session_id}")))
            .header("Authorization", bearer)
            .json(request)
            .send()
            .await
            .map_err(|e| ActivityError::network(format!("activity session patch failed: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            return Err(map_http_error_response(status, response).await);
        }

        let body: UpdateActivitySessionResponse = response.json().await.map_err(|e| {
            ActivityError::network(format!(
                "Failed to decode activity session patch response: {e}"
            ))
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
