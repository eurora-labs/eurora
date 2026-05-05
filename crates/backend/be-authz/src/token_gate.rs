use axum::http::Method;
use be_remote_db::{DatabaseManager, DbResult, year_month_key};
use uuid::Uuid;

/// HTTP routes that consume token budget.
///
/// `(http_method, axum_matched_path)`. The `MatchedPath` form is what the
/// HTTP gate compares against, so the entries must mirror the routes
/// declared in `be-thread-service::create_router` exactly.
const HTTP_TOKEN_GATED_ROUTES: &[(Method, &str)] = &[
    (Method::POST, "/threads/{thread_id}/title"),
    (Method::GET, "/threads/{thread_id}/chat"),
];

/// True if the (method, matched_path) tuple identifies a route whose call
/// must be rejected when the caller is over their monthly token budget.
pub fn is_http_token_gated(method: &Method, matched_path: &str) -> bool {
    HTTP_TOKEN_GATED_ROUTES
        .iter()
        .any(|(m, p)| m == method && *p == matched_path)
}

/// Repository contract for monthly token usage. `DatabaseManager` is the
/// canonical impl; the trait exists so middleware can be unit-tested with
/// a mock and so be-monolith doesn't have to leak its DB type into this
/// crate's public layer surface.
#[async_trait::async_trait]
pub trait TokenUsageRepo: Send + Sync {
    async fn get_token_limit_and_usage(
        &self,
        user_id: Uuid,
        year_month: i32,
    ) -> DbResult<(i64, i64)>;
}

#[async_trait::async_trait]
impl TokenUsageRepo for DatabaseManager {
    async fn get_token_limit_and_usage(
        &self,
        user_id: Uuid,
        year_month: i32,
    ) -> DbResult<(i64, i64)> {
        self.get_token_limit_and_usage()
            .user_id(user_id)
            .year_month(year_month)
            .call()
            .await
    }
}

#[async_trait::async_trait]
impl<T: TokenUsageRepo> TokenUsageRepo for std::sync::Arc<T> {
    async fn get_token_limit_and_usage(
        &self,
        user_id: Uuid,
        year_month: i32,
    ) -> DbResult<(i64, i64)> {
        (**self)
            .get_token_limit_and_usage(user_id, year_month)
            .await
    }
}

/// Outcome of an HTTP token-limit check. Convertible to an axum response
/// upstream — kept as a dedicated enum so the wire layer can be picked at
/// the call site.
#[derive(Debug, Clone)]
pub enum TokenGateError {
    /// Caller has exceeded their monthly budget.
    Exhausted { used: i64, limit: i64 },
    /// Database lookup failed.
    Internal,
}

pub async fn check_token_limit_http(
    db: &(impl TokenUsageRepo + ?Sized),
    user_id: Uuid,
) -> Result<(), TokenGateError> {
    let now = chrono::Utc::now();
    let year_month = year_month_key(&now);

    let (limit, used) = db
        .get_token_limit_and_usage(user_id, year_month)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check token limit");
            TokenGateError::Internal
        })?;

    if used >= limit {
        tracing::warn!(used, limit, "Token limit reached");
        return Err(TokenGateError::Exhausted { used, limit });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use be_remote_db::DbError;

    #[test]
    fn http_gated_routes_match() {
        assert!(is_http_token_gated(
            &Method::POST,
            "/threads/{thread_id}/title"
        ));
        assert!(is_http_token_gated(
            &Method::GET,
            "/threads/{thread_id}/chat"
        ));
    }

    #[test]
    fn http_non_gated_routes_pass() {
        assert!(!is_http_token_gated(&Method::GET, "/threads"));
        assert!(!is_http_token_gated(&Method::POST, "/threads"));
        assert!(!is_http_token_gated(
            &Method::GET,
            "/threads/{thread_id}/messages"
        ));
        // Method must match exactly.
        assert!(!is_http_token_gated(
            &Method::POST,
            "/threads/{thread_id}/chat"
        ));
    }

    struct MockRepo {
        limit: i64,
        used: i64,
        error: Option<DbError>,
    }

    impl MockRepo {
        fn ok(limit: i64, used: i64) -> Self {
            Self {
                limit,
                used,
                error: None,
            }
        }

        fn err(error: DbError) -> Self {
            Self {
                limit: 0,
                used: 0,
                error: Some(error),
            }
        }
    }

    #[async_trait::async_trait]
    impl TokenUsageRepo for MockRepo {
        async fn get_token_limit_and_usage(
            &self,
            _user_id: Uuid,
            _year_month: i32,
        ) -> DbResult<(i64, i64)> {
            if let Some(ref e) = self.error {
                return Err(DbError::Internal(e.to_string()));
            }
            Ok((self.limit, self.used))
        }
    }

    #[tokio::test]
    async fn check_token_limit_enforces_limit() {
        let repo = MockRepo::ok(1000, 1000);
        assert!(matches!(
            check_token_limit_http(&repo, Uuid::nil()).await,
            Err(TokenGateError::Exhausted { .. })
        ));

        let repo = MockRepo::ok(1000, 1500);
        assert!(matches!(
            check_token_limit_http(&repo, Uuid::nil()).await,
            Err(TokenGateError::Exhausted { .. })
        ));

        let repo = MockRepo::ok(1000, 999);
        assert!(check_token_limit_http(&repo, Uuid::nil()).await.is_ok());
    }

    #[tokio::test]
    async fn check_token_limit_zero_limit_blocks() {
        let repo = MockRepo::ok(0, 0);
        assert!(matches!(
            check_token_limit_http(&repo, Uuid::nil()).await,
            Err(TokenGateError::Exhausted { .. })
        ));
    }

    #[tokio::test]
    async fn check_token_limit_maps_db_error() {
        let repo = MockRepo::err(DbError::connection("db went away"));
        assert!(matches!(
            check_token_limit_http(&repo, Uuid::nil()).await,
            Err(TokenGateError::Internal)
        ));
    }
}
