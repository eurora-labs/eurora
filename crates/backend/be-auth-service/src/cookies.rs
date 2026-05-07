//! Cookie config + helpers for the web (cookie-mode) auth flow.
//!
//! The SPA never holds tokens in JS: the access JWT lives in
//! `eu_access` (HttpOnly), and the refresh JWT in `eu_refresh`
//! (HttpOnly, path-scoped to `/auth`). CSRF is handled by the
//! origin-allowlist middleware in `be-authz` plus `SameSite=Lax` on
//! these cookies — no separate CSRF token cookie is needed. This
//! module owns the cookie names, attributes, and `Set-Cookie` builders
//! so every handler emits a consistent shape.
//!
//! Desktop / mobile clients use the legacy bearer flow and never see
//! these cookies — see [`AuthMode`] for the dispatch.

use std::collections::HashSet;

use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use be_auth_core::{MissingWebOrigins, web_origins_from_env};
use time::Duration as TimeDuration;

pub const ACCESS_COOKIE: &str = "eu_access";
pub const REFRESH_COOKIE: &str = "eu_refresh";

/// Path attribute for the access cookie. Sent with every request to
/// the API host so any service (asset, activity, …) can authenticate.
pub const ACCESS_COOKIE_PATH: &str = "/";

/// Path attribute for the refresh cookie. Scoped to `/auth` so the
/// refresh token is only attached to refresh / logout requests, not
/// the rest of the API surface.
pub const REFRESH_COOKIE_PATH: &str = "/auth";

/// Runtime configuration for cookie attributes. Built once at
/// startup so individual handlers don't re-read env vars.
#[derive(Debug, Clone)]
pub struct CookieConfig {
    /// Optional `Domain` attribute. `None` means host-only — cookies
    /// are scoped to `api.eurora-labs.com` (or whatever host serves
    /// the API), which is the recommended posture. Set explicitly for
    /// sibling-subdomain deployments where multiple API hosts must
    /// share a session.
    pub domain: Option<String>,
    /// Whether to emit the `Secure` attribute. Production deploys
    /// must set `AUTH_COOKIE_SECURE=true` so cookies are only sent
    /// over HTTPS; local dev sets `false` because the stack runs
    /// without TLS.
    pub secure: bool,
    /// Set of browser origins (`scheme://host[:port]`) the SPA is
    /// served from. A request whose `Origin` matches one of these
    /// triggers cookie-mode auth; any other request is treated as
    /// bearer-mode.
    pub web_origins: HashSet<String>,
}

/// Failure modes when reading [`CookieConfig`] from the environment.
///
/// Each variant carries enough context (the variable name, the
/// rejected value) for the caller's startup error printer to produce
/// a remediation message without re-deriving what went wrong.
#[derive(Debug, thiserror::Error)]
pub enum CookieConfigError {
    #[error("`{name}` is unset or empty")]
    MissingEnv { name: &'static str },

    #[error("invalid `AUTH_COOKIE_SECURE` value `{value}` (expected `true` or `false`)")]
    InvalidCookieSecure { value: String },

    #[error(transparent)]
    WebOrigins(#[from] MissingWebOrigins),
}

impl CookieConfig {
    pub fn from_env() -> Result<Self, CookieConfigError> {
        let domain = std::env::var("AUTH_COOKIE_DOMAIN")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let raw_secure = std::env::var("AUTH_COOKIE_SECURE")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .ok_or(CookieConfigError::MissingEnv {
                name: "AUTH_COOKIE_SECURE",
            })?;

        let secure = if raw_secure.eq_ignore_ascii_case("true") {
            true
        } else if raw_secure.eq_ignore_ascii_case("false") {
            false
        } else {
            return Err(CookieConfigError::InvalidCookieSecure { value: raw_secure });
        };

        Ok(Self {
            domain,
            secure,
            web_origins: web_origins_from_env()?,
        })
    }

    pub fn is_web_origin(&self, origin: &str) -> bool {
        self.web_origins.contains(origin)
    }
}

/// Build the access-token cookie. `max_age_secs` should match the
/// JWT's `exp - iat`.
pub fn access_cookie(cfg: &CookieConfig, value: String, max_age_secs: i64) -> Cookie<'static> {
    base_cookie(cfg, ACCESS_COOKIE, value, ACCESS_COOKIE_PATH, max_age_secs)
}

/// Build the refresh-token cookie. Path-scoped to `/auth`.
pub fn refresh_cookie(cfg: &CookieConfig, value: String, max_age_secs: i64) -> Cookie<'static> {
    base_cookie(
        cfg,
        REFRESH_COOKIE,
        value,
        REFRESH_COOKIE_PATH,
        max_age_secs,
    )
}

