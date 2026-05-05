//! Bearer-token extractors for the auth-service handlers.
//!
//! The `/auth/*` prefix is bypassed by the global `authz_middleware`
//! (see `be-authz`), so handlers that require a valid token must
//! validate it themselves. Two extractors cover the cases:
//!
//! - [`AccessClaims`]: routes that need a valid **access** token in the
//!   `Authorization: Bearer <token>` header (e.g.
//!   `/auth/login-token/associate`, `/auth/email/resend-verification`).
//! - [`RefreshClaims`]: routes that need a valid **refresh** token in
//!   the same header (`/auth/refresh`, `/auth/logout`). Carries both the
//!   parsed claims and the raw token so the handler can hash it for DB
//!   lookups.
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
            .jwt_config()
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
            .jwt_config()
            .validate_refresh_token(token)
            .map_err(|_| AuthError::InvalidToken)?;
        Ok(RefreshClaims {
            claims,
            raw_token: token.to_string(),
        })
    }
}

/// Extract the bearer token from the `Authorization` header.
///
/// Tolerates RFC 7235 case variations on the scheme (`Bearer` /
/// `bearer` / `BEARER`) and any leading whitespace before the scheme.
/// The token itself must follow exactly one space.
fn extract_bearer(parts: &Parts) -> Result<&str, AuthError> {
    let header = parts
        .headers
        .get(axum::http::header::AUTHORIZATION)
        .ok_or(AuthError::MissingAuthHeader)?;

    let header_str = header.to_str().map_err(|_| AuthError::InvalidAuthHeader)?;
    let trimmed = header_str.trim_start();

    let (scheme, rest) = trimmed
        .split_once(' ')
        .ok_or(AuthError::InvalidAuthHeader)?;

    if !scheme.eq_ignore_ascii_case("Bearer") {
        return Err(AuthError::InvalidAuthHeader);
    }

    if rest.is_empty() {
        return Err(AuthError::InvalidAuthHeader);
    }

    Ok(rest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, HeaderValue, Request};

    fn parts_with_header(value: &str) -> Parts {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_str(value).unwrap(),
        );
        let mut req = Request::new(());
        *req.headers_mut() = headers;
        req.into_parts().0
    }

    #[test]
    fn extracts_bearer_token() {
        let parts = parts_with_header("Bearer abc.def.ghi");
        assert_eq!(extract_bearer(&parts).unwrap(), "abc.def.ghi");
    }

    #[test]
    fn extracts_bearer_lowercase() {
        let parts = parts_with_header("bearer abc.def.ghi");
        assert_eq!(extract_bearer(&parts).unwrap(), "abc.def.ghi");
    }

    #[test]
    fn extracts_bearer_uppercase() {
        let parts = parts_with_header("BEARER abc.def.ghi");
        assert_eq!(extract_bearer(&parts).unwrap(), "abc.def.ghi");
    }

    #[test]
    fn tolerates_leading_whitespace() {
        let parts = parts_with_header("  Bearer abc");
        assert_eq!(extract_bearer(&parts).unwrap(), "abc");
    }

    #[test]
    fn rejects_other_scheme() {
        let parts = parts_with_header("Basic abc");
        assert!(matches!(
            extract_bearer(&parts),
            Err(AuthError::InvalidAuthHeader)
        ));
    }

    #[test]
    fn rejects_empty_token() {
        let parts = parts_with_header("Bearer ");
        assert!(matches!(
            extract_bearer(&parts),
            Err(AuthError::InvalidAuthHeader)
        ));
    }

    #[test]
    fn rejects_missing_header() {
        let req = Request::new(());
        let (parts, _) = req.into_parts();
        assert!(matches!(
            extract_bearer(&parts),
            Err(AuthError::MissingAuthHeader)
        ));
    }
}
