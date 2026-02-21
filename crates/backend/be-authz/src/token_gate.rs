use be_remote_db::DatabaseManager;
use chrono::Datelike;
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

pub(crate) async fn check_token_limit(db: &DatabaseManager, user_id: Uuid) -> Result<(), Status> {
    let now = chrono::Utc::now();

    let token_limit = db.get_token_limit_for_user(user_id).await.map_err(|e| {
        tracing::error!("Failed to query token limit: {}", e);
        Status::internal("Failed to check token limit")
    })?;

    if let Some(limit) = token_limit {
        let used = db
            .get_monthly_token_usage(user_id, now.year(), now.month())
            .await
            .map_err(|e| {
                tracing::error!("Failed to query token usage: {}", e);
                Status::internal("Failed to check token usage")
            })?;

        if used >= limit {
            tracing::warn!(user_id = %user_id, used = used, limit = limit, "Token limit reached");
            return Err(Status::resource_exhausted(
                "Monthly token limit reached. Please upgrade your plan.",
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
