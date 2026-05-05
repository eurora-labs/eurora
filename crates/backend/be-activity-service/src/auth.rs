use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use uuid::Uuid;

use crate::error::ActivityServiceError;

/// Authenticated caller, extracted from request extensions.
///
/// `be-authz::authz_middleware` validates the JWT and inserts
/// [`be_auth_core::Claims`] into the request before any handler runs, so by
/// the time this extractor fires the claims are already trusted. The
/// `Unauthenticated` rejection here is defence in depth — it should only
/// fire if a route is wired without the authz middleware.
pub struct AuthUser(pub be_auth_core::Claims);

impl AuthUser {
    /// Parse the `sub` field as a UUID. Errors are mapped to a 401 because
    /// a malformed `sub` indicates a token we cannot trust.
    pub fn user_id(&self) -> Result<Uuid, ActivityServiceError> {
        Uuid::parse_str(&self.0.sub)
            .map_err(|_| ActivityServiceError::unauthenticated("Invalid user id in claims"))
    }
}

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = ActivityServiceError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let claims = parts
            .extensions
            .get::<be_auth_core::Claims>()
            .cloned()
            .ok_or_else(|| ActivityServiceError::unauthenticated("Missing authenticated claims"))?;
        Ok(AuthUser(claims))
    }
}
