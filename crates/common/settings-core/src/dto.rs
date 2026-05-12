use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// `GET /settings` — 200 body.
///
/// The server treats the settings document as an opaque JSON blob and only
/// owns the `schema_version` and `updated_at` metadata columns. Clients
/// deserialize `settings` into the [`CloudSettings`](crate::CloudSettings)
/// shape they understand; a blob written by a newer release of another
/// client round-trips verbatim through older readers.
///
/// A 404 from the server is *not* an error; it signals "no row for this
/// user yet" and triggers a first-run upload on the client. That branch
/// carries no body and is therefore not represented here.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct GetSettingsResponse {
    pub schema_version: u32,
    pub updated_at: DateTime<Utc>,
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Unknown))]
    pub settings: serde_json::Value,
}

/// `PUT /settings` — request body.
///
/// `schema_version` rides at the top level so the server can index it
/// without parsing the blob. `settings` is opaque JSON: clients are
/// responsible for calling [`CloudSettings::sanitize`](crate::CloudSettings::sanitize)
/// before serializing, since the server does not interpret the document.
///
/// `base_updated_at` is the `updated_at` the client last observed on the
/// server. `None` means "this is a first write; only succeed if no row
/// exists." The server uses this for optimistic concurrency: a mismatch
/// yields [`PutSettingsConflictResponse`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct PutSettingsRequest {
    pub schema_version: u32,
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Unknown))]
    pub settings: serde_json::Value,
    pub base_updated_at: Option<DateTime<Utc>>,
}

/// `PUT /settings` — 200 body. The client stamps these onto its local
/// cache so the next `base_updated_at` it sends matches the server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct PutSettingsAcceptedResponse {
    pub schema_version: u32,
    pub updated_at: DateTime<Utc>,
}

/// `PUT /settings` — 409 body. The server returns its current row so
/// the client can reconcile in one round trip rather than re-fetching.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct PutSettingsConflictResponse {
    pub schema_version: u32,
    pub updated_at: DateTime<Utc>,
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Unknown))]
    pub current: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn put_request_round_trip_with_base() {
        let req = PutSettingsRequest {
            schema_version: 1,
            settings: json!({"shared": {"theme": "dark"}}),
            base_updated_at: Some(DateTime::<Utc>::UNIX_EPOCH),
        };
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["schemaVersion"], 1);
        assert!(v.get("settings").is_some());
        let back: PutSettingsRequest = serde_json::from_value(v).unwrap();
        assert_eq!(back, req);
    }

    #[test]
    fn put_request_round_trip_without_base() {
        let req = PutSettingsRequest {
            schema_version: 1,
            settings: json!({}),
            base_updated_at: None,
        };
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["baseUpdatedAt"], serde_json::Value::Null);
        let back: PutSettingsRequest = serde_json::from_value(v).unwrap();
        assert_eq!(back, req);
    }

    #[test]
    fn accepted_response_uses_camel_case() {
        let r = PutSettingsAcceptedResponse {
            schema_version: 1,
            updated_at: DateTime::<Utc>::UNIX_EPOCH,
        };
        let v = serde_json::to_value(&r).unwrap();
        assert!(v.get("schemaVersion").is_some());
        assert!(v.get("updatedAt").is_some());
    }

    #[test]
    fn get_response_round_trips_opaque_settings() {
        let r = GetSettingsResponse {
            schema_version: 42,
            updated_at: DateTime::<Utc>::UNIX_EPOCH,
            settings: json!({"future_section": {"knob": 1}}),
        };
        let v = serde_json::to_value(&r).unwrap();
        let back: GetSettingsResponse = serde_json::from_value(v).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn conflict_response_round_trips() {
        let r = PutSettingsConflictResponse {
            schema_version: 1,
            updated_at: DateTime::<Utc>::UNIX_EPOCH,
            current: json!({"shared": {"theme": "light"}}),
        };
        let v = serde_json::to_value(&r).unwrap();
        let back: PutSettingsConflictResponse = serde_json::from_value(v).unwrap();
        assert_eq!(back, r);
    }
}
