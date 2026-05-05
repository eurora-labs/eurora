use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use thread_core::{
    SearchMessageResult, SearchMessagesQuery, SearchMessagesResponse, SearchThreadResult,
    SearchThreadsQuery, SearchThreadsResponse,
};

use be_auth_core::AuthUser;

use crate::error::ThreadServiceResult;
use crate::service::AppState;

const SEARCH_DEFAULT_LIMIT: u32 = 20;
const SEARCH_DEFAULT_OFFSET: u32 = 0;
const MIN_QUERY_LENGTH: usize = 2;

#[tracing::instrument(skip(state, user))]
pub async fn search_threads(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Query(query): Query<SearchThreadsQuery>,
) -> ThreadServiceResult<Json<SearchThreadsResponse>> {
    let user_id = user.user_id()?;

    if query.q.trim().len() < MIN_QUERY_LENGTH {
        return Ok(Json(SearchThreadsResponse { results: vec![] }));
    }

    let limit = query.limit.unwrap_or(SEARCH_DEFAULT_LIMIT);
    let offset = query.offset.unwrap_or(SEARCH_DEFAULT_OFFSET);

    let results = state
        .db
        .search_threads(user_id, &query.q, limit as i64, offset as i64)
        .await?;

    let results = results
        .into_iter()
        .map(|r| SearchThreadResult {
            id: r.id,
            title: r.title.unwrap_or_default(),
            rank: r.rank,
            updated_at: r.updated_at,
        })
        .collect();

    Ok(Json(SearchThreadsResponse { results }))
}

#[tracing::instrument(skip(state, user))]
pub async fn search_messages(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Query(query): Query<SearchMessagesQuery>,
) -> ThreadServiceResult<Json<SearchMessagesResponse>> {
    let user_id = user.user_id()?;

    if query.q.trim().len() < MIN_QUERY_LENGTH {
        return Ok(Json(SearchMessagesResponse { results: vec![] }));
    }

    let limit = query.limit.unwrap_or(SEARCH_DEFAULT_LIMIT);
    let offset = query.offset.unwrap_or(SEARCH_DEFAULT_OFFSET);

    let results = state
        .db
        .search_messages(user_id, &query.q, limit as i64, offset as i64)
        .await?;

    let results = results
        .into_iter()
        .map(|r| SearchMessageResult {
            id: r.id,
            thread_id: r.thread_id,
            message_type: r.message_type.to_string(),
            rank: r.rank,
            created_at: r.created_at,
            snippet: r.snippet,
        })
        .collect();

    Ok(Json(SearchMessagesResponse { results }))
}
