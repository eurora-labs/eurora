use std::sync::Arc;

use activity_core::{
    Activity as WireActivity, ActivitySession as WireActivitySession,
    ActivityWithLatestSession as WireActivityWithLatestSession, DEFAULT_LIST_LIMIT,
    InsertActivitySessionRequest, InsertActivitySessionResponse, ListActivitiesQuery,
    ListActivitiesResponse, ListActivitySessionsResponse, MAX_LIST_LIMIT,
    UpdateActivitySessionRequest, UpdateActivitySessionResponse,
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

    let rows = state
        .db
        .list_activities_with_latest_session()
        .user_id(user_id)
        .params(PaginationParams::new(offset, limit, "DESC"))
        .call()
        .await
        .map_err(|e| {
            let err = ActivityServiceError::from(e);
            analytics::track_activities_list_failed(err.error_kind());
            err
        })?;

    let result_count = rows.len();
    tracing::debug!(result_count, "Listed activities");
    analytics::track_activities_listed(limit, offset, result_count);

    Ok(Json(ListActivitiesResponse {
        activities: rows
            .into_iter()
            .map(|(activity, latest_session)| WireActivityWithLatestSession {
                activity: activity_to_wire(activity),
                latest_session: latest_session.map(session_to_wire),
            })
            .collect(),
    }))
}

#[tracing::instrument(skip_all, fields(user_id, activity_id = %activity_id, limit, offset))]
pub async fn list_activity_sessions(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(activity_id): Path<Uuid>,
    Query(query): Query<ListActivitiesQuery>,
) -> ActivityResult<Json<ListActivitySessionsResponse>> {
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

    let sessions = state
        .db
        .list_sessions_for_activity()
        .user_id(user_id)
        .activity_id(activity_id)
        .params(PaginationParams::new(offset, limit, "DESC"))
        .call()
        .await
        .map_err(ActivityServiceError::from)?;

    Ok(Json(ListActivitySessionsResponse {
        sessions: sessions.into_iter().map(session_to_wire).collect(),
    }))
}

#[tracing::instrument(skip_all, fields(user_id, identity_key, has_icon, has_ended_at))]
pub async fn insert_activity_session(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(body): Json<InsertActivitySessionRequest>,
) -> ActivityResult<Json<InsertActivitySessionResponse>> {
    let user_id = user.user_id()?;

    let icon_bytes =
        decode_optional_icon(body.activity.icon_png_base64.as_deref()).inspect_err(|e| {
            analytics::track_activity_insert_failed(e.error_kind());
        })?;

    let has_icon = icon_bytes.is_some();
    let has_ended_at = body.ended_at.is_some();

    let span = tracing::Span::current();
    span.record("user_id", tracing::field::display(user_id));
    span.record(
        "identity_key",
        tracing::field::display(&body.activity.identity_key),
    );
    span.record("has_icon", has_icon);
    span.record("has_ended_at", has_ended_at);

    // Upload the icon first so the session row is the last write. If the
    // asset upload fails, no activity / session is created; if the
    // session insert fails after a successful upload, we leak an asset
    // blob (cleanable by a sweeper) but never persist a half-built row.
    let icon_asset_id = match icon_bytes {
        Some(content) => Some(
            state
                .asset_service
                .create_asset(
                    CreateAssetInput {
                        name: format!("activity-icon-{}", body.activity.identity_key),
                        content,
                        mime_type: ICON_MIME_TYPE.to_string(),
                        metadata: None,
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

    let (activity, session) = state
        .db
        .insert_activity_session()
        .user_id(user_id)
        .maybe_session_id(body.session_id)
        .identity_key(body.activity.identity_key)
        .display_name(body.activity.display_name)
        .maybe_icon_asset_id(icon_asset_id)
        .process_name(body.process_name)
        .maybe_process_id(body.process_id)
        .maybe_window_title(body.window_title)
        .maybe_url(body.url)
        .started_at(body.started_at)
        .maybe_ended_at(body.ended_at)
        .call()
        .await
        .map_err(|e| {
            let err = ActivityServiceError::from(e);
            analytics::track_activity_insert_failed(err.error_kind());
            err
        })?;

    tracing::info!(
        activity_id = %activity.id,
        session_id = %session.id,
        "Created activity session"
    );
    analytics::track_activity_session_inserted(has_icon, has_ended_at, &activity.identity_key);

    Ok(Json(InsertActivitySessionResponse {
        activity: activity_to_wire(activity),
        session: session_to_wire(session),
    }))
}

#[tracing::instrument(skip_all, fields(user_id, session_id = %session_id, has_ended_at, has_window_title, has_url))]
pub async fn patch_activity_session(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(session_id): Path<Uuid>,
    Json(body): Json<UpdateActivitySessionRequest>,
) -> ActivityResult<Json<UpdateActivitySessionResponse>> {
    let user_id = user.user_id()?;

    let span = tracing::Span::current();
    span.record("user_id", tracing::field::display(user_id));
    span.record("has_ended_at", body.ended_at.is_some());
    span.record("has_window_title", body.window_title.is_some());
    span.record("has_url", body.url.is_some());

    let UpdateActivitySessionRequest {
        window_title,
        url,
        ended_at,
    } = body;

    if window_title.is_none() && url.is_none() && ended_at.is_none() {
        return Err(ActivityServiceError::invalid_argument(
            "PATCH body must set at least one of: window_title, url, ended_at",
        ));
    }

    let set_window_title = window_title.is_some();
    let set_url = url.is_some();
    let set_ended_at = ended_at.is_some();

    let session = state
        .db
        .update_activity_session()
        .session_id(session_id)
        .user_id(user_id)
        .maybe_window_title(window_title)
        .maybe_url(url)
        .maybe_ended_at(ended_at)
        .call()
        .await
        .map_err(|e| {
            let err = ActivityServiceError::from(e);
            analytics::track_activity_session_update_failed(err.error_kind());
            err
        })?;

    tracing::debug!(session_id = %session.id, "Patched activity session");
    analytics::track_activity_session_updated(set_ended_at, set_window_title, set_url);

    Ok(Json(UpdateActivitySessionResponse {
        session: session_to_wire(session),
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

fn activity_to_wire(activity: be_remote_db::Activity) -> WireActivity {
    WireActivity {
        id: activity.id,
        user_id: activity.user_id,
        identity_key: activity.identity_key,
        display_name: activity.display_name,
        icon_asset_id: activity.icon_asset_id,
        last_used_at: activity.last_used_at,
        created_at: activity.created_at,
        updated_at: activity.updated_at,
    }
}

fn session_to_wire(session: be_remote_db::ActivitySession) -> WireActivitySession {
    WireActivitySession {
        id: session.id,
        activity_id: session.activity_id,
        process_name: session.process_name,
        process_id: session.process_id,
        window_title: session.window_title,
        url: session.url,
        started_at: session.started_at,
        ended_at: session.ended_at,
        created_at: session.created_at,
        updated_at: session.updated_at,
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
