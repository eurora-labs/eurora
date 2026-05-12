//! Verifies the on-the-wire shape of [`SettingsServiceError`] responses.
//!
//! These tests don't touch Postgres or Axum routing — they just decode the
//! response that [`SettingsServiceError::into_response`] produces and assert
//! the status code, `error` discriminator, and `message` field. The
//! envelope is the cross-service contract that desktop / mobile / web all
//! parse, so it has to stay stable.

use axum::body::to_bytes;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use be_remote_db::DbError;
use be_settings_service::SettingsServiceError;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct EnvelopeOnWire {
    error: String,
    message: String,
    #[serde(default)]
    #[allow(dead_code)]
    details: Option<serde_json::Value>,
}

async fn decode<T: IntoResponse>(value: T) -> (StatusCode, EnvelopeOnWire) {
    let response = value.into_response();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("body within size limit");
    let body: EnvelopeOnWire = serde_json::from_slice(&bytes).expect("envelope JSON");
    (status, body)
}

#[tokio::test]
async fn unauthenticated_envelope() {
    let (status, body) = decode(SettingsServiceError::unauthenticated("Missing claims")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body.error, "unauthenticated");
    assert!(!body.message.is_empty());
}

#[tokio::test]
async fn invalid_argument_envelope() {
    let (status, body) = decode(SettingsServiceError::invalid_argument(
        "schemaVersion out of range",
    ))
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body.error, "invalid_argument");
    assert!(body.message.contains("schemaVersion"));
}

#[tokio::test]
async fn not_found_envelope() {
    let (status, body) = decode(SettingsServiceError::NotFound).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body.error, "not_found");
}

#[tokio::test]
async fn db_not_found_maps_to_404_envelope() {
    let err: SettingsServiceError = DbError::not_found_with_id("user_settings", "abc").into();
    let (status, body) = decode(err).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body.error, "not_found");
}

#[tokio::test]
async fn db_foreign_key_violation_maps_to_400_envelope() {
    let err: SettingsServiceError = DbError::foreign_key("user").into();
    let (status, body) = decode(err).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body.error, "invalid_argument");
    assert!(body.message.contains("user"));
}

#[tokio::test]
async fn db_invalid_input_maps_to_400_envelope() {
    let err: SettingsServiceError = DbError::invalid_input("bad blob").into();
    let (status, body) = decode(err).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body.error, "invalid_argument");
}

#[tokio::test]
async fn db_pool_error_maps_to_500_envelope_with_redacted_message() {
    let err: SettingsServiceError = DbError::pool("timed out").into();
    let (status, body) = decode(err).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body.error, "database_error");
    // Internal error details must not leak to the client.
    assert!(!body.message.contains("timed out"));
    assert_eq!(body.message, "Database operation failed");
}

#[tokio::test]
async fn internal_error_redacts_detail() {
    let err = SettingsServiceError::internal("secret stack trace");
    let (status, body) = decode(err).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body.error, "internal_error");
    assert!(!body.message.contains("secret stack trace"));
}
