use axum::http::Method;
use be_remote_db::{DatabaseManager, DbResult, year_month_key};
use tonic::Status;
use uuid::Uuid;

const TOKEN_GATED_METHODS: &[(&str, &str)] = &[
    ("thread_service.ProtoThreadService", "ChatStream"),
    ("thread_service.ProtoThreadService", "GenerateThreadTitle"),
];

/// HTTP routes that consume token budget.
///
/// `(http_method, axum_matched_path)`. The `MatchedPath` form is what the
/// HTTP gate compares against, so the entries must mirror the routes
/// declared in `be-thread-service::create_router` exactly.
const HTTP_TOKEN_GATED_ROUTES: &[(Method, &str)] = &[
    (Method::POST, "/threads/{thread_id}/title"),
    (Method::GET, "/threads/{thread_id}/chat"),
];

pub(crate) fn is_token_gated(service_full: &str, method: &str) -> bool {
    TOKEN_GATED_METHODS
        .iter()
        .any(|(s, m)| *s == service_full && *m == method)
}

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
/// upstream — kept as a dedicated enum (rather than [`tonic::Status`] or an
/// `axum::http::StatusCode`) so the wire layer can be picked at the call
/// site without dragging gRPC types into the HTTP path.
#[derive(Debug, Clone)]
pub enum TokenGateError {
    /// Caller has exceeded their monthly budget.
    Exhausted { used: i64, limit: i64 },
    /// Database lookup failed.
    Internal,
}

pub(crate) async fn check_token_limit(
    db: &impl TokenUsageRepo,
    user_id: Uuid,
) -> Result<(), Status> {
    match check_token_limit_inner(db, user_id).await {
        Ok(()) => Ok(()),
        Err(TokenGateError::Exhausted { .. }) => Err(Status::resource_exhausted(
            "Monthly token limit reached. Please upgrade your plan.",
        )),
        Err(TokenGateError::Internal) => Err(Status::internal("Failed to check token limit")),
    }
}

/// Same check, surfaced as a [`TokenGateError`] for the HTTP middleware path.
pub async fn check_token_limit_http(
    db: &(impl TokenUsageRepo + ?Sized),
    user_id: Uuid,
) -> Result<(), TokenGateError> {
    check_token_limit_inner(db, user_id).await
}

async fn check_token_limit_inner(
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
    fn gated_methods_match() {
        assert!(is_token_gated(
            "thread_service.ProtoThreadService",
            "ChatStream"
        ));
        assert!(is_token_gated(
            "thread_service.ProtoThreadService",
            "GenerateThreadTitle"
        ));
    }

    #[test]
    fn non_gated_methods_pass() {
        assert!(!is_token_gated(
            "thread_service.ProtoThreadService",
            "ListThreads"
        ));
        assert!(!is_token_gated(
            "thread_service.ProtoThreadService",
            "GetMessages"
        ));
        assert!(!is_token_gated(
            "auth_service.ProtoAuthService",
            "ChatStream"
        ));
        assert!(!is_token_gated("", ""));
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
        let err = check_token_limit(&repo, Uuid::nil()).await.unwrap_err();
        assert_eq!(err.code(), tonic::Code::ResourceExhausted);

        let repo = MockRepo::ok(1000, 1500);
        let err = check_token_limit(&repo, Uuid::nil()).await.unwrap_err();
        assert_eq!(err.code(), tonic::Code::ResourceExhausted);

        let repo = MockRepo::ok(1000, 999);
        assert!(check_token_limit(&repo, Uuid::nil()).await.is_ok());
    }

    #[tokio::test]
    async fn check_token_limit_zero_limit_blocks() {
        let repo = MockRepo::ok(0, 0);
        let err = check_token_limit(&repo, Uuid::nil()).await.unwrap_err();
        assert_eq!(err.code(), tonic::Code::ResourceExhausted);
    }

    #[tokio::test]
    async fn check_token_limit_maps_db_error() {
        let repo = MockRepo::err(DbError::connection("db went away"));
        let err = check_token_limit(&repo, Uuid::nil()).await.unwrap_err();
        assert_eq!(err.code(), tonic::Code::Internal);
    }
}
