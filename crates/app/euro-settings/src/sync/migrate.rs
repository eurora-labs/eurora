//! First-run upload of the local cache to the cloud.
//!
//! Triggered when `GET /settings` returns 404 — there is no row for
//! this user yet, so the engine seeds the server with whatever the
//! local cache currently holds. The migration is a single `PUT` with
//! `baseUpdatedAt: null`, which the server's optimistic-concurrency
//! ladder treats as "insert if nothing exists, otherwise conflict."
//!
//! Two terminal states:
//!
//! - 200 → the row was inserted; the engine stamps the response
//!   metadata onto the local cache and the user is "synced."
//! - 409 → another client raced us and now a row exists. The engine
//!   discards the local cache and adopts the server's row, since we
//!   have no basis to claim the local edits are newer (a 404 followed
//!   by a 409 means the row was created mid-request, not that ours
//!   collided with a known-earlier base).
//!
//! Module is intentionally small: the engine owns the cache-writeback
//! plumbing, so this file holds the request shape only.

use settings_core::{CURRENT_SCHEMA_VERSION, CloudSettings, PutSettingsRequest};

/// Build the `PUT /settings` body that uploads the local cache as a
/// brand-new server row. Centralised here so the "no `baseUpdatedAt`
/// means first-run insert" invariant is encoded in one place.
#[must_use]
pub(super) fn first_run_request(settings: &CloudSettings) -> PutSettingsRequest {
    PutSettingsRequest {
        schema_version: CURRENT_SCHEMA_VERSION,
        settings: serde_json::to_value(settings).expect(
            "CloudSettings is serialisable into serde_json::Value; see settings_core::cloud",
        ),
        // `None` is the load-bearing piece: the server interprets it as
        // "insert if the row doesn't exist, conflict if it does."
        base_updated_at: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_run_request_carries_null_base() {
        let body = first_run_request(&CloudSettings::default());
        assert_eq!(body.schema_version, CURRENT_SCHEMA_VERSION);
        assert!(body.base_updated_at.is_none());
        let wire = serde_json::to_value(&body).unwrap();
        assert_eq!(wire["baseUpdatedAt"], serde_json::Value::Null);
    }

    #[test]
    fn first_run_request_serialises_full_settings_blob() {
        let body = first_run_request(&CloudSettings::default());
        let settings_obj = body.settings.as_object().expect("settings is an object");
        // Pin the top-level keys so an inadvertent breaking refactor of
        // CloudSettings is caught here as well as in settings-core.
        for key in ["shared", "desktop", "mobile", "web"] {
            assert!(
                settings_obj.contains_key(key),
                "expected {key} in first-run payload, got {settings_obj:?}"
            );
        }
        // Schema version lives on the envelope, never inside the blob.
        assert!(
            !settings_obj.contains_key("schemaVersion"),
            "schemaVersion must not appear inside the settings blob, got {settings_obj:?}"
        );
    }
}
