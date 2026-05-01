use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{ConnectInfo, MatchedPath, Request};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use be_auth_core::JwtConfig;

use crate::CasbinAuthz;
use crate::bypass::is_rest_bypass;
use crate::rate_limit::{self, AuthFailureRateLimiter, HealthCheckRateLimiter, TrustedProxies};

pub struct AuthzState {
    pub authz: CasbinAuthz,
    pub jwt_config: JwtConfig,
    pub rate_limiter: AuthFailureRateLimiter,
    pub health_rate_limiter: HealthCheckRateLimiter,
    pub trusted_proxies: TrustedProxies,
}

impl AuthzState {
    pub fn new(
        authz: CasbinAuthz,
        jwt_config: JwtConfig,
        rate_limiter: AuthFailureRateLimiter,
        health_rate_limiter: HealthCheckRateLimiter,
        trusted_proxies: TrustedProxies,
    ) -> Self {
        Self {
            authz,
            jwt_config,
            rate_limiter,
            health_rate_limiter,
            trusted_proxies,
        }
    }
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

    let peer_addr = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.ip());
    let client_ip = rate_limit::extract_client_ip(req.headers(), peer_addr, &state.trusted_proxies);

    if is_rest_bypass(&raw_path) {
        if raw_path == "/health" && state.health_rate_limiter.check_key(&client_ip).is_err() {
            tracing::warn!(ip = %client_ip, "Rate limited health check request");
            return too_many_requests_response();
        }
        tracing::debug!(path = %raw_path, "Bypassing authorization for public route");
        return next.run(req).await;
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
            if state.rate_limiter.check_key(&client_ip).is_err() {
                return too_many_requests_response();
            }
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
            if state.rate_limiter.check_key(&client_ip).is_err() {
                return too_many_requests_response();
            }
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
            if state.rate_limiter.check_key(&client_ip).is_err() {
                return too_many_requests_response();
            }
            tracing::warn!(error = %e, "JWT validation failed");
            return (
                StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({"error": "Invalid or expired token"})),
            )
                .into_response();
        }
    };

    if !claims.email_verified {
        return (
            StatusCode::FORBIDDEN,
            axum::Json(
                serde_json::json!({"error": "Email verification required. Please check your inbox."}),
            ),
        )
            .into_response();
    }

    let role = claims.role.to_string();

    match state.authz.enforce(&role, &policy_path, &method) {
        Ok(true) => {
            tracing::debug!(role = %role, path = %raw_path, method = %method, "REST authorized");
            req.extensions_mut().insert(claims);
            next.run(req).await
        }
        Ok(false) => {
            if state.rate_limiter.check_key(&client_ip).is_err() {
                return too_many_requests_response();
            }
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

#[cfg(test)]
mod tests {
    //! These tests pin a contract that has bitten production: any short-circuit
    //! response produced inside `authz_middleware` (401 missing token, 403
    //! email-verified, 403 RBAC, 429 rate limit) must propagate through outer
    //! layers like CORS. A reversed layer order silently strips
    //! `Access-Control-*` headers from error responses, causing browsers to
    //! report the failure as a generic "Failed to fetch" instead of the real
    //! HTTP status — see `be-monolith/src/main.rs` for the production wiring.
    use super::*;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{HeaderValue, Method, Request, StatusCode, header};
    use axum::routing::post;
    use be_auth_core::{Claims, JwtConfig, Role};
    use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, encode};
    use tower::ServiceExt;
    use tower_http::cors::{AllowOrigin, CorsLayer};

    use crate::rate_limit::{
        TrustedProxies, new_auth_failure_rate_limiter, new_health_check_rate_limiter,
    };

    const TEST_ALLOWED_ORIGIN: &str = "https://www.eurora-labs.com";
    const TEST_JWT_SECRET: &[u8] = b"test-secret-do-not-use-in-production";

    fn build_test_jwt_config() -> JwtConfig {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_audience(&["eurora"]);
        validation.required_spec_claims.insert("aud".to_string());
        JwtConfig {
            access_token_encoding_key: EncodingKey::from_secret(TEST_JWT_SECRET),
            access_token_decoding_key: DecodingKey::from_secret(TEST_JWT_SECRET),
            refresh_token_encoding_key: EncodingKey::from_secret(TEST_JWT_SECRET),
            refresh_token_decoding_key: DecodingKey::from_secret(TEST_JWT_SECRET),
            access_token_expiry_hours: 1,
            refresh_token_expiry_days: 7,
            validation,
            approved_emails: vec![],
        }
    }

    fn mint_access_token(jwt_config: &JwtConfig, email_verified: bool) -> String {
        let now = chrono::Utc::now();
        let claims = Claims {
            sub: uuid::Uuid::new_v4().to_string(),
            email: "user@example.com".to_string(),
            display_name: None,
            iat: now.timestamp(),
            exp: (now + chrono::Duration::hours(1)).timestamp(),
            token_type: "access".to_string(),
            role: Role::Free,
            aud: "eurora".to_string(),
            email_verified,
            jti: uuid::Uuid::new_v4().to_string(),
        };
        encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &jwt_config.access_token_encoding_key,
        )
        .expect("failed to encode test JWT")
    }

    async fn build_router(jwt_config: JwtConfig) -> Router {
        let base = env!("CARGO_MANIFEST_DIR");
        let model = format!("{base}/../../../config/authz/model.conf");
        let policy = format!("{base}/../../../config/authz/policy.csv");
        let authz = CasbinAuthz::new(&model, &policy)
            .await
            .expect("failed to init enforcer");

        let state = Arc::new(AuthzState::new(
            authz,
            jwt_config,
            new_auth_failure_rate_limiter(),
            new_health_check_rate_limiter(),
            TrustedProxies::new(vec![]),
        ));

        let cors = CorsLayer::new()
            .allow_origin(AllowOrigin::list([HeaderValue::from_static(
                TEST_ALLOWED_ORIGIN,
            )]))
            .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
            .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]);

        // Mirror the production layer order from `be-monolith/src/main.rs`:
        // authz inside, CORS outermost.
        Router::new()
            .route("/payment/checkout", post(|| async { StatusCode::OK }))
            .layer(axum::middleware::from_fn_with_state(
                state,
                authz_middleware,
            ))
            .layer(cors)
    }

    fn request_with_origin(method: Method, uri: &str, auth: Option<&str>) -> Request<Body> {
        let mut builder = Request::builder()
            .method(method)
            .uri(uri)
            .header(header::ORIGIN, TEST_ALLOWED_ORIGIN);
        if let Some(token) = auth {
            builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        builder
            .body(Body::empty())
            .expect("failed to build request")
    }

    #[tokio::test]
    async fn missing_authorization_header_response_carries_cors_header() {
        let router = build_router(build_test_jwt_config()).await;
        let response = router
            .oneshot(request_with_origin(Method::POST, "/payment/checkout", None))
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            response
                .headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .and_then(|v| v.to_str().ok()),
            Some(TEST_ALLOWED_ORIGIN),
            "401 from authz must propagate through outer CORS layer; \
             a missing Access-Control-Allow-Origin header surfaces in browsers \
             as a generic 'Failed to fetch'"
        );
    }

    #[tokio::test]
    async fn unverified_email_response_carries_cors_header() {
        let jwt_config = build_test_jwt_config();
        let token = mint_access_token(&jwt_config, false);
        let router = build_router(jwt_config).await;

        let response = router
            .oneshot(request_with_origin(
                Method::POST,
                "/payment/checkout",
                Some(&token),
            ))
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(
            response
                .headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .and_then(|v| v.to_str().ok()),
            Some(TEST_ALLOWED_ORIGIN),
            "403 email-verification denial must propagate through outer CORS layer"
        );
    }

    #[tokio::test]
    async fn verified_email_request_is_authorized() {
        let jwt_config = build_test_jwt_config();
        let token = mint_access_token(&jwt_config, true);
        let router = build_router(jwt_config).await;

        let response = router
            .oneshot(request_with_origin(
                Method::POST,
                "/payment/checkout",
                Some(&token),
            ))
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .and_then(|v| v.to_str().ok()),
            Some(TEST_ALLOWED_ORIGIN)
        );
    }
}
