use std::sync::Arc;

use activity_core::{
    Activity as WireActivity, InsertActivityRequest, InsertActivityResponse, ListActivitiesQuery,
    ListActivitiesResponse,
};
use axum::{
    Json,
    extract::{Query, State},
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use be_asset::CreateAssetInput;
use be_remote_db::PaginationParams;

use crate::analytics;
use crate::auth::AuthUser;
use crate::error::{ActivityResult, ActivityServiceError};
use crate::service::AppState;

const DEFAULT_LIST_LIMIT: u32 = 20;
const DEFAULT_LIST_OFFSET: u32 = 0;

#[tracing::instrument(skip(state, user), fields(limit, offset))]
pub async fn list_activities(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Query(query): Query<ListActivitiesQuery>,
) -> ActivityResult<Json<ListActivitiesResponse>> {
    let user_id = user.user_id()?;

    let limit = query.limit.unwrap_or(DEFAULT_LIST_LIMIT);
    let offset = query.offset.unwrap_or(DEFAULT_LIST_OFFSET);
    tracing::Span::current().record("limit", limit);
    tracing::Span::current().record("offset", offset);

    let activities = state
        .db
        .list_activities()
        .user_id(user_id)
        .params(PaginationParams::new(offset, limit, "DESC"))
        .call()
        .await
        .map_err(|e| {
            analytics::track_activities_list_failed("database_error");
            ActivityServiceError::from(e)
        })?;

    let result_count = activities.len();
    let activities: Vec<WireActivity> = activities.into_iter().map(db_to_wire).collect();

    tracing::debug!("Listed {} activities", result_count);
    analytics::track_activities_listed(limit, offset, result_count);

    Ok(Json(ListActivitiesResponse { activities }))
}

#[tracing::instrument(skip(state, user, body), fields(name = %body.name, process_name = %body.process_name))]
pub async fn insert_activity(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(body): Json<InsertActivityRequest>,
) -> ActivityResult<Json<InsertActivityResponse>> {
    let user_id = user.user_id()?;

    let icon_bytes = decode_optional_icon(body.icon_png_base64.as_deref())
        .inspect_err(|e| analytics::track_activity_insert_failed(e.error_kind()))?;

    let process_name = body.process_name.clone();
    let has_icon = icon_bytes.is_some();
    let has_ended_at = body.ended_at.is_some();

    let activity = state
        .db
        .create_activity()
        .maybe_id(body.id)
        .user_id(user_id)
        .name(body.name)
        .process_name(body.process_name)
        .window_title(body.window_title)
        .started_at(body.started_at)
        .maybe_ended_at(body.ended_at)
        .call()
        .await
        .map_err(|e| {
            analytics::track_activity_insert_failed("database_error");
            ActivityServiceError::from(e)
        })?;

    tracing::info!(
        "Created activity {} at {:?}",
        activity.id,
        activity.created_at
    );

    let icon_asset_id = if let Some(content) = icon_bytes {
        let asset = state
            .asset_service
            .create_asset(
                CreateAssetInput {
                    name: "icon".to_string(),
                    content,
                    mime_type: "image/png".to_string(),
                    metadata: None,
                    activity_id: None,
                },
                user_id,
            )
            .await
            .map_err(|e| {
                analytics::track_activity_insert_failed("asset_error");
                ActivityServiceError::from(e)
            })?;

        Some(asset.id)
    } else {
        None
    };

    let activity = if let Some(asset_id) = icon_asset_id {
        state
            .db
            .update_activity()
            .id(activity.id)
            .user_id(user_id)
            .icon_asset_id(asset_id)
            .call()
            .await
            .map_err(|e| {
                analytics::track_activity_insert_failed("database_error");
                ActivityServiceError::from(e)
            })?
    } else {
        activity
    };

    analytics::track_activity_inserted(has_icon, has_ended_at, &process_name);

    Ok(Json(InsertActivityResponse {
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
