//! Axum extractor for the authenticated caller.
//!
//! `be-authz::authz_middleware` validates the JWT and inserts a
//! [`Claims`](crate::Claims) value into the request extensions before any
//! handler runs. This extractor pulls those claims back out so handlers can
//! work with strongly-typed identity instead of poking at extensions.
//!
//! The [`MissingClaims`] rejection is defence-in-depth: it should only fire
//! if a route is wired into a router that does not have the authz middleware
//! in front of it. Each consuming service typically converts it into its own
//! `Unauthenticated` error variant via a `From` impl so the rendered response
//! shape matches the rest of that service's error envelope.

use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use thiserror::Error;
use uuid::Uuid;

use crate::Claims;

/// Authenticated caller, extracted from request extensions.
#[derive(Debug, Clone)]
pub struct AuthUser(pub Claims);

impl AuthUser {
    /// Parse the `sub` claim as a UUID. A malformed `sub` is a token we
    /// cannot trust, so callers usually surface this as `401 Unauthorized`.
    pub fn user_id(&self) -> Result<Uuid, InvalidUserId> {
        Uuid::parse_str(&self.0.sub).map_err(InvalidUserId)
    }

    pub fn claims(&self) -> &Claims {
        &self.0
    }
}

/// Rejection returned when the authz middleware did not run ahead of the
/// extractor.
#[derive(Debug, Error)]
#[error("missing authenticated claims")]
pub struct MissingClaims;

impl IntoResponse for MissingClaims {
    fn into_response(self) -> Response {
        tracing::warn!(
            "AuthUser extractor fired without authz middleware — route is misconfigured"
        );
        (StatusCode::UNAUTHORIZED, "Unauthenticated").into_response()
    }
}

/// Returned by [`AuthUser::user_id`] when the JWT `sub` claim is not a UUID.
#[derive(Debug, Error)]
#[error("invalid user id in claims: {0}")]
pub struct InvalidUserId(#[source] pub uuid::Error);

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = MissingClaims;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Claims>()
            .cloned()
            .map(AuthUser)
            .ok_or(MissingClaims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Role;
    use axum::http::Request;

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
    async fn extractor_returns_user_when_claims_present() {
        let claims = sample_claims("0192f8b3-3a9c-7c5d-9000-000000000001");
        let mut req = Request::new(());
        req.extensions_mut().insert(claims.clone());
        let (mut parts, _) = req.into_parts();

        let user = AuthUser::from_request_parts(&mut parts, &()).await.unwrap();
        assert_eq!(user.0.sub, claims.sub);
    }

    #[tokio::test]
    async fn extractor_rejects_when_claims_missing() {
        let req = Request::new(());
        let (mut parts, _) = req.into_parts();

        let err = AuthUser::from_request_parts(&mut parts, &())
            .await
            .unwrap_err();
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn user_id_parses_valid_uuid() {
        let claims = sample_claims("0192f8b3-3a9c-7c5d-9000-000000000001");
        let user = AuthUser(claims);
        assert!(user.user_id().is_ok());
    }

    #[test]
    fn user_id_rejects_garbage() {
        let claims = sample_claims("not-a-uuid");
        let user = AuthUser(claims);
        assert!(user.user_id().is_err());
    }
}
