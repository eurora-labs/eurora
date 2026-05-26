//! Shared wire types for the Eurora activity HTTP service.
//!
//! This crate is the single source of truth for the JSON contract between
//! `be-activity-service` (Axum) and `euro-activity` (reqwest), and is also
//! the input to the TypeScript bindings emitted by the workspace-level
//! `euro-codegen` orchestrator (`pnpm specta`).
//!
//! The data model has two tiers:
//!
//! * [`Activity`] is the **parent** — a stable per-user identity for an
//!   app or site, uniquely keyed by `(user_id, identity_key)`. The same
//!   parent row receives every visit to the same domain or process.
//! * [`ActivitySession`] is the **child** — one time-windowed focus run
//!   with its own start/end and per-visit `window_title` / `url`.
//!
//! The desktop never POSTs a parent directly: every `POST
//! /activity-sessions` carries an [`ActivityInsert`] alongside the
//! session payload, the server upserts the parent by identity, and the
//! response embeds both rows so the client can prepend a rail item and
//! show "live now" without a second round-trip.
//!
//! Types are pure data with `serde` derives; the optional `specta` feature
//! adds `specta::Type` so the same definitions can be re-exported as TS.
//! No HTTP, database, or gRPC dependencies live here on purpose — pulling
//! this crate into a leaf binary must not drag in transport plumbing.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "specta")]
use specta::Type;

/// A persisted parent activity as returned to the client.
///
/// One row per `(user_id, identity_key)` — `identity_key` is a lowercased
/// process name (default strategy) or a base domain label (browser
/// strategy, with the public suffix stripped). `last_used_at` advances on
/// every new session insert and on each session's closing PATCH so the
/// rail can sort by recency without scanning the children table.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct Activity {
    pub id: Uuid,
    pub user_id: Uuid,
    pub identity_key: String,
    pub display_name: String,
    pub icon_asset_id: Option<Uuid>,
    pub last_used_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// One persisted focus run.
///
/// `started_at` is client-supplied (the desktop owns wall-clock for the
/// focus event); `ended_at` is ratcheted forward by heartbeat PATCHes and
/// finalised on `Stopping`. `url` and `window_title` are optional because
/// not every strategy produces them (the default strategy has no URL; an
/// extension-less browser tab has no title yet).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct ActivitySession {
    pub id: Uuid,
    pub activity_id: Uuid,
    pub process_name: String,
    pub process_id: Option<i32>,
    pub window_title: Option<String>,
    pub url: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Identity-portion of a session insert.
///
/// Travels alongside the session in [`InsertActivitySessionRequest`] so
/// the server can upsert the parent by `(user_id, identity_key)` in the
/// same transaction as the session insert. `display_name` is *set-once*
/// on the server: subsequent inserts that arrive with a different value
/// are ignored, so a future rename endpoint is the only thing that ever
/// mutates it. `icon_png_base64` carries the icon as standard-base64
/// (JSON-friendly, well under the monolith's body limit at app-launcher
/// sizes).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct ActivityInsert {
    pub identity_key: String,
    pub display_name: String,
    #[serde(default)]
    pub icon_png_base64: Option<String>,
}

/// Request body for `POST /activity-sessions`.
///
/// `session_id` and `ended_at` are sent at insert time so a subsequent
/// PATCH targets the same row (idempotent retries / heartbeat) and an
/// unexpected crash before the first heartbeat still leaves a bounded
/// `ended_at` instead of `NULL`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct InsertActivitySessionRequest {
    #[serde(default)]
    pub session_id: Option<Uuid>,
    pub activity: ActivityInsert,
    pub process_name: String,
    #[serde(default)]
    pub process_id: Option<i32>,
    #[serde(default)]
    pub window_title: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    pub started_at: DateTime<Utc>,
    #[serde(default)]
    pub ended_at: Option<DateTime<Utc>>,
}

/// Response body for `POST /activity-sessions`.
///
/// Always carries both rows: the parent (which may have been freshly
/// upserted or merely bumped) and the newly-inserted session. The client
/// hands both to the timeline rail as a single atomic update so the rail
/// never has to render a parent without knowing whether it is "live".
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct InsertActivitySessionResponse {
    pub activity: Activity,
    pub session: ActivitySession,
}

