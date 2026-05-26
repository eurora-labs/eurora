//! Persisted-activity surface exposed to the desktop frontend.
//!
//! Three responsibilities live here:
//!
//! - [`activity_list`] — the one-shot fetch the rail uses on mount to
//!   hydrate from the most recent persisted activities (parents, with
//!   their latest session embedded inline). Bridges the
//!   `GET /activities` HTTP endpoint and the `GET /v1/assets/{id}` icon
//!   endpoint, decorating each row with a precomputed [`AccentColor`]
//!   and a `data:` URL the frontend can drop straight into `<img src>`.
//! - [`SavedActivityUpserted`] — the tauri-specta event the persist
//!   path emits *after* a successful `POST /activity-sessions`. Carries
//!   the (possibly upserted) parent activity and the new live session
//!   atomically, so the rail can update both the row position and the
//!   live indicator in one transaction.
//! - [`SavedActivityLiveSessionEnded`] — the tauri-specta event the
//!   persist path emits *after* a successful closing PATCH on the live
//!   session's `ended_at`. Lets the rail strip the live indicator from
//!   the matching parent row without a re-fetch.
//!
//! The frontend wire shape ([`SavedActivity`]) is intentionally
//! distinct from `activity_core::Activity`: that one is the JSON-HTTP
//! contract between the desktop and the backend, this one is the
//! tauri-specta-typed presentation DTO. Keeping them separate means the
//! rail can carry precomputed accent / icon-data-URL fields without
//! polluting the network type.

use std::sync::Arc;

use activity_core::{
    Activity as WireActivity, ActivitySession as WireActivitySession, ActivityWithLatestSession,
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use chrono::{DateTime, Utc};
use euro_activity::ActivityStorage;
use euro_timeline::TimelineManager;
use futures::future;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};
use tauri_specta::Event;
use thiserror::Error;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::procedures::accent::{accent_from_image, decode_image};
use crate::procedures::timeline::AccentColor;

/// Frontend-facing view of one persisted parent activity, with its
/// most recent session embedded inline.
///
/// `accent` and `icon_base64` are populated by the desktop tauri layer
/// (decoded from the asset's PNG bytes); both are `None` whenever the
/// activity has no icon or the icon fetch failed — treat them as
/// presentation hints, never as load-bearing fields.
///
/// `live_session` carries the most recent session and is the only
/// signal the rail uses to render "live now": when
/// `live_session.ended_at` is `None` the activity is currently in use.
/// The desktop emits an [`SavedActivityLiveSessionEnded`] event when
/// the active session closes, so the rail can flip the indicator
/// without re-fetching.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SavedActivity {
    pub id: Uuid,
    pub identity_key: String,
    pub display_name: String,
    pub last_used_at: DateTime<Utc>,
    pub accent: Option<AccentColor>,
    /// `data:<mime>;base64,...` URL suitable for direct embedding in
    /// `<img src>`. Bare base64 would force the frontend to know the
    /// mime out-of-band, which it currently does not.
    pub icon_base64: Option<String>,
    pub live_session: Option<SavedActivitySession>,
}

/// Frontend-facing view of one persisted activity session.
///
/// A subset of `activity_core::ActivitySession` — the rail only needs
/// the bits that drive the live indicator and per-tab labelling, not
/// the full audit columns (`created_at` / `updated_at` are server
/// bookkeeping and never surface).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SavedActivitySession {
    pub id: Uuid,
    pub activity_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub window_title: Option<String>,
    pub url: Option<String>,
}

/// Push event fired after the cloud `POST /activity-sessions` succeeds.
///
/// Carries the (possibly upserted) parent activity *and* the new
/// session it created, so the desktop frontend can prepend the row to
/// the timeline rail with the right "live" indicator without re-polling
/// `GET /activities`.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct SavedActivityUpserted(pub SavedActivity);

/// Push event fired after the cloud closing PATCH of `ended_at`
/// succeeds for the live session of a parent activity.
///
/// Payload is intentionally minimal: the frontend already has the
/// parent row in memory, so only the parent id, the session id, and
/// the now-known end timestamp are shipped over the wire.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct SavedActivityLiveSessionEnded {
    pub activity_id: Uuid,
    pub session_id: Uuid,
    pub ended_at: DateTime<Utc>,
}

