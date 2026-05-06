//! HTTP-level contract tests for the activity-service error envelope.
//!
//! Pin the wire shape that the desktop app dispatches on. The `error` field
//! is the stable kind discriminator used for client-side error handling and
//! analytics; the status-code mapping must stay consistent across releases.

use activity_core::ActivityErrorResponse;
use axum::body::to_bytes;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use be_activity_service::ActivityServiceError;
use be_remote_db::DbError;

async fn decode<T: IntoResponse>(value: T) -> (StatusCode, ActivityErrorResponse) {
    let response = value.into_response();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
    let body: ActivityErrorResponse = serde_json::from_slice(&bytes).unwrap();
    (status, body)
}

#[tokio::test]
async fn unauthenticated_envelope() {
    let (status, body) = decode(ActivityServiceError::unauthenticated("Missing claims")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body.error, "unauthenticated");
    assert!(!body.message.is_empty());
}

#[tokio::test]
async fn invalid_argument_passes_message_through() {
    let (status, body) = decode(ActivityServiceError::invalid_argument(
        "limit must be <= 100",
    ))
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body.error, "invalid_argument");
    assert_eq!(body.message, "Invalid argument: limit must be <= 100");
}

#[tokio::test]
async fn invalid_base64_envelope() {
    use base64::Engine;
    let decode_err = base64::engine::general_purpose::STANDARD
        .decode("not-base64!!!")
        .unwrap_err();
    let err = ActivityServiceError::invalid_base64("icon_png_base64", decode_err);
    let (status, body) = decode(err).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body.error, "invalid_base64");
    assert!(body.message.contains("icon_png_base64"));
}

#[tokio::test]
async fn db_not_found_maps_to_404_envelope() {
    let err: ActivityServiceError = DbError::not_found_with_id("activity", "abc").into();
    let (status, body) = decode(err).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body.error, "not_found");
}

#[tokio::test]
async fn db_unique_violation_maps_to_409_envelope() {
    let err: ActivityServiceError = DbError::unique_violation("activities_pkey").into();
    let (status, body) = decode(err).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body.error, "conflict");
    assert!(body.message.contains("activities_pkey"));
}

#[tokio::test]
async fn db_foreign_key_violation_maps_to_400_envelope() {
    let err: ActivityServiceError = DbError::foreign_key("asset").into();
    let (status, body) = decode(err).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body.error, "invalid_argument");
}

#[tokio::test]
async fn db_pool_error_redacts_detail() {
    let err: ActivityServiceError = DbError::pool("connection string with secrets").into();
    let (status, body) = decode(err).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body.error, "database_error");
    assert_eq!(
        body.message, "Database operation failed",
        "internal database errors must never echo their detail to the client"
    );
}

#[tokio::test]
async fn asset_validation_error_maps_to_400_envelope() {
    let err: ActivityServiceError = be_asset::AssetError::EmptyContent.into();
    let (status, body) = decode(err).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body.error, "invalid_argument");
}

#[tokio::test]
async fn asset_internal_error_redacts_detail() {
    let err: ActivityServiceError = be_asset::AssetError::DatabaseCreate(
        be_remote_db::DbError::Internal("postgres exploded".to_string()),
    )
    .into();
    let (status, body) = decode(err).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body.error, "asset_error");
    assert_eq!(
        body.message, "Asset operation failed",
        "internal asset errors must never echo their detail to the client"
    );
}

#[tokio::test]
async fn missing_claims_maps_to_401() {
    use be_auth_core::MissingClaims;
    let err: ActivityServiceError = MissingClaims.into();
    let (status, body) = decode(err).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body.error, "unauthenticated");
}
