use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, Query, State};
use be_auth_core::AuthUser;
use be_remote_db::PaginationParams;
use thread_core::{GetMessagesQuery, GetMessagesResponse, SwitchBranchRequest};
use uuid::Uuid;

use crate::conversion::{build_branch_tree, build_full_tree};
use crate::error::{ThreadServiceError, ThreadServiceResult};
use crate::service::AppState;

const GET_MESSAGES_DEFAULT_LIMIT: u32 = 100;
const GET_MESSAGES_DEFAULT_OFFSET: u32 = 0;
const SWITCH_BRANCH_FETCH_LIMIT: u32 = 100;

#[tracing::instrument(skip(state, user), fields(thread_id = %thread_id))]
pub async fn get_messages(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(thread_id): Path<Uuid>,
    Query(query): Query<GetMessagesQuery>,
) -> ThreadServiceResult<Json<GetMessagesResponse>> {
    let user_id = user.user_id()?;
    let limit = query.limit.unwrap_or(GET_MESSAGES_DEFAULT_LIMIT);
    let offset = query.offset.unwrap_or(GET_MESSAGES_DEFAULT_OFFSET);

    let messages = if query.all_variants {
        let rows = state
            .db
            .list_all_thread_messages(thread_id, user_id, limit as i64, offset as i64)
            .await?;
        build_full_tree(rows)?
    } else {
        let rows = state
            .db
            .list_branch_with_siblings()
            .thread_id(thread_id)
            .user_id(user_id)
            .params(PaginationParams::new(offset, limit, "ASC"))
            .call()
            .await?;
        build_branch_tree(rows)?
    };

    Ok(Json(GetMessagesResponse { messages }))
}

#[tracing::instrument(skip(state, user, body), fields(thread_id = %thread_id, message_id = %body.message_id, direction = body.direction))]
pub async fn switch_branch(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(thread_id): Path<Uuid>,
    Json(body): Json<SwitchBranchRequest>,
) -> ThreadServiceResult<Json<GetMessagesResponse>> {
    let user_id = user.user_id()?;

    let target_id = match body.direction {
        0 => body.message_id,
        -1 | 1 => state
            .db
            .get_adjacent_sibling()
            .thread_id(thread_id)
            .user_id(user_id)
            .message_id(body.message_id)
            .direction(body.direction)
            .call()
            .await?
            .ok_or_else(|| ThreadServiceError::not_found("No adjacent sibling found"))?,
        _ => {
            return Err(ThreadServiceError::invalid_argument(
                "direction must be -1, 0, or 1",
            ));
        }
    };

    let new_leaf = state
        .db
        .find_deepest_leaf(thread_id, user_id, target_id)
        .await?;

    state
        .db
        .set_active_leaf()
        .id(thread_id)
        .user_id(user_id)
        .active_leaf_id(new_leaf)
        .call()
        .await?;

    let rows = state
        .db
        .list_branch_with_siblings()
        .thread_id(thread_id)
        .user_id(user_id)
        .params(PaginationParams::new(0, SWITCH_BRANCH_FETCH_LIMIT, "ASC"))
        .call()
        .await?;

    Ok(Json(GetMessagesResponse {
        messages: build_branch_tree(rows)?,
    }))
}