/// Request body for `PATCH /activity-sessions/{id}`.
///
/// All fields are optional; missing fields are left untouched on the row.
/// Used by the desktop client to (a) ratchet `ended_at` forward on every
/// heartbeat tick and at session transitions, and (b) update
/// `window_title` / `url` when an intra-domain SPA navigation reports a
/// title-only change.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct UpdateActivitySessionRequest {
    #[serde(default)]
    pub window_title: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub ended_at: Option<DateTime<Utc>>,
}

/// Response body for `PATCH /activity-sessions/{id}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct UpdateActivitySessionResponse {
    pub session: ActivitySession,
}

/// Default page size when the client omits `limit`.
pub const DEFAULT_LIST_LIMIT: u32 = 20;

/// Maximum page size the server will accept. Larger values are rejected
/// with `400 Bad Request` so the contract is explicit; clients should
/// paginate rather than rely on silent clamping.
pub const MAX_LIST_LIMIT: u32 = 100;

/// Query parameters for `GET /activities` and `GET /activities/{id}/sessions`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct ListActivitiesQuery {
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub offset: Option<u32>,
}

/// One element of [`ListActivitiesResponse`].
///
/// Embeds the most recent session inline so the rail can decide "is this
/// activity live right now?" (`latest_session.ended_at IS NULL`) without
/// a second fetch. `latest_session` is `None` only for parents that have
/// no sessions yet — a transient state that should never appear in
/// practice but is allowed to keep the wire shape monotonic under
/// concurrent inserts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct ActivityWithLatestSession {
    #[serde(flatten)]
    pub activity: Activity,
    pub latest_session: Option<ActivitySession>,
}

/// Response body for `GET /activities`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct ListActivitiesResponse {
    pub activities: Vec<ActivityWithLatestSession>,
}

/// Response body for `GET /activities/{id}/sessions`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct ListActivitySessionsResponse {
    pub sessions: Vec<ActivitySession>,
}

/// JSON error body returned by the activity service on non-2xx responses.
///
/// Mirrors the shape used by `be-update-service` so the desktop client
/// can decode failures uniformly across HTTP services.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct ActivityErrorResponse {
    pub error: String,
    pub message: String,
    #[serde(default)]
    pub details: Option<String>,
}

