//! Rewrite in-line content blocks into asset references.
//!
//! Some content blocks arrive from the client with their payload inlined
//! (a `text` field on `PlainText`, a `base64` field on `Image`). Persisting
//! those payloads in the message row would bloat the database and the
//! follow-up LLM context payloads. Instead we upload them to the asset
//! service up-front and replace the inline payload with a `file_id` + `url`
//! pair pointing at the asset row.
//!
//! This used to be exposed as a separate `POST /threads/{id}/preliminary-blocks`
//! endpoint that the desktop client called before opening the chat WebSocket.
//! The chat handler now calls this helper directly, so the round trip is
//! gone and clients can send raw blocks straight through the WebSocket.

use base64::{Engine as _, engine::general_purpose};
use std::collections::HashMap;

use agent_chain::messages::ContentBlock;
use uuid::Uuid;

use crate::error::{ThreadServiceError, ThreadServiceResult};
use crate::service::AppState;

/// Maximum number of blocks accepted in a single rewrite pass.
///
/// The previous REST endpoint enforced this; we keep it here so the chat
/// handler is also bounded — a malicious or buggy client cannot stuff a
/// turn with thousands of inline payloads.
pub const MAX_PRELIMINARY_BLOCKS: usize = 50;

/// Replace inline `PlainText.text` and `Image.base64` payloads in `blocks`
/// with asset references uploaded under `user_id`. Other block variants
/// pass through unchanged.
pub async fn rewrite_preliminary_blocks(
    state: &AppState,
    user_id: Uuid,
    blocks: Vec<ContentBlock>,
) -> ThreadServiceResult<Vec<ContentBlock>> {
    if blocks.len() > MAX_PRELIMINARY_BLOCKS {
        return Err(ThreadServiceError::invalid_argument(format!(
            "Too many content blocks (max {MAX_PRELIMINARY_BLOCKS})"
        )));
    }

    let mut rewritten: Vec<ContentBlock> = Vec::with_capacity(blocks.len());
    for block in blocks {
        match block {
            ContentBlock::PlainText(mut plain) => {
                let Some(text) = plain.text.take() else {
                    rewritten.push(ContentBlock::PlainText(plain));
                    continue;
                };
                let name = plain
                    .title
                    .clone()
                    .unwrap_or_else(|| "content.json".to_string());

                let (asset_id, storage_uri) = upload_block_content(
                    state,
                    &name,
                    text.as_bytes(),
                    &plain.mime_type,
                    &plain.extras,
                    user_id,
                )
                .await?;

                plain.file_id = Some(asset_id);
                plain.url = Some(storage_uri);
                rewritten.push(ContentBlock::PlainText(plain));
            }
            ContentBlock::Image(mut image) => {
                let Some(b64) = image.base64.take() else {
                    rewritten.push(ContentBlock::Image(image));
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
                    upload_block_content(state, &name, &content, &mime, &image.extras, user_id)
                        .await?;

                image.file_id = Some(asset_id);
                image.url = Some(storage_uri);
                rewritten.push(ContentBlock::Image(image));
            }
            other => rewritten.push(other),
        }
    }

    Ok(rewritten)
}

async fn upload_block_content(
    state: &AppState,
    name: &str,
    content: &[u8],
    mime_type: &str,
    extras: &Option<HashMap<String, serde_json::Value>>,
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
            },
            user_id,
        )
        .await?;

    Ok((asset.id.to_string(), asset.storage_uri))
}
