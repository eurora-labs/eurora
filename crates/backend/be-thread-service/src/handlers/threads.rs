use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, Query, State};
use be_auth_core::AuthUser;
use be_remote_db::PaginationParams;
use thread_core::{
    CreateThreadRequest, CreateThreadResponse, DeleteThreadResponse, GenerateThreadTitleRequest,
    GenerateThreadTitleResponse, GetThreadResponse, ListThreadsQuery, ListThreadsResponse,
};
use uuid::Uuid;

use crate::conversion::db_thread_to_wire;
use crate::error::ThreadServiceResult;
use crate::service::AppState;
use crate::title::{TITLE_DEFAULT, auto_generate_title_if_needed};

const LIST_DEFAULT_LIMIT: u32 = 20;
const LIST_DEFAULT_OFFSET: u32 = 0;

#[tracing::instrument(skip(state, user, body))]
pub async fn create_thread(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(body): Json<CreateThreadRequest>,
) -> ThreadServiceResult<Json<CreateThreadResponse>> {
    let user_id = user.user_id()?;
    let title = body
        .title
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| TITLE_DEFAULT.to_string());

    let thread = state
        .db
        .create_thread()
        .user_id(user_id)
        .title(title)
        .call()
        .await?;

    tracing::info!("Created thread {}", thread.id);

    Ok(Json(CreateThreadResponse {
        thread: db_thread_to_wire(thread),
    }))
}

#[tracing::instrument(skip(state, user))]
pub async fn list_threads(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Query(query): Query<ListThreadsQuery>,
) -> ThreadServiceResult<Json<ListThreadsResponse>> {
    let user_id = user.user_id()?;
    let limit = query.limit.unwrap_or(LIST_DEFAULT_LIMIT);
    let offset = query.offset.unwrap_or(LIST_DEFAULT_OFFSET);

    let threads = state
        .db
        .list_threads()
        .user_id(user_id)
        .params(PaginationParams::new(offset, limit, "DESC"))
        .call()
        .await?;

    Ok(Json(ListThreadsResponse {
        threads: threads.into_iter().map(db_thread_to_wire).collect(),
    }))
}

/// List threads linked (via `activity_threads`) to a single activity.
///
/// Powers the desktop sidebar's per-app filter: as the user cycles through
/// the timeline rail, the frontend fetches the threads associated with the
/// currently-active activity and shows them as the filtered list.
#[tracing::instrument(skip(state, user), fields(activity_id = %activity_id))]
pub async fn list_threads_for_activity(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(activity_id): Path<Uuid>,
    Query(query): Query<ListThreadsQuery>,
) -> ThreadServiceResult<Json<ListThreadsResponse>> {
    let user_id = user.user_id()?;
    let limit = query.limit.unwrap_or(LIST_DEFAULT_LIMIT);
    let offset = query.offset.unwrap_or(LIST_DEFAULT_OFFSET);

    let threads = state
        .db
        .list_threads_for_activity()
        .user_id(user_id)
        .activity_id(activity_id)
        .params(PaginationParams::new(offset, limit, "DESC"))
        .call()
        .await?;

    Ok(Json(ListThreadsResponse {
        threads: threads.into_iter().map(db_thread_to_wire).collect(),
    }))
}

#[tracing::instrument(skip(state, user), fields(thread_id = %thread_id))]
pub async fn get_thread(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(thread_id): Path<Uuid>,
) -> ThreadServiceResult<Json<GetThreadResponse>> {
    let user_id = user.user_id()?;

    let thread = state
        .db
        .get_thread()
        .id(thread_id)
        .user_id(user_id)
        .call()
        .await?;

    Ok(Json(GetThreadResponse {
        thread: db_thread_to_wire(thread),
    }))
}

#[tracing::instrument(skip(state, user), fields(thread_id = %thread_id))]
pub async fn delete_thread(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(thread_id): Path<Uuid>,
) -> ThreadServiceResult<Json<DeleteThreadResponse>> {
    let user_id = user.user_id()?;

    state
        .db
        .delete_thread()
        .id(thread_id)
        .user_id(user_id)
        .call()
        .await?;

    tracing::info!("Deleted thread {}", thread_id);

    Ok(Json(DeleteThreadResponse {}))
}

/// Token-gated. The `be-authz` middleware checks the user's monthly token
/// limit before this handler runs; on exhaustion it short-circuits with a
/// 429 and this code never executes.
///
/// Manual rename / regenerate path — the agent loop already auto-titles a
/// thread on the first turn that settles, so most threads never hit this
/// endpoint. It remains useful for "regenerate title" UX and for clients
/// that lost the wire frame. Idempotency is enforced by
/// [`auto_generate_title_if_needed`]: a thread that already has a
/// user-meaningful title is returned untouched.
#[tracing::instrument(skip(state, user), fields(thread_id = %thread_id))]
pub async fn generate_thread_title(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(thread_id): Path<Uuid>,
    Json(_): Json<GenerateThreadTitleRequest>,
) -> ThreadServiceResult<Json<GenerateThreadTitleResponse>> {
    let user_id = user.user_id()?;

    // The helper writes the row on success; we always re-read so the
    // response carries the canonical post-update state (and we don't have
    // to fork the helper's "Some(title)" return into a half-Thread).
    auto_generate_title_if_needed(
        &state.db,
        state.providers.title.as_ref(),
        thread_id,
        user_id,
    )
    .await?;

    let thread = state
        .db
        .get_thread()
        .id(thread_id)
        .user_id(user_id)
        .call()
        .await?;

    Ok(Json(GenerateThreadTitleResponse {
        thread: db_thread_to_wire(thread),
    }))
}