/// Build a [`specta::Types`] containing every activity wire type the
/// desktop app needs. Used by the codegen binary to emit `activity.ts`.
#[cfg(feature = "specta")]
pub fn type_collection() -> specta::Types {
    specta::Types::default()
        .register::<Activity>()
        .register::<ActivitySession>()
        .register::<ActivityInsert>()
        .register::<InsertActivitySessionRequest>()
        .register::<InsertActivitySessionResponse>()
        .register::<UpdateActivitySessionRequest>()
        .register::<UpdateActivitySessionResponse>()
        .register::<ListActivitiesQuery>()
        .register::<ActivityWithLatestSession>()
        .register::<ListActivitiesResponse>()
        .register::<ListActivitySessionsResponse>()
        .register::<ActivityErrorResponse>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_request_round_trips_with_optional_fields_null() {
        let req = InsertActivitySessionRequest {
            session_id: None,
            activity: ActivityInsert {
                identity_key: "youtube".into(),
                display_name: "Youtube".into(),
                icon_png_base64: None,
            },
            process_name: "chrome".into(),
            process_id: Some(42),
            window_title: None,
            url: Some("https://youtube.com/watch?v=abc".into()),
            started_at: Utc::now(),
            ended_at: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"identity_key\":\"youtube\""));
        assert!(json.contains("\"icon_png_base64\":null"));
        assert!(json.contains("\"window_title\":null"));
        let back: InsertActivitySessionRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.activity.identity_key, req.activity.identity_key);
        assert_eq!(back.activity.display_name, req.activity.display_name);
        assert_eq!(back.process_id, Some(42));
        assert!(back.window_title.is_none());
        assert!(back.session_id.is_none());
    }

    #[test]
    fn insert_request_decodes_with_missing_optional_fields() {
        // Forward-compat: a payload that omits optionals still parses.
        let json = r#"{
            "activity": {"identity_key": "code", "display_name": "Code"},
            "process_name": "code",
            "started_at": "2026-01-01T00:00:00Z"
        }"#;
        let back: InsertActivitySessionRequest = serde_json::from_str(json).unwrap();
        assert!(back.session_id.is_none());
        assert!(back.activity.icon_png_base64.is_none());
        assert!(back.window_title.is_none());
        assert!(back.url.is_none());
        assert!(back.process_id.is_none());
        assert!(back.ended_at.is_none());
    }

    #[test]
    fn list_query_defaults_to_all_none() {
        let q: ListActivitiesQuery = serde_json::from_str("{}").unwrap();
        assert!(q.limit.is_none());
        assert!(q.offset.is_none());
    }

    #[test]
    fn update_request_round_trips_with_partial_fields() {
        let req = UpdateActivitySessionRequest {
            window_title: None,
            url: None,
            ended_at: Some(Utc::now()),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"window_title\":null"));
        assert!(json.contains("\"url\":null"));
        assert!(json.contains("\"ended_at\":"));
        let back: UpdateActivitySessionRequest = serde_json::from_str(&json).unwrap();
        assert!(back.window_title.is_none());
        assert!(back.url.is_none());
        assert!(back.ended_at.is_some());
    }

    #[test]
    fn update_request_decodes_with_only_ended_at() {
        // Heartbeat path: PATCH body contains only `ended_at`.
        let json = r#"{"ended_at":"2026-01-15T12:00:00Z"}"#;
        let back: UpdateActivitySessionRequest = serde_json::from_str(json).unwrap();
        assert!(back.window_title.is_none());
        assert!(back.url.is_none());
        assert!(back.ended_at.is_some());
    }

    #[test]
    fn update_request_decodes_empty_body_as_all_none() {
        // The handler rejects all-None at the application layer; the
        // wire type itself must still parse so that error path is
        // reachable.
        let back: UpdateActivitySessionRequest = serde_json::from_str("{}").unwrap();
        assert!(back.window_title.is_none());
        assert!(back.url.is_none());
        assert!(back.ended_at.is_none());
    }

    #[test]
    fn list_response_round_trips_with_embedded_session() {
        let activity = Activity {
            id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            identity_key: "youtube".into(),
            display_name: "Youtube".into(),
            icon_asset_id: None,
            last_used_at: Utc::now(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let session = ActivitySession {
            id: Uuid::now_v7(),
            activity_id: activity.id,
            process_name: "chrome".into(),
            process_id: Some(99),
            window_title: Some("Great Video".into()),
            url: Some("https://youtube.com/watch?v=abc".into()),
            started_at: Utc::now(),
            ended_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let resp = ListActivitiesResponse {
            activities: vec![ActivityWithLatestSession {
                activity: activity.clone(),
                latest_session: Some(session.clone()),
            }],
        };
        let json = serde_json::to_string(&resp).unwrap();
        // `#[serde(flatten)]` on `activity` means parent fields appear
        // alongside `latest_session` rather than under an `activity` key.
        assert!(json.contains("\"identity_key\":\"youtube\""));
        assert!(json.contains("\"latest_session\":"));
        let back: ListActivitiesResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(back.activities.len(), 1);
        assert_eq!(back.activities[0].activity.identity_key, "youtube");
        assert_eq!(
            back.activities[0]
                .latest_session
                .as_ref()
                .map(|s| s.process_name.as_str()),
            Some("chrome")
        );
    }

    #[cfg(feature = "specta")]
    #[test]
    fn type_collection_contains_all_wire_types() {
        let types = type_collection();
        let names: Vec<String> = types
            .into_unsorted_iter()
            .map(|ndt| ndt.name.to_string())
            .collect();
        for expected in [
            "Activity",
            "ActivitySession",
            "ActivityInsert",
            "InsertActivitySessionRequest",
            "InsertActivitySessionResponse",
            "UpdateActivitySessionRequest",
            "UpdateActivitySessionResponse",
            "ListActivitiesQuery",
            "ActivityWithLatestSession",
            "ListActivitiesResponse",
            "ListActivitySessionsResponse",
            "ActivityErrorResponse",
        ] {
            assert!(
                names.iter().any(|n| n == expected),
                "missing {expected} from collection: {names:?}"
            );
        }
    }
}
