//! Verify that the activity-service's `From<MissingClaims>` impl produces the
//! same JSON envelope as any other client error when the authz middleware
//! has not run ahead of a route.
//!
//! The handler under test takes [`AuthUser`] and returns the user's id. We
//! exercise it through an Axum router with `oneshot` so we observe the same
//! response a real client would.

use activity_core::ActivityErrorResponse;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use axum::response::IntoResponse;
use axum::{Router, routing::get};
use be_activity_service::{ActivityResult, ActivityServiceError};
use be_auth_core::{AuthUser, Claims, Role};
use tower::util::ServiceExt;

async fn echo_user(user: AuthUser) -> ActivityResult<String> {
    Ok(user.user_id()?.to_string())
}

fn router() -> Router {
    // Compose our error type with the shared extractor: the `?` in
    // `echo_user` relies on `From<MissingClaims>` and `From<InvalidUserId>`
    // for `ActivityServiceError`, so any failure renders through the same
    // envelope as the real handlers.
    async fn handler(
        user: Result<AuthUser, be_auth_core::MissingClaims>,
    ) -> axum::response::Response {
        match user {
            Ok(user) => match echo_user(user).await {
                Ok(s) => s.into_response(),
                Err(e) => e.into_response(),
            },
            Err(_) => ActivityServiceError::from(be_auth_core::MissingClaims).into_response(),
        }
    }
    Router::new().route("/echo", get(handler))
}

fn sample_claims(sub: &str) -> Claims {
    Claims {
        sub: sub.to_string(),
        email: "test@example.com".to_string(),
        display_name: None,
        exp: 0,
        iat: 0,
        token_type: "access".to_string(),
        role: Role::Free,
        aud: "eurora".to_string(),
        email_verified: true,
        jti: "jti".to_string(),
    }
}

#[tokio::test]
async fn missing_claims_returns_unauthenticated_envelope() {
    let response = router()
        .oneshot(Request::builder().uri("/echo").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let bytes = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
    let body: ActivityErrorResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body.error, "unauthenticated");
}

#[tokio::test]
async fn invalid_user_id_returns_unauthenticated_envelope() {
    let mut req = Request::builder().uri("/echo").body(Body::empty()).unwrap();
    req.extensions_mut().insert(sample_claims("not-a-uuid"));

    let response = router().oneshot(req).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let bytes = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
    let body: ActivityErrorResponse = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body.error, "unauthenticated");
}

#[tokio::test]
async fn valid_claims_pass_through() {
    let sub = "0192f8b3-3a9c-7c5d-9000-000000000001";
    let mut req = Request::builder().uri("/echo").body(Body::empty()).unwrap();
    req.extensions_mut().insert(sample_claims(sub));

    let response = router().oneshot(req).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
    assert_eq!(std::str::from_utf8(&bytes).unwrap(), sub);
}
