//! Shared wire types for the Eurora activity HTTP service.
//!
//! This crate is the single source of truth for the JSON contract between
//! `be-activity-service` (Axum) and `euro-activity` (reqwest), and is also
//! the input to the TypeScript bindings emitted by the workspace-level
//! `euro-api-codegen` orchestrator (`pnpm specta:backend`).
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

/// A persisted activity row as returned to the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct Activity {
    pub id: Uuid,
    pub name: String,
    pub process_name: String,
    pub window_title: String,
    pub icon_asset_id: Option<Uuid>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request body for `POST /activities`.
///
/// `icon_png_base64` carries an optional PNG icon as standard-base64. We
/// take JSON-with-base64 instead of `multipart/form-data` so the wire shape
/// stays specta-friendly; in practice icons are app-launcher size (a few
/// hundred bytes to ~50 KB), well under the monolith's 2 MiB body limit.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct InsertActivityRequest {
    #[serde(default)]
    pub id: Option<Uuid>,
    pub name: String,
    pub process_name: String,
    pub window_title: String,
    #[serde(default)]
    pub icon_png_base64: Option<String>,
    pub started_at: DateTime<Utc>,
    #[serde(default)]
    pub ended_at: Option<DateTime<Utc>>,
}

/// Response body for `POST /activities`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct InsertActivityResponse {
    pub activity: Activity,
}

/// Request body for `PATCH /activities/{id}`.
///
/// All fields are optional; missing fields are left untouched on the row.
/// Used by the desktop client to (a) ratchet `ended_at` forward on every
/// heartbeat tick and at activity transitions, and (b) correct
/// `window_title` when a browser strategy reports a title-only update.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct UpdateActivityRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub window_title: Option<String>,
    #[serde(default)]
    pub ended_at: Option<DateTime<Utc>>,
}

/// Response body for `PATCH /activities/{id}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct UpdateActivityResponse {
    pub activity: Activity,
}

/// Default page size when the client omits `limit`.
pub const DEFAULT_LIST_LIMIT: u32 = 20;

/// Maximum page size the server will accept. Larger values are rejected with
/// `400 Bad Request` so the contract is explicit; clients should paginate
/// rather than rely on silent clamping.
pub const MAX_LIST_LIMIT: u32 = 100;

/// Query parameters for `GET /activities`.
///
/// Both fields are optional. When `limit` is omitted the server falls back to
/// [`DEFAULT_LIST_LIMIT`]; values greater than [`MAX_LIST_LIMIT`] are rejected
/// rather than silently clamped.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct ListActivitiesQuery {
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub offset: Option<u32>,
}

/// Response body for `GET /activities`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct ListActivitiesResponse {
    pub activities: Vec<Activity>,
}

/// JSON error body returned by the activity service on non-2xx responses.
///
/// Mirrors the shape used by `be-update-service` so the desktop client can
/// decode failures uniformly across HTTP services.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct ActivityErrorResponse {
    pub error: String,
    pub message: String,
    #[serde(default)]
    pub details: Option<String>,
}

/// Build a [`specta::Types`] containing every activity wire type the desktop
/// app needs. Used by the codegen binary to emit `activity.ts`.
#[cfg(feature = "specta")]
pub fn type_collection() -> specta::Types {
    specta::Types::default()
        .register::<Activity>()
        .register::<InsertActivityRequest>()
        .register::<InsertActivityResponse>()
        .register::<UpdateActivityRequest>()
        .register::<UpdateActivityResponse>()
        .register::<ListActivitiesQuery>()
        .register::<ListActivitiesResponse>()
        .register::<ActivityErrorResponse>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_request_round_trips_with_optional_fields_null() {
        let req = InsertActivityRequest {
            id: None,
            name: "name".into(),
            process_name: "proc".into(),
            window_title: "title".into(),
            icon_png_base64: None,
            started_at: Utc::now(),
            ended_at: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"id\":null"));
        assert!(json.contains("\"icon_png_base64\":null"));
        assert!(json.contains("\"ended_at\":null"));
        let back: InsertActivityRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, req.name);
        assert!(back.id.is_none());
        assert!(back.icon_png_base64.is_none());
        assert!(back.ended_at.is_none());
    }

    #[test]
    fn insert_request_decodes_with_missing_optional_fields() {
        // Forward-compat: a payload that omits optionals (older clients) still parses.
        let json = r#"{"name":"n","process_name":"p","window_title":"t","started_at":"2024-01-01T00:00:00Z"}"#;
        let back: InsertActivityRequest = serde_json::from_str(json).unwrap();
        assert!(back.id.is_none());
        assert!(back.icon_png_base64.is_none());
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
        let req = UpdateActivityRequest {
            name: None,
            window_title: None,
            ended_at: Some(Utc::now()),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"name\":null"));
        assert!(json.contains("\"window_title\":null"));
        assert!(json.contains("\"ended_at\":"));
        let back: UpdateActivityRequest = serde_json::from_str(&json).unwrap();
        assert!(back.name.is_none());
        assert!(back.window_title.is_none());
        assert!(back.ended_at.is_some());
    }

    #[test]
    fn update_request_decodes_with_only_ended_at() {
        // Heartbeat path: PATCH body contains only `ended_at`.
        let json = r#"{"ended_at":"2026-01-15T12:00:00Z"}"#;
        let back: UpdateActivityRequest = serde_json::from_str(json).unwrap();
        assert!(back.name.is_none());
        assert!(back.window_title.is_none());
        assert!(back.ended_at.is_some());
    }

    #[test]
    fn update_request_decodes_empty_body_as_all_none() {
        // The handler rejects all-None at the application layer; the
        // wire type itself must still parse so that error path is
        // reachable.
        let back: UpdateActivityRequest = serde_json::from_str("{}").unwrap();
        assert!(back.name.is_none());
        assert!(back.window_title.is_none());
        assert!(back.ended_at.is_none());
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
            "InsertActivityRequest",
            "InsertActivityResponse",
            "UpdateActivityRequest",
            "UpdateActivityResponse",
            "ListActivitiesQuery",
            "ListActivitiesResponse",
            "ActivityErrorResponse",
        ] {
            assert!(
                names.iter().any(|n| n == expected),
                "missing {expected} from collection: {names:?}"
            );
        }
    }
}
