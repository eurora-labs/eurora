use std::sync::Arc;

use agent_chain::messages::ContentBlock;
use axum::Json;
use axum::extract::{Path, Query, State};
use base64::{Engine as _, engine::general_purpose};
use be_auth_core::AuthUser;
use be_remote_db::PaginationParams;
use thread_core::{
    GetMessagesQuery, GetMessagesResponse, SavePreliminaryContentBlocksRequest,
    SavePreliminaryContentBlocksResponse, SwitchBranchRequest,
};
use uuid::Uuid;

use crate::conversion::{build_branch_tree, build_full_tree};
use crate::error::{ThreadServiceError, ThreadServiceResult};
use crate::service::AppState;

const GET_MESSAGES_DEFAULT_LIMIT: u32 = 100;
const GET_MESSAGES_DEFAULT_OFFSET: u32 = 0;
const SWITCH_BRANCH_FETCH_LIMIT: u32 = 100;
const MAX_PRELIMINARY_BLOCKS: usize = 50;

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

/// Persist large in-line content blocks as assets and return the rewritten
/// blocks (with `url`/`file_id` set, and the in-line payload stripped).
///
/// Currently rewrites:
/// * `text-plain` blocks with `text` set → uploaded as a UTF-8 asset.
/// * `image` blocks with `base64` set → decoded and uploaded as an image
///   asset; mime type defaults to `image/png` if absent.
///
/// Other blocks are passed through unchanged.
#[tracing::instrument(skip(state, user, body), fields(thread_id = %thread_id, block_count = body.content_blocks.len()))]
pub async fn save_preliminary_content_blocks(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(thread_id): Path<Uuid>,
    Json(body): Json<SavePreliminaryContentBlocksRequest>,
) -> ThreadServiceResult<Json<SavePreliminaryContentBlocksResponse>> {
    let user_id = user.user_id()?;

    if body.content_blocks.len() > MAX_PRELIMINARY_BLOCKS {
        return Err(ThreadServiceError::invalid_argument(format!(
            "Too many content blocks (max {MAX_PRELIMINARY_BLOCKS})"
        )));
    }

    let blocks = body.content_blocks;

    state
        .db
        .get_thread()
        .id(thread_id)
        .user_id(user_id)
        .call()
        .await?;

    let mut result_blocks: Vec<ContentBlock> = Vec::with_capacity(blocks.len());
    for block in blocks {
        match block {
            ContentBlock::PlainText(mut plain) => {
                let Some(text) = plain.text.take() else {
                    result_blocks.push(ContentBlock::PlainText(plain));
                    continue;
                };
                let name = plain
                    .title
                    .clone()
                    .unwrap_or_else(|| "content.json".to_string());

                let (asset_id, storage_uri) = upload_block_content(
                    &state,
                    &name,
                    text.as_bytes(),
                    &plain.mime_type,
                    &plain.extras,
                    user_id,
                )
                .await?;

                plain.file_id = Some(asset_id);
                plain.url = Some(storage_uri);
                result_blocks.push(ContentBlock::PlainText(plain));
            }
            ContentBlock::Image(mut image) => {
                let Some(b64) = image.base64.take() else {
                    result_blocks.push(ContentBlock::Image(image));
                    continue;
                };
                let content = general_purpose::STANDARD
                    .decode(&b64)
                    .map_err(|e| ThreadServiceError::invalid_base64("image.base64", e))?;
                let mime = image
                    .mime_type
                    .clone()
                    .unwrap_or_else(|| "image/png".to_string());
                let name = format!(
                    "image.{}",
                    be_storage::StorageService::extension_from_mime(&mime)
                );

                let (asset_id, storage_uri) =
                    upload_block_content(&state, &name, &content, &mime, &image.extras, user_id)
                        .await?;

                image.file_id = Some(asset_id);
                image.url = Some(storage_uri);
                result_blocks.push(ContentBlock::Image(image));
            }
            other => result_blocks.push(other),
        }
    }

    Ok(Json(SavePreliminaryContentBlocksResponse {
        content_blocks: result_blocks,
    }))
}

async fn upload_block_content(
    state: &AppState,
    name: &str,
    content: &[u8],
    mime_type: &str,
    extras: &Option<std::collections::HashMap<String, serde_json::Value>>,
    user_id: Uuid,
) -> ThreadServiceResult<(String, String)> {
    let metadata = extras.as_ref().map(|map| {
        serde_json::Value::Object(map.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
    });

    let asset = state
        .asset_service
        .create_asset(
            be_asset::CreateAssetInput {
                name: name.to_string(),
                content: content.to_vec(),
                mime_type: mime_type.to_string(),
                metadata,
                activity_id: None,
            },
            user_id,
        )
        .await?;

    Ok((asset.id.to_string(), asset.storage_uri))
}