/// Errors surfaced to the frontend from [`activity_list`].
///
/// Externally tagged so the JS side gets `{ type: "Network", data: "..." }`
/// and can branch on `type` rather than parsing strings. Variants are
/// intentionally narrow — any failure to fetch a *single* icon falls back
/// to `accent: None, icon_base64: None` on that row instead of failing
/// the whole call.
#[derive(Debug, Error, Serialize, Type)]
#[serde(tag = "type", content = "data")]
pub enum SavedActivityError {
    #[error("state unavailable: {0}")]
    StateUnavailable(&'static str),
    #[error("network: {0}")]
    Network(String),
}

impl From<euro_activity::ActivityError> for SavedActivityError {
    fn from(err: euro_activity::ActivityError) -> Self {
        Self::Network(err.to_string())
    }
}

/// Fetch the most-recent persisted activities and decorate each with
/// the presentation data the timeline rail needs (accent colour +
/// `data:`-URL icon + embedded live session).
///
/// Per-row icon fetches fan out concurrently via `join_all`; reqwest's
/// HTTP/2 pool multiplexes them over a single connection. Failures on
/// a single icon log + degrade to `(accent: None, icon_base64: None)`
/// so one bad asset can't block the rest of the page from rendering.
#[tauri::command]
#[specta::specta]
pub async fn activity_list(
    app_handle: AppHandle,
    limit: u32,
    offset: u32,
) -> Result<Vec<SavedActivity>, SavedActivityError> {
    let activity_storage = activity_storage(&app_handle).await?;

    let rows = activity_storage.list_activities(limit, offset).await?;

    let storage: &ActivityStorage = &activity_storage;
    let enriched = future::join_all(rows.into_iter().map(|row| enrich_row(storage, row))).await;

    Ok(enriched)
}

async fn enrich_row(storage: &ActivityStorage, row: ActivityWithLatestSession) -> SavedActivity {
    let (accent, icon_base64) = match row.activity.icon_asset_id {
        Some(asset_id) => fetch_icon_assets(storage, asset_id).await,
        None => (None, None),
    };

    saved_activity_from_parts(row.activity, row.latest_session, accent, icon_base64)
}

/// Assemble a `SavedActivity` from the persisted wire types plus the
/// already-resolved accent / icon. Shared between [`activity_list`]
/// (which fans out a fresh icon fetch per row) and the push-event
/// path in `main.rs` (which already has the icon bytes in hand from
/// the strategy that just produced them).
pub fn saved_activity_from_parts(
    activity: WireActivity,
    latest_session: Option<WireActivitySession>,
    accent: Option<AccentColor>,
    icon_base64: Option<String>,
) -> SavedActivity {
    SavedActivity {
        id: activity.id,
        identity_key: activity.identity_key,
        display_name: activity.display_name,
        last_used_at: activity.last_used_at,
        accent,
        icon_base64,
        live_session: latest_session.map(|s| SavedActivitySession {
            id: s.id,
            activity_id: s.activity_id,
            started_at: s.started_at,
            ended_at: s.ended_at,
            window_title: s.window_title,
            url: s.url,
        }),
    }
}

/// Fetch one icon and project it into the two presentation fields
/// (`accent`, `icon_base64`). Errors degrade to `(None, None)` and a
/// `warn!` log — a missing icon must not poison the rail.
///
/// The PNG bytes are decoded exactly once: the decoded [`RgbaImage`]
/// feeds the accent classifier, and the original bytes (not a re-encode
/// of the decoded image) feed the `data:` URL.
async fn fetch_icon_assets(
    storage: &ActivityStorage,
    asset_id: Uuid,
) -> (Option<AccentColor>, Option<String>) {
    match storage.fetch_asset_bytes(asset_id).await {
        Ok(Some((bytes, mime_type))) => {
            let accent = decode_image(&bytes).as_ref().and_then(accent_from_image);
            let icon_base64 = Some(format!(
                "data:{};base64,{}",
                mime_type,
                BASE64_STANDARD.encode(&bytes)
            ));
            (accent, icon_base64)
        }
        Ok(None) => {
            tracing::debug!(asset_id = %asset_id, "Icon asset not found (404)");
            (None, None)
        }
        Err(err) => {
            tracing::warn!(asset_id = %asset_id, error = %err, "Failed to fetch icon asset");
            (None, None)
        }
    }
}

async fn activity_storage(
    app_handle: &AppHandle,
) -> Result<Arc<ActivityStorage>, SavedActivityError> {
    let timeline_state: tauri::State<'_, Mutex<TimelineManager>> = app_handle
        .try_state()
        .ok_or(SavedActivityError::StateUnavailable("timeline"))?;
    let timeline = timeline_state.lock().await;
    Ok(Arc::clone(&timeline.activity_storage))
}
