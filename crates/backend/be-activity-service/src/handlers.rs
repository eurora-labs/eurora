use std::sync::Arc;

use activity_core::{
    Activity as WireActivity, DEFAULT_LIST_LIMIT, InsertActivityRequest, InsertActivityResponse,
    ListActivitiesQuery, ListActivitiesResponse, MAX_LIST_LIMIT, UpdateActivityRequest,
    UpdateActivityResponse,
};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use be_asset::CreateAssetInput;
use be_auth_core::AuthUser;
use be_remote_db::PaginationParams;
use uuid::Uuid;

use crate::analytics;
use crate::error::{ActivityResult, ActivityServiceError};
use crate::service::AppState;

const ICON_MIME_TYPE: &str = "image/png";

#[tracing::instrument(skip_all, fields(user_id, limit, offset))]
pub async fn list_activities(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Query(query): Query<ListActivitiesQuery>,
) -> ActivityResult<Json<ListActivitiesResponse>> {
    let user_id = user.user_id()?;

    let limit = query.limit.unwrap_or(DEFAULT_LIST_LIMIT);
    let offset = query.offset.unwrap_or(0);

    if limit > MAX_LIST_LIMIT {
        return Err(ActivityServiceError::invalid_argument(format!(
            "limit must be <= {MAX_LIST_LIMIT}"
        )));
    }

    let span = tracing::Span::current();
    span.record("user_id", tracing::field::display(user_id));
    span.record("limit", limit);
    span.record("offset", offset);

    let activities = state
        .db
        .list_activities()
        .user_id(user_id)
        .params(PaginationParams::new(offset, limit, "DESC"))
        .call()
        .await
        .map_err(|e| {
            let err = ActivityServiceError::from(e);
            analytics::track_activities_list_failed(err.error_kind());
            err
        })?;

    let result_count = activities.len();
    tracing::debug!(result_count, "Listed activities");
    analytics::track_activities_listed(limit, offset, result_count);

    Ok(Json(ListActivitiesResponse {
        activities: activities.into_iter().map(db_to_wire).collect(),
    }))
}

#[tracing::instrument(skip_all, fields(user_id, has_icon, has_ended_at))]
pub async fn insert_activity(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(body): Json<InsertActivityRequest>,
) -> ActivityResult<Json<InsertActivityResponse>> {
    let user_id = user.user_id()?;

    let icon_bytes = decode_optional_icon(body.icon_png_base64.as_deref()).inspect_err(|e| {
        analytics::track_activity_insert_failed(e.error_kind());
    })?;

    let has_icon = icon_bytes.is_some();
    let has_ended_at = body.ended_at.is_some();

    let span = tracing::Span::current();
    span.record("user_id", tracing::field::display(user_id));
    span.record("has_icon", has_icon);
    span.record("has_ended_at", has_ended_at);

    // Upload the icon first so the activity row is the last write. If the
    // asset upload fails, no activity is created; if the activity insert
    // fails after a successful upload, we leak an asset blob (cleanable by
    // a sweeper) but never persist a half-built activity.
    let activity_id = body.id.unwrap_or_else(Uuid::now_v7);
    let icon_asset_id = match icon_bytes {
        Some(content) => Some(
            state
                .asset_service
                .create_asset(
                    CreateAssetInput {
                        name: format!("activity-icon-{activity_id}"),
                        content,
                        mime_type: ICON_MIME_TYPE.to_string(),
                        metadata: None,
                        activity_id: None,
                    },
                    user_id,
                )
                .await
                .map_err(|e| {
                    let err = ActivityServiceError::from(e);
                    analytics::track_activity_insert_failed(err.error_kind());
                    err
                })?
                .id,
        ),
        None => None,
    };

    let activity = state
        .db
        .create_activity()
        .id(activity_id)
        .user_id(user_id)
        .name(body.name)
        .process_name(body.process_name)
        .window_title(body.window_title)
        .maybe_icon_asset_id(icon_asset_id)
        .started_at(body.started_at)
        .maybe_ended_at(body.ended_at)
        .call()
        .await
        .map_err(|e| {
            let err = ActivityServiceError::from(e);
            analytics::track_activity_insert_failed(err.error_kind());
            err
        })?;

    tracing::info!(activity_id = %activity.id, "Created activity");
    analytics::track_activity_inserted(has_icon, has_ended_at, &activity.process_name);

    Ok(Json(InsertActivityResponse {
        activity: db_to_wire(activity),
    }))
}

#[tracing::instrument(skip_all, fields(user_id, activity_id = %activity_id, has_ended_at, has_window_title, has_name))]
pub async fn patch_activity(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(activity_id): Path<Uuid>,
    Json(body): Json<UpdateActivityRequest>,
) -> ActivityResult<Json<UpdateActivityResponse>> {
    let user_id = user.user_id()?;

    let span = tracing::Span::current();
    span.record("user_id", tracing::field::display(user_id));
    span.record("has_ended_at", body.ended_at.is_some());
    span.record("has_window_title", body.window_title.is_some());
    span.record("has_name", body.name.is_some());

    let UpdateActivityRequest {
        name,
        window_title,
        ended_at,
    } = body;

    if name.is_none() && window_title.is_none() && ended_at.is_none() {
        return Err(ActivityServiceError::invalid_argument(
            "PATCH body must set at least one of: name, window_title, ended_at",
        ));
    }

    let set_name = name.is_some();
    let set_window_title = window_title.is_some();
    let set_ended_at = ended_at.is_some();

    let activity = state
        .db
        .update_activity()
        .id(activity_id)
        .user_id(user_id)
        .maybe_name(name)
        .maybe_window_title(window_title)
        .maybe_ended_at(ended_at)
        .call()
        .await
        .map_err(|e| {
            let err = ActivityServiceError::from(e);
            analytics::track_activity_update_failed(err.error_kind());
            err
        })?;

    tracing::debug!(activity_id = %activity.id, "Patched activity");
    analytics::track_activity_updated(set_ended_at, set_window_title, set_name);

    Ok(Json(UpdateActivityResponse {
        activity: db_to_wire(activity),
    }))
}

fn decode_optional_icon(b64: Option<&str>) -> ActivityResult<Option<Vec<u8>>> {
    match b64 {
        Some(s) if !s.is_empty() => BASE64_STANDARD
            .decode(s)
            .map(Some)
            .map_err(|e| ActivityServiceError::invalid_base64("icon_png_base64", e)),
        _ => Ok(None),
    }
}

fn db_to_wire(activity: be_remote_db::Activity) -> WireActivity {
    WireActivity {
        id: activity.id,
        name: activity.name,
        process_name: activity.process_name,
        window_title: activity.window_title,
        icon_asset_id: activity.icon_asset_id,
        started_at: activity.started_at,
        ended_at: activity.ended_at,
        created_at: activity.created_at,
        updated_at: activity.updated_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_optional_icon_handles_none_and_empty() {
        assert!(decode_optional_icon(None).unwrap().is_none());
        assert!(decode_optional_icon(Some("")).unwrap().is_none());
    }

    #[test]
    fn decode_optional_icon_decodes_standard_base64() {
        let payload = b"PNG-bytes";
        let encoded = BASE64_STANDARD.encode(payload);
        assert_eq!(
            decode_optional_icon(Some(&encoded)).unwrap(),
            Some(payload.to_vec())
        );
    }

    #[test]
    fn decode_optional_icon_rejects_garbage() {
        let err = decode_optional_icon(Some("not valid base64 *****")).unwrap_err();
        assert_eq!(err.error_kind(), "invalid_base64");
    }
}
