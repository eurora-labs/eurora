//! Axum middleware that enforces monthly per-user token budgets on the
//! HTTP routes that drive paid LLM usage.
//!
//! Layered *inside* `authz_middleware` so claims are already verified and
//! available in request extensions. If the route doesn't appear in the
//! token-gated registry the middleware is a no-op; otherwise it looks up
//! the caller's monthly token usage and short-circuits with a 429 if the
//! budget has been spent.
//!
//! The state holds an `Arc<dyn TokenUsageRepo>` rather than a concrete
//! type so the binary can swap in `DatabaseManager` (production) or a mock
//! (tests) without recompiling this crate.

use std::sync::Arc;

use axum::extract::{MatchedPath, Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use be_auth_core::Claims;
use uuid::Uuid;

use crate::token_gate::{
    TokenGateError, TokenUsageRepo, check_token_limit_http, is_http_token_gated,
};

/// State injected into [`http_token_gate_middleware`].
#[derive(Clone)]
pub struct HttpTokenGateState {
    pub repo: Arc<dyn TokenUsageRepo>,
}

impl HttpTokenGateState {
    pub fn new(repo: Arc<dyn TokenUsageRepo>) -> Self {
        Self { repo }
    }
}

pub async fn http_token_gate_middleware(
    State(state): State<Arc<HttpTokenGateState>>,
    req: Request,
    next: Next,
) -> Response {
    let method = req.method().clone();
    let matched_path = req
        .extensions()
        .get::<MatchedPath>()
        .map(|m| m.as_str().to_string());

    let Some(matched) = matched_path else {
        // Routes without a MatchedPath (e.g. fallbacks) are never gated;
        // pass through unchanged.
        return next.run(req).await;
    };

    if !is_http_token_gated(&method, &matched) {
        return next.run(req).await;
    }

    let claims = req.extensions().get::<Claims>().cloned();
    let Some(claims) = claims else {
        // Token-gated routes must be authenticated. authz_middleware should
        // have inserted Claims; if it didn't, fail closed with 401.
        return (
            StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({"error": "Missing authenticated claims"})),
        )
            .into_response();
    };

    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({"error": "Invalid user id in claims"})),
            )
                .into_response();
        }
    };

    match check_token_limit_http(&*state.repo, user_id).await {
        Ok(()) => next.run(req).await,
        Err(TokenGateError::Exhausted { .. }) => (
            StatusCode::TOO_MANY_REQUESTS,
            axum::Json(serde_json::json!({
                "error": "token_limit_reached",
                "message": "Monthly token limit reached. Please upgrade your plan.",
            })),
        )
            .into_response(),
        Err(TokenGateError::Internal) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({
                "error": "internal_error",
                "message": "Failed to check token limit",
            })),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Method, Request as HttpRequest};
    use axum::routing::{get, post};
    use be_auth_core::{Claims, Role};
    use be_remote_db::{DbError, DbResult};
    use tower::ServiceExt;

    struct MockRepo {
        used: i64,
        limit: i64,
        error: bool,
    }

    #[async_trait]
    impl TokenUsageRepo for MockRepo {
        async fn get_token_limit_and_usage(
            &self,
            _user_id: Uuid,
            _year_month: i32,
        ) -> DbResult<(i64, i64)> {
            if self.error {
                return Err(DbError::Internal("boom".into()));
            }
            Ok((self.limit, self.used))
        }
    }

    fn build_router(state: Arc<HttpTokenGateState>) -> Router {
        Router::new()
            .route(
                "/threads/{thread_id}/title",
                post(|| async { StatusCode::OK }),
            )
            .route(
                "/threads/{thread_id}/chat",
                get(|| async { StatusCode::OK }),
            )
            .route(
                "/threads",
                post(|| async { StatusCode::OK }).get(|| async { StatusCode::OK }),
            )
            .layer(axum::middleware::from_fn_with_state(
                state,
                http_token_gate_middleware,
            ))
            .layer(axum::middleware::from_fn(inject_claims))
    }

    async fn inject_claims(mut req: Request, next: Next) -> Response {
        let claims = Claims {
            sub: Uuid::new_v4().to_string(),
            email: "u@example.com".into(),
            display_name: None,
            iat: 0,
            exp: i64::MAX,
            token_type: "access".into(),
            role: Role::Free,
            aud: "eurora".into(),
            email_verified: true,
            jti: Uuid::new_v4().to_string(),
        };
        req.extensions_mut().insert(claims);
        next.run(req).await
    }

    fn req(method: Method, uri: &str) -> HttpRequest<Body> {
        HttpRequest::builder()
            .method(method)
            .uri(uri)
            .body(Body::empty())
            .unwrap()
    }

    #[tokio::test]
    async fn ungated_route_passes_through_even_when_exhausted() {
        let state = Arc::new(HttpTokenGateState::new(Arc::new(MockRepo {
            used: 100,
            limit: 100,
            error: false,
        })));
        let router = build_router(state);
        let r = router.oneshot(req(Method::GET, "/threads")).await.unwrap();
        assert_eq!(r.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn gated_route_passes_when_under_budget() {
        let state = Arc::new(HttpTokenGateState::new(Arc::new(MockRepo {
            used: 10,
            limit: 100,
            error: false,
        })));
        let router = build_router(state);
        let r = router
            .oneshot(req(
                Method::POST,
                "/threads/00000000-0000-0000-0000-000000000000/title",
            ))
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn gated_route_rejects_when_exhausted() {
        let state = Arc::new(HttpTokenGateState::new(Arc::new(MockRepo {
            used: 100,
            limit: 100,
            error: false,
        })));
        let router = build_router(state);
        let r = router
            .oneshot(req(
                Method::POST,
                "/threads/00000000-0000-0000-0000-000000000000/title",
            ))
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn chat_websocket_upgrade_path_is_gated() {
        let state = Arc::new(HttpTokenGateState::new(Arc::new(MockRepo {
            used: 100,
            limit: 100,
            error: false,
        })));
        let router = build_router(state);
        let r = router
            .oneshot(req(
                Method::GET,
                "/threads/00000000-0000-0000-0000-000000000000/chat",
            ))
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn db_error_surfaces_as_500() {
        let state = Arc::new(HttpTokenGateState::new(Arc::new(MockRepo {
            used: 0,
            limit: 100,
            error: true,
        })));
        let router = build_router(state);
        let r = router
            .oneshot(req(
                Method::POST,
                "/threads/00000000-0000-0000-0000-000000000000/title",
            ))
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
