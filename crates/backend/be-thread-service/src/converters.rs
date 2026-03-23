use std::collections::HashMap;

use agent_chain::messages::content::{ContentBlock, ContentBlocks, ImageContentBlock, TextContentBlock};
use agent_chain::{AIMessage, AnyMessage, HumanMessage, SystemMessage, ToolCall, ToolMessage};
use be_asset::AssetService;
use be_remote_db::{Asset, DatabaseManager, Message, MessageType};
use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{ThreadServiceError, ThreadServiceResult};

/// Reference to an image asset stored in additional_kwargs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageAssetRef {
    pub asset_id: Uuid,
    pub mime_type: String,
}

/// Extracted content from a HumanMessage for storage
#[derive(Debug)]
pub struct ExtractedMessageContent {
    pub text: String,
    pub images: Vec<ExtractedImage>,
}

/// An image extracted from a message content block
#[derive(Debug)]
pub struct ExtractedImage {
    pub base64_data: String,
    pub mime_type: String,
}

/// Extract text and images from a HumanMessage's content blocks
pub fn extract_message_content(message: &HumanMessage) -> ExtractedMessageContent {
    let mut text = String::new();
    let mut images = Vec::new();

    for block in message.content.iter() {
        match block {
            ContentBlock::Text(t) => {
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(&t.text);
            }
            ContentBlock::Image(img) => {
                // Extract image data (prefer base64, fall back to URL)
                if let Some(base64) = &img.base64 {
                    let mime = img.mime_type.clone().unwrap_or_else(|| "image/png".to_string());
                    images.push(ExtractedImage {
                        base64_data: base64.clone(),
                        mime_type: mime,
                    });
                } else if let Some(url) = &img.url {
                    // If it's a data URL, extract the base64 part
                    if url.starts_with("data:") {
                        if let Some(base64_part) = url.split(',').nth(1) {
                            let mime = img.mime_type.clone().unwrap_or_else(|| "image/png".to_string());
                            images.push(ExtractedImage {
                                base64_data: base64_part.to_string(),
                                mime_type: mime,
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    ExtractedMessageContent { text, images }
}

/// Create assets from extracted images and return references
pub async fn create_image_assets(
    images: &[ExtractedImage],
    asset_service: &AssetService,
    user_id: Uuid,
) -> ThreadServiceResult<Vec<ImageAssetRef>> {
    let mut refs = Vec::new();
    
    for image in images {
        // Decode base64 to bytes
        let bytes = general_purpose::STANDARD
            .decode(&image.base64_data)
            .map_err(|e| ThreadServiceError::Internal(format!("Failed to decode base64 image: {}", e)))?;
        
        // Create asset
        let request = proto_gen::asset::CreateAssetRequest {
            name: format!("image_{}", Uuid::now_v7()),
            content: bytes,
            mime_type: image.mime_type.clone(),
            metadata: None,
            activity_id: None,
        };
        
        let response = asset_service
            .create_asset(request, user_id)
            .await
            .map_err(|e| ThreadServiceError::Internal(format!("Failed to create image asset: {}", e)))?;
        
        if let Some(asset) = response.asset {
            refs.push(ImageAssetRef {
                asset_id: Uuid::parse_str(&asset.id)
                    .map_err(|e| ThreadServiceError::Internal(format!("Invalid asset ID: {}", e)))?,
                mime_type: asset.mime_type,
            });
        }
    }
    
    Ok(refs)
}

/// Extract image asset references from additional_kwargs
fn extract_image_asset_refs(additional_kwargs: &serde_json::Value) -> Vec<ImageAssetRef> {
    additional_kwargs
        .get("image_assets")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default()
}

/// Convert a database message to a base message (sync version, no image support)
pub fn convert_db_message_to_base_message(db_message: Message) -> ThreadServiceResult<AnyMessage> {
    let id = db_message.id.to_string();

    match db_message.message_type {
        MessageType::Human => {
            let message = HumanMessage::builder()
                .id(id)
                .content(db_message.content)
                .build();
            Ok(AnyMessage::HumanMessage(message))
        }
        MessageType::System => {
            let message = SystemMessage::builder()
                .id(id)
                .content(db_message.content)
                .build();
            Ok(AnyMessage::SystemMessage(message))
        }
        MessageType::Ai => {
            let tool_calls = parse_tool_calls(&db_message.tool_calls)?;
            let message = AIMessage::builder()
                .id(id)
                .content(db_message.content)
                .tool_calls(tool_calls)
                .build();
            Ok(AnyMessage::AIMessage(message))
        }
        MessageType::Tool => {
            let tool_call_id = db_message.tool_call_id.ok_or_else(|| {
                ThreadServiceError::Internal("Tool message missing tool_call_id".to_string())
            })?;
            let message = ToolMessage::builder()
                .id(id)
                .content(db_message.content)
                .tool_call_id(tool_call_id)
                .build();
            Ok(AnyMessage::ToolMessage(message))
        }
    }
}

/// Convert a database message to a base message with image reconstruction support
pub async fn convert_db_message_to_base_message_async(
    db_message: Message,
    db: &DatabaseManager,
    asset_cache: &mut HashMap<Uuid, Asset>,
) -> ThreadServiceResult<AnyMessage> {
    let id = db_message.id.to_string();
    let image_refs = extract_image_asset_refs(&db_message.additional_kwargs);

    match db_message.message_type {
        MessageType::Human => {
            let content_blocks = build_content_blocks(&db_message.content, &image_refs, db, asset_cache).await;
            let message = HumanMessage::builder()
                .id(id)
                .content(content_blocks)
                .build();
            Ok(AnyMessage::HumanMessage(message))
        }
        MessageType::System => {
            let message = SystemMessage::builder()
                .id(id)
                .content(db_message.content)
                .build();
            Ok(AnyMessage::SystemMessage(message))
        }
        MessageType::Ai => {
            let tool_calls = parse_tool_calls(&db_message.tool_calls)?;
            let message = AIMessage::builder()
                .id(id)
                .content(db_message.content)
                .tool_calls(tool_calls)
                .build();
            Ok(AnyMessage::AIMessage(message))
        }
        MessageType::Tool => {
            let tool_call_id = db_message.tool_call_id.ok_or_else(|| {
                ThreadServiceError::Internal("Tool message missing tool_call_id".to_string())
            })?;
            let message = ToolMessage::builder()
                .id(id)
                .content(db_message.content)
                .tool_call_id(tool_call_id)
                .build();
            Ok(AnyMessage::ToolMessage(message))
        }
    }
}

/// Build content blocks from text and image asset references
async fn build_content_blocks(
    text: &str,
    image_refs: &[ImageAssetRef],
    db: &DatabaseManager,
    asset_cache: &mut HashMap<Uuid, Asset>,
) -> ContentBlocks {
    let mut blocks: Vec<ContentBlock> = vec![
        ContentBlock::Text(TextContentBlock::new(text))
    ];

    for image_ref in image_refs {
        match get_or_fetch_asset(image_ref.asset_id, db, asset_cache).await {
            Ok(asset) => {
                blocks.push(ContentBlock::Image(ImageContentBlock {
                    url: Some(asset.storage_uri.clone()),
                    mime_type: Some(image_ref.mime_type.clone()),
                    ..Default::default()
                }));
            }
            Err(e) => {
                tracing::warn!("Failed to fetch image asset {}: {}", image_ref.asset_id, e);
            }
        }
    }

    ContentBlocks::from(blocks)
}

/// Get asset from cache or fetch from database
async fn get_or_fetch_asset(
    asset_id: Uuid,
    db: &DatabaseManager,
    cache: &mut HashMap<Uuid, Asset>,
) -> ThreadServiceResult<Asset> {
    if let Some(asset) = cache.get(&asset_id) {
        return Ok(asset.clone());
    }

    let asset = db.get_asset()
        .id(asset_id)
        .call()
        .await
        .map_err(|e| ThreadServiceError::Internal(format!("Asset not found: {}", e)))?;

    cache.insert(asset_id, asset.clone());
    Ok(asset)
}

fn parse_tool_calls(tool_calls: &Option<serde_json::Value>) -> ThreadServiceResult<Vec<ToolCall>> {
    match tool_calls {
        None => Ok(Vec::new()),
        Some(serde_json::Value::Null) => Ok(Vec::new()),
        Some(value) => serde_json::from_value(value.clone()).map_err(|e| {
            ThreadServiceError::Internal(format!("Failed to parse tool calls: {}", e))
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_image_asset_refs_valid() {
        let kwargs = serde_json::json!({
            "image_assets": [
                {"asset_id": "00000000-0000-0000-0000-000000000001", "mime_type": "image/png"}
            ]
        });
        let refs = extract_image_asset_refs(&kwargs);
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].mime_type, "image/png");
    }

    #[test]
    fn test_extract_image_asset_refs_missing() {
        let kwargs = serde_json::json!({});
        let refs = extract_image_asset_refs(&kwargs);
        assert!(refs.is_empty());
    }

    #[test]
    fn test_extract_image_asset_refs_invalid() {
        let kwargs = serde_json::json!({
            "image_assets": "not an array"
        });
        let refs = extract_image_asset_refs(&kwargs);
        assert!(refs.is_empty());
    }

    #[test]
    fn test_convert_db_message_to_base_message_human() {
        let msg = Message {
            id: Uuid::nil(),
            thread_id: Uuid::nil(),
            user_id: Uuid::nil(),
            parent_message_id: None,
            message_type: MessageType::Human,
            content: "Hello".to_string(),
            tool_call_id: None,
            tool_calls: None,
            additional_kwargs: serde_json::json!({}),
            reasoning_blocks: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let result = convert_db_message_to_base_message(msg).unwrap();
        match result {
            AnyMessage::HumanMessage(h) => {
                assert_eq!(h.content.as_text(), "Hello");
            }
            _ => panic!("Expected HumanMessage"),
        }
    }

    #[test]
    fn test_convert_db_message_to_base_message_with_image_refs() {
        let msg = Message {
            id: Uuid::nil(),
            thread_id: Uuid::nil(),
            user_id: Uuid::nil(),
            parent_message_id: None,
            message_type: MessageType::Human,
            content: "What is this?".to_string(),
            tool_call_id: None,
            tool_calls: None,
            additional_kwargs: serde_json::json!({
                "image_assets": [
                    {"asset_id": "00000000-0000-0000-0000-000000000001", "mime_type": "image/png"}
                ]
            }),
            reasoning_blocks: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        // Sync version should still work (ignores images)
        let result = convert_db_message_to_base_message(msg).unwrap();
        match result {
            AnyMessage::HumanMessage(h) => {
                assert_eq!(h.content.as_text(), "What is this?");
            }
            _ => panic!("Expected HumanMessage"),
        }
    }
}
