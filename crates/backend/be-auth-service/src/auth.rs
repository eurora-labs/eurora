//! Token extractors for the auth-service handlers.
//!
//! The `/auth/*` prefix is bypassed by the global `authz_middleware`
//! (see `be-authz`), so handlers that require a valid token must
//! validate it themselves. Two extractors cover the cases:
//!
//! - [`AccessClaims`]: routes that need a valid **access** token. Looks
//!   for `Authorization: Bearer …` first (desktop / mobile) and falls
//!   back to the `eu_access` cookie (browser SPA).
//! - [`RefreshClaims`]: routes that need a valid **refresh** token,
//!   either as `Authorization: Bearer …` or as the `eu_refresh`
//!   cookie. Carries the raw token alongside the parsed claims so the
//!   handler can hash it for DB lookups.
//!
//! Both extractors return [`AuthError`] directly so failures propagate
//! through the same JSON envelope as any other auth-service error.

use std::sync::Arc;

use auth_core::Claims;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum_extra::extract::cookie::CookieJar;

use crate::cookies::{ACCESS_COOKIE, REFRESH_COOKIE};
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
        let path = parts.uri.path().to_owned();
        let token = match extract_token(parts, ACCESS_COOKIE) {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(%path, error = ?e, "AccessClaims: token extraction failed");
                return Err(e);
            }
        };
        let claims = state
            .jwt_config()
            .validate_access_token(&token)
            .map_err(|e| {
                tracing::warn!(
                    %path,
                    token_len = token.len(),
                    error = %e,
                    "AccessClaims: JWT validation failed"
                );
                AuthError::InvalidToken
            })?;
        tracing::debug!(%path, sub = %claims.sub, "AccessClaims: validated");
        Ok(AccessClaims(claims))
    }
}

impl FromRequestParts<Arc<AppState>> for RefreshClaims {
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let token = extract_token(parts, REFRESH_COOKIE)?;
        let claims = state
            .jwt_config()
            .validate_refresh_token(&token)
            .map_err(|_| AuthError::InvalidToken)?;
        Ok(RefreshClaims {
            claims,
            raw_token: token,
        })
    }
}

/// Pull a token from either the `Authorization: Bearer …` header
/// (desktop / mobile flow) or the named cookie (browser flow). The
/// header wins when both are present so a desktop client that happens
/// to receive a stale cookie doesn't authenticate twice.
fn extract_token(parts: &Parts, cookie_name: &str) -> Result<String, AuthError> {
    if let Some(header) = parts.headers.get(axum::http::header::AUTHORIZATION) {
        return extract_bearer_from_header(header).map(str::to_owned);
    }

    let jar = CookieJar::from_headers(&parts.headers);
    if let Some(cookie) = jar.get(cookie_name) {
        let value = cookie.value();
        if value.is_empty() {
            return Err(AuthError::InvalidAuthHeader);
        }
        return Ok(value.to_owned());
    }

    Err(AuthError::MissingAuthHeader)
}

fn extract_bearer_from_header(header: &axum::http::HeaderValue) -> Result<&str, AuthError> {
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

    fn parts_with_header(name: axum::http::HeaderName, value: &str) -> Parts {
        let mut headers = HeaderMap::new();
        headers.insert(name, HeaderValue::from_str(value).unwrap());
        let mut req = Request::new(());
        *req.headers_mut() = headers;
        req.into_parts().0
    }

    fn parts_with_authorization(value: &str) -> Parts {
        parts_with_header(axum::http::header::AUTHORIZATION, value)
    }

    #[test]
    fn extracts_bearer_token() {
        let parts = parts_with_authorization("Bearer abc.def.ghi");
        assert_eq!(extract_token(&parts, ACCESS_COOKIE).unwrap(), "abc.def.ghi");
    }

    #[test]
    fn extracts_bearer_lowercase() {
        let parts = parts_with_authorization("bearer abc.def.ghi");
        assert_eq!(extract_token(&parts, ACCESS_COOKIE).unwrap(), "abc.def.ghi");
    }

    #[test]
    fn extracts_bearer_uppercase() {
        let parts = parts_with_authorization("BEARER abc.def.ghi");
        assert_eq!(extract_token(&parts, ACCESS_COOKIE).unwrap(), "abc.def.ghi");
    }

    #[test]
    fn tolerates_leading_whitespace() {
        let parts = parts_with_authorization("  Bearer abc");
        assert_eq!(extract_token(&parts, ACCESS_COOKIE).unwrap(), "abc");
    }

    #[test]
    fn rejects_other_scheme() {
        let parts = parts_with_authorization("Basic abc");
        assert!(matches!(
            extract_token(&parts, ACCESS_COOKIE),
            Err(AuthError::InvalidAuthHeader)
        ));
    }

    #[test]
    fn rejects_empty_bearer_token() {
        let parts = parts_with_authorization("Bearer ");
        assert!(matches!(
            extract_token(&parts, ACCESS_COOKIE),
            Err(AuthError::InvalidAuthHeader)
        ));
    }

    #[test]
    fn rejects_missing_header_and_cookie() {
        let req = Request::new(());
        let (parts, _) = req.into_parts();
        assert!(matches!(
            extract_token(&parts, ACCESS_COOKIE),
            Err(AuthError::MissingAuthHeader)
        ));
    }

    #[test]
    fn falls_back_to_cookie_when_header_missing() {
        let parts = parts_with_header(axum::http::header::COOKIE, "eu_access=abc.def.ghi");
        assert_eq!(extract_token(&parts, ACCESS_COOKIE).unwrap(), "abc.def.ghi");
    }

    #[test]
    fn cookie_for_different_name_does_not_match() {
        let parts = parts_with_header(axum::http::header::COOKIE, "other=abc.def.ghi");
        assert!(matches!(
            extract_token(&parts, ACCESS_COOKIE),
            Err(AuthError::MissingAuthHeader)
        ));
    }

    #[test]
    fn header_wins_over_cookie() {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_static("Bearer header-token"),
        );
        headers.insert(
            axum::http::header::COOKIE,
            HeaderValue::from_static("eu_access=cookie-token"),
        );
        let mut req = Request::new(());
        *req.headers_mut() = headers;
        let parts = req.into_parts().0;
        assert_eq!(
            extract_token(&parts, ACCESS_COOKIE).unwrap(),
            "header-token"
        );
    }
}
