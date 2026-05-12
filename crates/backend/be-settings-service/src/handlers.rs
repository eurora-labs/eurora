use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode};
use be_auth_core::AuthUser;
use be_remote_db::{UpsertOutcome, UserSettingsRow};
use settings_core::{
    GetSettingsResponse, PutSettingsAcceptedResponse, PutSettingsConflictResponse,
    PutSettingsRequest,
};

use crate::AppState;
use crate::error::{SettingsResult, SettingsServiceError};
use crate::response::PutOutcomeResponse;

#[tracing::instrument(skip_all, fields(user_id))]
pub async fn get_settings(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> SettingsResult<Json<GetSettingsResponse>> {
    let user_id = user.user_id()?;
    tracing::Span::current().record("user_id", tracing::field::display(user_id));

    let row = state
        .db
        .get_user_settings()
        .user_id(user_id)
        .call()
        .await?
        .ok_or(SettingsServiceError::NotFound)?;

    let schema_version = schema_version_to_wire(row.schema_version)?;
    Ok(Json(GetSettingsResponse {
        schema_version,
        updated_at: row.updated_at,
        settings: row.settings,
    }))
}

#[tracing::instrument(skip_all, fields(user_id, schema_version, has_base))]
pub async fn put_settings(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(body): Json<PutSettingsRequest>,
) -> SettingsResult<PutOutcomeResponse> {
    let user_id = user.user_id()?;

    let schema_version = i32::try_from(body.schema_version).map_err(|_| {
        SettingsServiceError::invalid_argument(format!(
            "schemaVersion {} does not fit in a signed 32-bit integer",
            body.schema_version
        ))
    })?;

    let span = tracing::Span::current();
    span.record("user_id", tracing::field::display(user_id));
    span.record("schema_version", schema_version);
    span.record("has_base", body.base_updated_at.is_some());

    let outcome = state
        .db
        .upsert_user_settings()
        .user_id(user_id)
        .schema_version(schema_version)
        .settings(body.settings)
        .maybe_base_updated_at(body.base_updated_at)
        .call()
        .await?;

    Ok(match outcome {
        UpsertOutcome::Inserted(row) => {
            tracing::info!(updated_at = %row.updated_at, "Inserted settings row");
            PutOutcomeResponse::Accepted(accepted_from(row)?)
        }
        UpsertOutcome::Updated(row) => {
            tracing::info!(updated_at = %row.updated_at, "Updated settings row");
            PutOutcomeResponse::Accepted(accepted_from(row)?)
        }
        UpsertOutcome::Conflict { current } => {
            tracing::info!(
                current_updated_at = %current.updated_at,
                "Settings upsert conflict — returning current row"
            );
            PutOutcomeResponse::Conflict(conflict_from(current)?)
        }
    })
}

#[tracing::instrument(skip_all, fields(user_id))]
pub async fn delete_settings(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> SettingsResult<StatusCode> {
    let user_id = user.user_id()?;
    tracing::Span::current().record("user_id", tracing::field::display(user_id));

    state
        .db
        .delete_user_settings()
        .user_id(user_id)
        .call()
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Lift the stored `schema_version` back into the wire-level `u32`.
///
/// Negative values are unreachable through the normal write path because
/// `put_settings` runs every inbound `u32` through `i32::try_from` before
/// hitting the DB, and the column carries a `CHECK (schema_version >= 0)`
/// constraint. A failure here therefore indicates external tampering with
/// the table and is reported as an internal error rather than wrapped
/// silently by an `as` cast.
fn schema_version_to_wire(stored: i32) -> SettingsResult<u32> {
    u32::try_from(stored).map_err(|_| {
        SettingsServiceError::internal(format!(
            "user_settings.schema_version stored as negative value: {stored}"
        ))
    })
}

fn accepted_from(row: UserSettingsRow) -> SettingsResult<PutSettingsAcceptedResponse> {
    Ok(PutSettingsAcceptedResponse {
        schema_version: schema_version_to_wire(row.schema_version)?,
        updated_at: row.updated_at,
    })
}

fn conflict_from(row: UserSettingsRow) -> SettingsResult<PutSettingsConflictResponse> {
    Ok(PutSettingsConflictResponse {
        schema_version: schema_version_to_wire(row.schema_version)?,
        updated_at: row.updated_at,
        current: row.settings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_version_to_wire_passes_through_non_negative() {
        assert_eq!(schema_version_to_wire(0).unwrap(), 0);
        assert_eq!(schema_version_to_wire(1).unwrap(), 1);
        assert_eq!(schema_version_to_wire(i32::MAX).unwrap(), i32::MAX as u32);
    }

    #[test]
    fn schema_version_to_wire_rejects_negative() {
        let err = schema_version_to_wire(-1).expect_err("negative must be rejected");
        assert!(matches!(err, SettingsServiceError::Internal(_)));
    }
}