/// Append `Set-Cookie` headers that delete every auth cookie we own.
pub fn clear_all(cfg: &CookieConfig, jar: CookieJar) -> CookieJar {
    jar.add(removal_cookie(cfg, ACCESS_COOKIE, ACCESS_COOKIE_PATH))
        .add(removal_cookie(cfg, REFRESH_COOKIE, REFRESH_COOKIE_PATH))
}

fn base_cookie(
    cfg: &CookieConfig,
    name: &'static str,
    value: String,
    path: &'static str,
    max_age_secs: i64,
) -> Cookie<'static> {
    let mut cookie = Cookie::build((name, value))
        .path(path)
        .secure(cfg.secure)
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(TimeDuration::seconds(max_age_secs))
        .build();
    if let Some(domain) = cfg.domain.clone() {
        cookie.set_domain(domain);
    }
    cookie
}

fn removal_cookie(cfg: &CookieConfig, name: &'static str, path: &'static str) -> Cookie<'static> {
    let mut cookie = Cookie::build((name, ""))
        .path(path)
        .secure(cfg.secure)
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(TimeDuration::ZERO)
        .build();
    if let Some(domain) = cfg.domain.clone() {
        cookie.set_domain(domain);
    }
    cookie
}

/// Whether the request comes from a configured browser SPA origin.
/// Drives [`AuthMode`] selection for the auth handlers.
pub fn is_web_request(cfg: &CookieConfig, headers: &axum::http::HeaderMap) -> bool {
    headers
        .get(axum::http::header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .map(|origin| cfg.is_web_origin(origin))
        .unwrap_or(false)
}

/// How an inbound auth request wants its session delivered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMode {
    /// Browser SPA — set HttpOnly cookies, return [`auth_core::UserResponse`].
    Cookie,
    /// Desktop / mobile — return [`auth_core::TokenResponse`] in the JSON body.
    Bearer,
}

impl AuthMode {
    pub fn from_headers(cfg: &CookieConfig, headers: &axum::http::HeaderMap) -> Self {
        if is_web_request(cfg, headers) {
            Self::Cookie
        } else {
            Self::Bearer
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, HeaderValue};

    fn config_with_origin(origin: &str) -> CookieConfig {
        let mut origins = HashSet::new();
        origins.insert(origin.to_string());
        CookieConfig {
            domain: None,
            secure: true,
            web_origins: origins,
        }
    }

    #[test]
    fn allowed_origin_is_web_request() {
        let cfg = config_with_origin("https://www.eurora-labs.com");
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::ORIGIN,
            HeaderValue::from_static("https://www.eurora-labs.com"),
        );
        assert!(is_web_request(&cfg, &headers));
        assert_eq!(AuthMode::from_headers(&cfg, &headers), AuthMode::Cookie);
    }

    #[test]
    fn unknown_origin_falls_back_to_bearer() {
        let cfg = config_with_origin("https://www.eurora-labs.com");
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::ORIGIN,
            HeaderValue::from_static("https://attacker.example"),
        );
        assert!(!is_web_request(&cfg, &headers));
        assert_eq!(AuthMode::from_headers(&cfg, &headers), AuthMode::Bearer);
    }

    #[test]
    fn missing_origin_is_bearer() {
        let cfg = config_with_origin("https://www.eurora-labs.com");
        let headers = HeaderMap::new();
        assert_eq!(AuthMode::from_headers(&cfg, &headers), AuthMode::Bearer);
    }

    #[test]
    fn access_cookie_is_http_only_and_path_root() {
        let cfg = config_with_origin("https://www.eurora-labs.com");
        let cookie = access_cookie(&cfg, "tok".into(), 3600);
        assert_eq!(cookie.name(), ACCESS_COOKIE);
        assert_eq!(cookie.path(), Some("/"));
        assert_eq!(cookie.http_only(), Some(true));
        assert_eq!(cookie.secure(), Some(true));
        assert_eq!(cookie.same_site(), Some(SameSite::Lax));
    }

    #[test]
    fn refresh_cookie_path_is_auth_scoped() {
        let cfg = config_with_origin("https://www.eurora-labs.com");
        let cookie = refresh_cookie(&cfg, "tok".into(), 7 * 24 * 3600);
        assert_eq!(cookie.path(), Some(REFRESH_COOKIE_PATH));
        assert_eq!(cookie.http_only(), Some(true));
    }
}
