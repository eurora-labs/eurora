use std::sync::Arc;

use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use be_auth_core::JwtConfig;
use tracing::{debug, warn};

use crate::CasbinAuthz;

/// Path prefixes that bypass authorization entirely (public/webhook routes).
const BYPASS_PREFIXES: &[&str] = &["/releases/", "/extensions/"];
const BYPASS_EXACT: &[&str] = &["/payment/webhook"];

/// Shared state for the axum authz middleware.
pub struct AuthzState {
    pub authz: CasbinAuthz,
    pub jwt_config: JwtConfig,
}

impl AuthzState {
    pub fn new(authz: CasbinAuthz, jwt_config: JwtConfig) -> Self {
        Self { authz, jwt_config }
    }
}

/// Axum middleware that validates JWT and enforces casbin policy on REST routes.
pub async fn authz_middleware(
    axum::extract::State(state): axum::extract::State<Arc<AuthzState>>,
    mut req: Request,
    next: Next,
) -> Response {
    let path = req.uri().path().to_string();
    let method = req.method().to_string();

    if BYPASS_PREFIXES
        .iter()
        .any(|prefix| path.starts_with(prefix))
        || BYPASS_EXACT.iter().any(|exact| path == *exact)
    {
        debug!(path = %path, "Bypassing authorization for public route");
        return next.run(req).await;
    }

    let auth_header = match req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
    {
        Some(h) => h.to_string(),
        None => {
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
            return (
                StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response();
        }
    };

    let role = claims.role.to_string();

    match state.authz.enforce(&role, &path, &method).await {
        Ok(true) => {
            debug!(role = %role, path = %path, method = %method, "REST authorized");
            req.extensions_mut().insert(claims);
            next.run(req).await
        }
        Ok(false) => {
            warn!(role = %role, path = %path, method = %method, "REST authorization denied");
            (
                StatusCode::FORBIDDEN,
                axum::Json(
                    serde_json::json!({"error": "Insufficient permissions. Please upgrade your plan."}),
                ),
            )
                .into_response()
        }
        Err(e) => {
            warn!(error = %e, "REST authorization enforcement error");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({"error": "Authorization error"})),
            )
                .into_response()
        }
    }
}
