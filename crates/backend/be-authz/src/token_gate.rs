use be_remote_db::{DatabaseManager, DbResult, year_month_key};
use tonic::Status;
use uuid::Uuid;

const TOKEN_GATED_METHODS: &[(&str, &str)] = &[
    ("thread_service.ProtoThreadService", "ChatStream"),
    ("thread_service.ProtoThreadService", "GenerateThreadTitle"),
];

pub(crate) fn is_token_gated(service_full: &str, method: &str) -> bool {
    TOKEN_GATED_METHODS
        .iter()
        .any(|(s, m)| *s == service_full && *m == method)
}

#[async_trait::async_trait]
pub(crate) trait TokenUsageRepo: Send + Sync {
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
        self.get_token_limit_and_usage(user_id, year_month).await
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

pub(crate) async fn check_token_limit(
    db: &impl TokenUsageRepo,
    user_id: Uuid,
) -> Result<(), Status> {
    let now = chrono::Utc::now();
    let year_month = year_month_key(&now);

    let (limit, used) = db
        .get_token_limit_and_usage(user_id, year_month)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check token limit");
            Status::internal("Failed to check token limit")
        })?;

    if used >= limit {
        tracing::warn!(user_id = %user_id, used, limit, "Token limit reached");
        return Err(Status::resource_exhausted(
            "Monthly token limit reached. Please upgrade your plan.",
        ));
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
            "activity_service.ProtoActivityService",
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
