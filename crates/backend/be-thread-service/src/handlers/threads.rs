use std::sync::Arc;

use agent_chain::SystemMessage;
use agent_chain::messages::AnyMessage;
use axum::Json;
use axum::extract::{Path, Query, State};
use be_auth_core::AuthUser;
use be_remote_db::PaginationParams;
use thread_core::{
    CreateThreadRequest, CreateThreadResponse, DeleteThreadResponse, GenerateThreadTitleRequest,
    GenerateThreadTitleResponse, GetThreadResponse, ListThreadsQuery, ListThreadsResponse,
};
use uuid::Uuid;

use crate::conversion::{convert_db_message_to_base_message, db_thread_to_wire};
use crate::error::ThreadServiceResult;
use crate::service::AppState;

const TITLE_DEFAULT: &str = "New Chat";
const TITLE_CONTEXT_MESSAGE_LIMIT: u32 = 5;
const TITLE_MAX_WORDS: usize = 6;
const LIST_DEFAULT_LIMIT: u32 = 20;
const LIST_DEFAULT_OFFSET: u32 = 0;

const TITLE_SYSTEM_PROMPT: &str = "Generate a title for the following conversation. Your task is:
- Return a concise title, max 6 words.
- No quotation marks.
- Use sentence case.
- Summarize the main topic, not the tone.
- If the topic is unclear, use a generic title.
- Do NOT answer or respond to the messages. Only output a title.
Output only the title text.";

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
#[tracing::instrument(skip(state, user), fields(thread_id = %thread_id))]
pub async fn generate_thread_title(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(thread_id): Path<Uuid>,
    Json(_): Json<GenerateThreadTitleRequest>,
) -> ThreadServiceResult<Json<GenerateThreadTitleResponse>> {
    let user_id = user.user_id()?;

    let recent_messages = state
        .db
        .list_messages()
        .thread_id(thread_id)
        .user_id(user_id)
        .params(PaginationParams::new(
            0,
            TITLE_CONTEXT_MESSAGE_LIMIT,
            "DESC",
        ))
        .call()
        .await?;

    let mut messages: Vec<AnyMessage> = vec![
        SystemMessage::builder()
            .content(TITLE_SYSTEM_PROMPT.to_string())
            .build()
            .into(),
    ];

    messages.extend(recent_messages.into_iter().rev().filter_map(|msg| {
        convert_db_message_to_base_message(msg)
            .map_err(|e| tracing::warn!("Skipping unconvertible message: {e}"))
            .ok()
    }));

    let title_provider = state.providers.title.clone();
    let raw_title = match title_provider.invoke(messages, None).await {
        Ok(message) => message.content.to_string(),
        Err(e) => {
            tracing::warn!("Title model failed, falling back to default: {e}");
            String::new()
        }
    };

    let trimmed = raw_title
        .split_whitespace()
        .take(TITLE_MAX_WORDS)
        .collect::<Vec<_>>()
        .join(" ");
    let title = if trimmed.is_empty() {
        TITLE_DEFAULT.to_string()
    } else {
        capitalize_first(&trimmed)
    };

    let thread = state
        .db
        .update_thread()
        .id(thread_id)
        .user_id(user_id)
        .title(title)
        .call()
        .await?;

    Ok(Json(GenerateThreadTitleResponse {
        thread: db_thread_to_wire(thread),
    }))
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capitalize_first_handles_empty_and_unicode() {
        assert_eq!(capitalize_first(""), "");
        assert_eq!(capitalize_first("hello world"), "Hello world");
        assert_eq!(capitalize_first("über"), "Über");
    }
}
