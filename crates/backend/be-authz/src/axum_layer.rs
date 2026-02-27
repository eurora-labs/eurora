use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use axum::extract::{ConnectInfo, MatchedPath, Request};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use be_auth_core::JwtConfig;

use crate::CasbinAuthz;
use crate::bypass::is_rest_bypass;
use crate::rate_limit::{AuthFailureRateLimiter, HealthCheckRateLimiter};

pub struct AuthzState {
    pub authz: CasbinAuthz,
    pub jwt_config: JwtConfig,
    pub rate_limiter: AuthFailureRateLimiter,
    pub health_rate_limiter: HealthCheckRateLimiter,
}

impl AuthzState {
    pub fn new(
        authz: CasbinAuthz,
        jwt_config: JwtConfig,
        rate_limiter: AuthFailureRateLimiter,
        health_rate_limiter: HealthCheckRateLimiter,
    ) -> Self {
        Self {
            authz,
            jwt_config,
            rate_limiter,
            health_rate_limiter,
        }
    }
}

fn extract_client_ip(req: &Request) -> IpAddr {
    req.extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.ip())
        .unwrap_or(IpAddr::from([127, 0, 0, 1]))
}

fn too_many_requests_response() -> Response {
    (
        StatusCode::TOO_MANY_REQUESTS,
        axum::Json(serde_json::json!({"error": "Too many failed requests. Try again later."})),
    )
        .into_response()
}

pub async fn authz_middleware(
    axum::extract::State(state): axum::extract::State<Arc<AuthzState>>,
    mut req: Request,
    next: Next,
) -> Response {
    if req.method() == axum::http::Method::OPTIONS {
        return next.run(req).await;
    }

    let raw_path = req.uri().path().to_string();
    let method = req.method().to_string();

    let client_ip = extract_client_ip(&req);

    if is_rest_bypass(&raw_path) {
        if raw_path == "/health" && state.health_rate_limiter.check_key(&client_ip).is_err() {
            tracing::warn!(ip = %client_ip, "Rate limited health check request");
            return too_many_requests_response();
        }
        tracing::debug!(path = %raw_path, "Bypassing authorization for public route");
        return next.run(req).await;
    }

    if state.rate_limiter.check_key(&client_ip).is_err() {
        tracing::warn!(ip = %client_ip, "Rate limited — too many auth failures");
        return too_many_requests_response();
    }

    let policy_path = match req.extensions().get::<MatchedPath>() {
        Some(m) => m.as_str().to_string(),
        None => {
            tracing::warn!(
                path = %raw_path,
                "MatchedPath missing from request extensions, falling back to raw URI \
                 — parameterized routes may fail policy matching"
            );
            raw_path.clone()
        }
    };

    let auth_header = match req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
    {
        Some(h) => h.to_string(),
        None => {
            let _ = state.rate_limiter.check_key(&client_ip);
            return (
                StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({"error": "Missing authorization header"})),
            )
                .into_response();
        }
    };

    let token = match auth_header.strip_prefix("Bearer ") {
        Some(t) => t,
        None => {
            let _ = state.rate_limiter.check_key(&client_ip);
            return (
                StatusCode::UNAUTHORIZED,
                axum::Json(
                    serde_json::json!({"error": "Authorization header must start with 'Bearer '"}),
                ),
            )
                .into_response();
        }
    };

    let claims = match state.jwt_config.validate_access_token(token) {
        Ok(c) => c,
        Err(e) => {
            let _ = state.rate_limiter.check_key(&client_ip);
            tracing::warn!(error = %e, "JWT validation failed");
            return (
                StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({"error": "Invalid or expired token"})),
            )
                .into_response();
        }
    };

    let role = claims.role.to_string();

    match state.authz.enforce(&role, &policy_path, &method) {
        Ok(true) => {
            tracing::debug!(role = %role, path = %raw_path, method = %method, "REST authorized");
            req.extensions_mut().insert(claims);
            next.run(req).await
        }
        Ok(false) => {
            let _ = state.rate_limiter.check_key(&client_ip);
            tracing::warn!(role = %role, path = %raw_path, method = %method, "REST authorization denied");
            (
                StatusCode::FORBIDDEN,
                axum::Json(
                    serde_json::json!({"error": "Insufficient permissions. Please upgrade your plan."}),
                ),
            )
                .into_response()
        }
        Err(e) => {
            tracing::warn!(error = %e, "REST authorization enforcement error");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({"error": "Authorization error"})),
            )
                .into_response()
        }
    }
}
