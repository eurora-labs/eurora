//! Snapshot variant produced by [`crate::strategies::DefaultStrategy`]
//! when the focused application has no specialised strategy.
//!
//! Carries a single PNG of the focused window, base64-encoded, alongside
//! the process name and window title so the LLM has a textual anchor for
//! the image. The `image_base64` field is intentionally
//! `#[serde(skip_serializing)]` so screenshots never land in the on-disk
//! timeline — they are reconstructed per chat turn from a live capture.

use agent_chain_core::messages::{
    ContentBlock, ContentBlocks, ImageContentBlock, PlainTextContentBlock,
};
use euro_vision::capture::CapturedImage;
use serde::{Deserialize, Serialize};

use crate::types::SnapshotFunctionality;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultSnapshot {
    pub id: String,
    pub process_name: String,
    pub window_title: Option<String>,
    pub width: u32,
    pub height: u32,
    /// Raw base64-encoded PNG. Excluded from serialisation so it never
    /// hits disk; only construct LLM messages from it.
    #[serde(skip_serializing)]
    pub image_base64: String,
    pub created_at: u64,
    pub updated_at: u64,
}

impl DefaultSnapshot {
    pub fn new(process_name: String, window_title: Option<String>, image: CapturedImage) -> Self {
        let now = chrono::Utc::now().timestamp() as u64;
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            process_name,
            window_title,
            width: image.width,
            height: image.height,
            image_base64: image.png_base64,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().timestamp() as u64;
    }
}

impl SnapshotFunctionality for DefaultSnapshot {
    fn construct_messages(&self) -> ContentBlocks {
        let snapshot_json = serde_json::to_string(&self).unwrap_or_default();

        let context = match &self.window_title {
            Some(title) => format!(
                "Screenshot of '{}' from process '{}'",
                title, self.process_name
            ),
            None => format!("Screenshot of process '{}'", self.process_name),
        };

        let text_block = PlainTextContentBlock::builder()
            .context(context)
            .title("default_snapshot.json".to_string())
            .mime_type("application/json".to_string())
            .text(snapshot_json)
            .build();

        let mut blocks: Vec<ContentBlock> = vec![text_block.into()];

        if !self.image_base64.is_empty() {
            match ImageContentBlock::builder()
                .base64(self.image_base64.clone())
                .mime_type("image/png".to_string())
                .build()
            {
                Ok(block) => blocks.push(ContentBlock::Image(block)),
                Err(e) => tracing::warn!("Failed to create screenshot image block: {e}"),
            }
        }

        blocks.into()
    }

    fn get_updated_at(&self) -> u64 {
        self.updated_at
    }

    fn get_created_at(&self) -> u64 {
        self.created_at
    }

    fn get_id(&self) -> &str {
        &self.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_chain_core::messages::ContentBlock;

    fn sample(image_base64: &str) -> DefaultSnapshot {
        DefaultSnapshot::new(
            "code".to_string(),
            Some("main.rs - Eurora".to_string()),
            CapturedImage {
                png_base64: image_base64.to_string(),
                width: 640,
                height: 480,
            },
        )
    }

    #[test]
    fn construct_messages_emits_text_and_image_when_base64_present() {
        let snapshot = sample("aGVsbG8="); // "hello"
        let blocks = snapshot.construct_messages();

        assert_eq!(blocks.len(), 2);
        assert!(matches!(blocks[0], ContentBlock::PlainText(_)));
        match &blocks[1] {
            ContentBlock::Image(img) => {
                assert_eq!(img.mime_type.as_deref(), Some("image/png"));
                assert_eq!(img.base64.as_deref(), Some("aGVsbG8="));
            }
            other => panic!("expected image block, got {other:?}"),
        }
    }

    #[test]
    fn construct_messages_skips_image_when_base64_empty() {
        let snapshot = sample("");
        let blocks = snapshot.construct_messages();

        assert_eq!(blocks.len(), 1);
        assert!(matches!(blocks[0], ContentBlock::PlainText(_)));
    }

    #[test]
    fn serialised_form_omits_image_bytes() {
        let snapshot = sample("aGVsbG8=");
        let json = serde_json::to_string(&snapshot).expect("serialise");
        assert!(
            !json.contains("aGVsbG8="),
            "screenshot bytes leaked into serialised snapshot: {json}"
        );
        assert!(json.contains("\"process_name\":\"code\""));
    }
}
