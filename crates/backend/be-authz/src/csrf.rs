//! CSRF protection for the cookie-mode (browser) auth flow.
//!
//! The SPA holds its session in `HttpOnly` cookies the browser attaches
//! automatically — which means a request from a malicious origin would
//! also carry the cookie if it reached the wire. We block that two ways:
//!
//! 1. **Origin allowlist.** Mutating requests from a browser must come
//!    from a configured SPA origin. Cross-site forgery shows up as a
//!    `Origin` header pointing somewhere we don't recognise.
//! 2. **Double-submit token.** Cookie-mode requests must echo the
//!    `eu_csrf` cookie value back as `X-CSRF-Token`. JS on a
//!    third-party origin can't read the cookie (Same-Origin Policy
//!    blocks `document.cookie` cross-site), so it can't synthesise a
//!    matching header.
//!
//! Bearer-mode (desktop / mobile) traffic — recognised by the
//! `Authorization` header — is left alone; CSRF is a browser concern.
//! Same-origin requests with no `Origin` header (curl, server-to-server)
//! and the configured public bypass paths also pass through untouched.

use std::collections::HashSet;
use std::sync::Arc;

use axum::extract::Request;
use axum::http::{Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum_extra::extract::cookie::CookieJar;

/// Cookie + header pair the SPA echoes for the double-submit check.
const CSRF_COOKIE: &str = "eu_csrf";
const CSRF_HEADER: &str = "x-csrf-token";

#[derive(Debug, Clone)]
pub struct CsrfConfig {
    /// Browser origins (`scheme://host[:port]`) the SPA is served from.
    /// A cookie-mode mutating request must match one of these.
    pub web_origins: HashSet<String>,
}

impl CsrfConfig {
    pub fn from_env() -> Self {
        let origins: HashSet<String> = std::env::var("WEB_ALLOWED_ORIGINS")
            .or_else(|_| std::env::var("CORS_ALLOWED_ORIGINS"))
            .unwrap_or_else(|_| "https://www.eurora-labs.com".into())
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Self {
            web_origins: origins,
        }
    }

    pub fn new(web_origins: HashSet<String>) -> Self {
        Self { web_origins }
    }

    fn is_allowed_origin(&self, origin: &str) -> bool {
        self.web_origins.contains(origin)
    }
}

pub async fn csrf_middleware(
    axum::extract::State(state): axum::extract::State<Arc<CsrfConfig>>,
    req: Request,
    next: Next,
) -> Response {
    if !is_mutating(req.method()) {
        return next.run(req).await;
    }

    if has_bearer_auth(&req) {
        return next.run(req).await;
    }

    let jar = CookieJar::from_headers(req.headers());
    let csrf_cookie = jar.get(CSRF_COOKIE).map(|c| c.value().to_owned());

    let origin = req
        .headers()
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);

    // No CSRF cookie + no Origin header → most likely a same-origin
    // server-to-server caller (no browser, no session). Let it through;
    // it has no cookie to be tricked into sending anyway.
    if csrf_cookie.is_none() && origin.is_none() {
        return next.run(req).await;
    }

    if let Some(origin) = origin.as_deref()
        && !state.is_allowed_origin(origin)
    {
        tracing::warn!(%origin, "CSRF: rejecting request from non-allowlisted origin");
        return forbidden("Origin not allowed");
    }

    let Some(cookie_value) = csrf_cookie else {
        tracing::warn!("CSRF: missing eu_csrf cookie on browser mutating request");
        return forbidden("Missing CSRF cookie");
    };

    let header_value = req
        .headers()
        .get(CSRF_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);

    let Some(header_value) = header_value else {
        tracing::warn!("CSRF: missing X-CSRF-Token header");
        return forbidden("Missing CSRF token");
    };

    if !constant_time_eq(cookie_value.as_bytes(), header_value.as_bytes()) {
        tracing::warn!("CSRF: header / cookie mismatch");
        return forbidden("CSRF token mismatch");
    }

    next.run(req).await
}

fn is_mutating(method: &Method) -> bool {
    !matches!(method, &Method::GET | &Method::HEAD | &Method::OPTIONS)
}

fn has_bearer_auth(req: &Request) -> bool {
    req.headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|s| {
            s.trim_start()
                .split_once(' ')
                .is_some_and(|(scheme, _)| scheme.eq_ignore_ascii_case("Bearer"))
        })
        .unwrap_or(false)
}

fn forbidden(message: &'static str) -> Response {
    (
        StatusCode::FORBIDDEN,
        axum::Json(serde_json::json!({ "error": message })),
    )
        .into_response()
}

/// Constant-time byte comparison so we don't leak the CSRF token byte
/// by byte through timing.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{HeaderValue, Request};
    use axum::routing::post;
    use tower::ServiceExt;

    fn build_router() -> Router {
        let mut origins = HashSet::new();
        origins.insert("https://www.eurora-labs.com".to_string());
        let cfg = Arc::new(CsrfConfig::new(origins));
        Router::new()
            .route("/mutate", post(|| async { StatusCode::OK }))
            .layer(axum::middleware::from_fn_with_state(cfg, csrf_middleware))
    }

    fn cookie_mode_request(
        origin: Option<&str>,
        cookie: Option<&str>,
        header: Option<&str>,
    ) -> Request<Body> {
        let mut builder = Request::builder().method(Method::POST).uri("/mutate");
        if let Some(o) = origin {
            builder = builder.header(header::ORIGIN, o);
        }
        if let Some(c) = cookie {
            builder = builder.header(header::COOKIE, format!("eu_csrf={c}"));
        }
        if let Some(h) = header {
            builder = builder.header(CSRF_HEADER, HeaderValue::from_str(h).unwrap());
        }
        builder.body(Body::empty()).unwrap()
    }

    #[tokio::test]
    async fn matching_cookie_and_header_passes() {
        let router = build_router();
        let resp = router
            .oneshot(cookie_mode_request(
                Some("https://www.eurora-labs.com"),
                Some("token-abc"),
                Some("token-abc"),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn missing_header_is_rejected() {
        let router = build_router();
        let resp = router
            .oneshot(cookie_mode_request(
                Some("https://www.eurora-labs.com"),
                Some("token-abc"),
                None,
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn token_mismatch_is_rejected() {
        let router = build_router();
        let resp = router
            .oneshot(cookie_mode_request(
                Some("https://www.eurora-labs.com"),
                Some("cookie-token"),
                Some("header-token"),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn disallowed_origin_is_rejected() {
        let router = build_router();
        let resp = router
            .oneshot(cookie_mode_request(
                Some("https://attacker.example"),
                Some("token-abc"),
                Some("token-abc"),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn bearer_auth_bypasses_csrf() {
        let router = build_router();
        let req = Request::builder()
            .method(Method::POST)
            .uri("/mutate")
            .header(header::AUTHORIZATION, "Bearer some.jwt.token")
            .body(Body::empty())
            .unwrap();
        let resp = router.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn server_to_server_no_origin_no_cookie_passes() {
        let router = build_router();
        let resp = router
            .oneshot(cookie_mode_request(None, None, None))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn get_requests_are_not_checked() {
        let router = build_router();
        let req = Request::builder()
            .method(Method::GET)
            .uri("/mutate")
            .header(header::ORIGIN, "https://attacker.example")
            .body(Body::empty())
            .unwrap();
        // Route doesn't have a GET handler, but the middleware should
        // not 403 — it should let the request through to a 405.
        let resp = router.oneshot(req).await.unwrap();
        assert_ne!(resp.status(), StatusCode::FORBIDDEN);
    }
}
