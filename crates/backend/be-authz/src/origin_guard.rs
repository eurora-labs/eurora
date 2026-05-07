//! Origin allowlist for the cookie-mode (browser) auth flow.
//!
//! The SPA holds its session in `HttpOnly` cookies the browser attaches
//! automatically — which means a request from a malicious origin would
//! also carry the cookie if it reached the wire. Two layers stop that
//! before this middleware runs:
//!
//! 1. `SameSite=Lax` on the session cookies — modern browsers refuse to
//!    attach them to cross-site requests in the first place.
//! 2. The `Origin` request header is browser-set and unforgeable from
//!    JavaScript (it's a forbidden header per the Fetch spec), so any
//!    cross-site mutating request a browser does send carries an
//!    attacker-controlled origin we can recognise.
//!
//! This middleware enforces (2): on a mutating request from a browser,
//! the `Origin` header must match a configured SPA origin. Bearer-mode
//! traffic (Authorization header) and same-origin server-to-server
//! callers (no Origin header) are left alone — they have no ambient
//! cookie to be tricked into sending.

use std::collections::HashSet;
use std::sync::Arc;

use axum::extract::Request;
use axum::http::{Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use be_auth_core::{MissingWebOrigins, web_origins_from_env};

#[derive(Debug, Clone)]
pub struct OriginGuardConfig {
    /// Browser origins (`scheme://host[:port]`) the SPA is served from.
    /// A mutating request carrying an `Origin` header must match one of
    /// these.
    pub web_origins: HashSet<String>,
}

impl OriginGuardConfig {
    pub fn from_env() -> Result<Self, MissingWebOrigins> {
        Ok(Self {
            web_origins: web_origins_from_env()?,
        })
    }

    pub fn new(web_origins: HashSet<String>) -> Self {
        Self { web_origins }
    }

    fn is_allowed_origin(&self, origin: &str) -> bool {
        self.web_origins.contains(origin)
    }
}

pub async fn origin_guard_middleware(
    axum::extract::State(state): axum::extract::State<Arc<OriginGuardConfig>>,
    req: Request,
    next: Next,
) -> Response {
    if !is_mutating(req.method()) {
        return next.run(req).await;
    }

    if has_bearer_auth(&req) {
        return next.run(req).await;
    }

    let origin = req
        .headers()
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok());

    // No Origin header → not a browser fetch (curl, server-to-server,
    // same-origin form post on legacy browsers). Nothing to allowlist
    // against; let it through.
    let Some(origin) = origin else {
        return next.run(req).await;
    };

    if !state.is_allowed_origin(origin) {
        tracing::warn!(%origin, "Origin guard: rejecting request from non-allowlisted origin");
        return forbidden("Origin not allowed");
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::Body;
    use axum::http::Request;
    use axum::routing::post;
    use tower::ServiceExt;

    fn build_router() -> Router {
        let mut origins = HashSet::new();
        origins.insert("https://www.eurora-labs.com".to_string());
        let cfg = Arc::new(OriginGuardConfig::new(origins));
        Router::new()
            .route("/mutate", post(|| async { StatusCode::OK }))
            .layer(axum::middleware::from_fn_with_state(
                cfg,
                origin_guard_middleware,
            ))
    }

    fn mutating_request(origin: Option<&str>) -> Request<Body> {
        let mut builder = Request::builder().method(Method::POST).uri("/mutate");
        if let Some(o) = origin {
            builder = builder.header(header::ORIGIN, o);
        }
        builder.body(Body::empty()).unwrap()
    }

    #[tokio::test]
    async fn allowed_origin_passes() {
        let router = build_router();
        let resp = router
            .oneshot(mutating_request(Some("https://www.eurora-labs.com")))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn disallowed_origin_is_rejected() {
        let router = build_router();
        let resp = router
            .oneshot(mutating_request(Some("https://attacker.example")))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn missing_origin_passes_through() {
        // Server-to-server / curl: no browser ambient credentials to
        // exploit, no allowlist to compare against.
        let router = build_router();
        let resp = router.oneshot(mutating_request(None)).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn bearer_auth_bypasses_guard() {
        let router = build_router();
        let req = Request::builder()
            .method(Method::POST)
            .uri("/mutate")
            .header(header::AUTHORIZATION, "Bearer some.jwt.token")
            // Bearer-mode requests aren't subject to the origin
            // allowlist — even an "evil" origin is fine because the
            // session is carried by the Authorization header, not an
            // ambient cookie.
            .header(header::ORIGIN, "https://attacker.example")
            .body(Body::empty())
            .unwrap();
        let resp = router.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn safe_methods_are_not_checked() {
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
