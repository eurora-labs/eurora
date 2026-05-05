//! HTTP-level contract tests for the auth-service error envelope.
//!
//! Pin the wire shape that clients (the desktop app, the web app) dispatch
//! on. The `error` field is the stable kind discriminator and must match
//! values exposed in `auth_core::error_kinds`; the status-code mapping
//! must stay consistent across releases. These tests replace the gRPC
//! `oauth_email_conflict` round-trip test that lived in `error.rs`.

use auth_core::{AuthErrorResponse, error_kinds};
use axum::body::to_bytes;
use axum::http::StatusCode;
use be_auth_service::AuthError;

async fn decode<T: axum::response::IntoResponse>(value: T) -> (StatusCode, AuthErrorResponse) {
    let response = value.into_response();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
    let body: AuthErrorResponse = serde_json::from_slice(&bytes).unwrap();
    (status, body)
}

#[tokio::test]
async fn oauth_email_conflict_envelope() {
    let (status, body) = decode(AuthError::OAuthEmailConflict).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body.error, error_kinds::OAUTH_EMAIL_CONFLICT);
    assert!(
        !body.message.is_empty(),
        "client message must explain the failure"
    );
}

#[tokio::test]
async fn invalid_token_envelope() {
    let (status, body) = decode(AuthError::InvalidToken).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body.error, error_kinds::UNAUTHENTICATED);
}

#[tokio::test]
async fn email_not_verified_envelope() {
    let (status, body) = decode(AuthError::EmailNotVerified).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body.error, error_kinds::EMAIL_NOT_VERIFIED);
}

#[tokio::test]
async fn invalid_input_passes_message_through() {
    let err = AuthError::InvalidInput("Email already taken".to_string());
    let (status, body) = decode(err).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body.error, error_kinds::INVALID_ARGUMENT);
    assert_eq!(body.message, "Email already taken");
}

#[tokio::test]
async fn internal_errors_redact_detail() {
    let err = AuthError::Internal("redis exploded with token=secret123".to_string());
    let (status, body) = decode(err).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body.error, error_kinds::INTERNAL_ERROR);
    assert_eq!(
        body.message, "Internal error",
        "internal error detail must never be echoed back to the client"
    );
}

#[tokio::test]
async fn verification_resend_cooldown_envelope() {
    let (status, body) = decode(AuthError::VerificationResendCooldown).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(body.error, error_kinds::RATE_LIMITED);
}
