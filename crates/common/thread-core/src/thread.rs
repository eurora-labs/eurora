//! Thread CRUD + search wire types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "specta")]
use specta::Type;

/// A persisted thread row as returned to the client.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct Thread {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub active_leaf_id: Option<Uuid>,
}

/// Request body for `POST /threads`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct CreateThreadRequest {
    #[serde(default)]
    pub title: Option<String>,
}

/// Response body for `POST /threads`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct CreateThreadResponse {
    pub thread: Thread,
}

/// Query parameters for `GET /threads`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct ListThreadsQuery {
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub offset: Option<u32>,
}

/// Response body for `GET /threads`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct ListThreadsResponse {
    pub threads: Vec<Thread>,
}

/// Response body for `GET /threads/{thread_id}`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct GetThreadResponse {
    pub thread: Thread,
}

/// Response body for `DELETE /threads/{thread_id}`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct DeleteThreadResponse {}

/// Request body for `POST /threads/{thread_id}/title`.
///
/// The endpoint reads recent thread history server-side, so no payload is
/// needed. An empty body type keeps the request well-typed for code-gen and
/// leaves room to add fields later.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct GenerateThreadTitleRequest {}

/// Response body for `POST /threads/{thread_id}/title`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct GenerateThreadTitleResponse {
    pub thread: Thread,
}

/// Query parameters for `GET /threads/search`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct SearchThreadsQuery {
    pub q: String,
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub offset: Option<u32>,
}

/// Response body for `GET /threads/search`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct SearchThreadsResponse {
    pub results: Vec<SearchThreadResult>,
}

/// One thread hit returned by full-text search.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct SearchThreadResult {
    pub id: Uuid,
    pub title: String,
    pub rank: f32,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_thread_request_serializes_optional_title_as_null() {
        let req = CreateThreadRequest::default();
        let s = serde_json::to_string(&req).unwrap();
        assert_eq!(s, r#"{"title":null}"#);
        let back: CreateThreadRequest = serde_json::from_str(&s).unwrap();
        assert!(back.title.is_none());
    }

    #[test]
    fn create_thread_request_decodes_with_missing_title() {
        // Forward-compat: older clients that omit the field still parse.
        let back: CreateThreadRequest = serde_json::from_str("{}").unwrap();
        assert!(back.title.is_none());
    }

    #[test]
    fn list_threads_query_round_trips() {
        let q = ListThreadsQuery {
            limit: Some(10),
            offset: Some(5),
        };
        let s = serde_json::to_string(&q).unwrap();
        let back: ListThreadsQuery = serde_json::from_str(&s).unwrap();
        assert_eq!(q, back);
    }
}
