//! Bearer-token extractors for the auth-service handlers.
//!
//! The `/auth/*` prefix is bypassed by the global `authz_middleware`
//! (see `be-authz`), so handlers that require a valid token must validate
//! it themselves. Two extractors cover the cases:
//!
//! - [`AccessClaims`]: routes that need a valid **access** token in the
//!   `Authorization: Bearer <token>` header (e.g. associate-login-token,
//!   resend-verification-email).
//! - [`RefreshClaims`]: routes that need a valid **refresh** token in the
//!   same header (refresh, logout). Carries both the parsed claims and the
//!   raw token so the handler can hash it for DB lookups.
//!
//! Both extractors return [`AuthError`] directly so failures propagate
//! through the same JSON envelope as any other auth-service error.

use std::sync::Arc;

use auth_core::Claims;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use crate::error::AuthError;
use crate::service::AppState;

pub struct AccessClaims(pub Claims);

pub struct RefreshClaims {
    pub claims: Claims,
    /// The raw token as presented by the client. Handlers hash this
    /// value to look up / revoke the matching DB row.
    pub raw_token: String,
}

impl FromRequestParts<Arc<AppState>> for AccessClaims {
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let token = extract_bearer(parts)?;
        let claims = state
            .jwt_config
            .validate_access_token(token)
            .map_err(|_| AuthError::InvalidToken)?;
        Ok(AccessClaims(claims))
    }
}

impl FromRequestParts<Arc<AppState>> for RefreshClaims {
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let token = extract_bearer(parts)?;
        let claims = state
            .jwt_config
            .validate_refresh_token(token)
            .map_err(|_| AuthError::InvalidToken)?;
        Ok(RefreshClaims {
            claims,
            raw_token: token.to_string(),
        })
    }
}

fn extract_bearer(parts: &Parts) -> Result<&str, AuthError> {
    let header = parts
        .headers
        .get(axum::http::header::AUTHORIZATION)
        .ok_or(AuthError::MissingAuthHeader)?;

    let header_str = header.to_str().map_err(|_| AuthError::InvalidAuthHeader)?;

    header_str
        .strip_prefix("Bearer ")
        .ok_or(AuthError::InvalidAuthHeader)
}
