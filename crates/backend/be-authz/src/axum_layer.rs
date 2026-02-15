use std::sync::Arc;

use axum::extract::{MatchedPath, Request};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use be_auth_core::JwtConfig;
use tracing::{debug, warn};

use crate::CasbinAuthz;
use crate::bypass::is_rest_bypass;

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
    let raw_path = req.uri().path().to_string();
    let method = req.method().to_string();

    if is_rest_bypass(&raw_path) {
        debug!(path = %raw_path, "Bypassing authorization for public route");
        return next.run(req).await;
    }

    // Use the route template (e.g. "/payment/checkout") for policy matching instead
    // of the concrete path (e.g. "/payment/subscription/sub_123"). This ensures
    // routes with path parameters match their policy entries correctly.
    let policy_path = req
        .extensions()
        .get::<MatchedPath>()
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| raw_path.clone());

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
            warn!(error = %e, "JWT validation failed");
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
            debug!(role = %role, path = %raw_path, method = %method, "REST authorized");
            req.extensions_mut().insert(claims);
            next.run(req).await
        }
        Ok(false) => {
            warn!(role = %role, path = %raw_path, method = %method, "REST authorization denied");
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
