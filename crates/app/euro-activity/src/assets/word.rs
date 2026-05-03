//! Activity-pipeline trait implementations for [`WordAsset`].
//!
//! The struct itself lives in `euro-office` next to the wire-level
//! `WordDocumentAsset`. The trait impls live here because the traits
//! (`AssetFunctionality`, `SaveableAsset`) are owned by `euro-activity`
//! — keeping them close to the trait definitions and away from
//! `euro-office`'s codegen surface.

use agent_chain_core::messages::{ContentBlocks, PlainTextContentBlock};
use async_trait::async_trait;
use euro_office::WordAsset;
use std::collections::HashMap;

use crate::{ActivityResult, storage::SaveableAsset, types::AssetFunctionality};

const WORD_EXTENSION_ID: &str = "0d3a9d8b-5f5e-4a39-9f3a-1f6f1d5d7c2b";

impl AssetFunctionality for WordAsset {
    fn get_id(&self) -> &str {
        &self.id
    }

    fn get_name(&self) -> &str {
        &self.document_name
    }

    fn get_icon(&self) -> Option<&str> {
        Some("word")
    }

    fn construct_messages(&self) -> ContentBlocks {
        let asset_json = serde_json::to_string(self).unwrap_or_default();

        let extras =
            HashMap::from([("asset_id".to_string(), serde_json::json!(WORD_EXTENSION_ID))]);

        let block = PlainTextContentBlock::builder()
            .context(format!(
                "Body of the Microsoft Word document titled: '{}'",
                self.document_name
            ))
            .title(format!("{}.json", self.document_name))
            .mime_type("application/json".to_string())
            .text(asset_json)
            .extras(extras)
            .build();

        vec![block.into()].into()
    }
}

#[async_trait]
impl SaveableAsset for WordAsset {
    fn get_asset_type(&self) -> &'static str {
        "WordAsset"
    }

    async fn serialize_content(&self) -> ActivityResult<Vec<u8>> {
        let bytes = serde_json::to_vec(self)?;
        Ok(bytes)
    }

    fn get_unique_id(&self) -> String {
        self.id.clone()
    }

    fn get_display_name(&self) -> String {
        self.document_name.clone()
    }
}

#[cfg(test)]
mod tests {
    use agent_chain_core::messages::ContentBlock;
    use euro_office::{WordAsset, WordDocumentAsset};

    use crate::types::AssetFunctionality;

    #[test]
    fn from_wire_assigns_id_and_preserves_fields() {
        let wire = WordDocumentAsset {
            document_name: "Doc.docx".into(),
            text: "body".into(),
        };
        let asset = WordAsset::from(wire);
        assert!(!asset.id.is_empty());
        assert_eq!(asset.document_name, "Doc.docx");
        assert_eq!(asset.text, "body");
    }

    #[test]
    fn construct_messages_emits_one_json_block() {
        let asset = WordAsset::new("test-id".into(), "Test Doc".into(), "Some content.".into());

        let blocks = AssetFunctionality::construct_messages(&asset);
        assert_eq!(blocks.len(), 1);
        assert!(matches!(blocks[0], ContentBlock::PlainText(_)));
    }

    #[test]
    fn metadata_accessors_use_document_name() {
        let asset = WordAsset::new("test-id".into(), "Notes.docx".into(), "body".into());
        assert_eq!(asset.get_id(), "test-id");
        assert_eq!(asset.get_name(), "Notes.docx");
        assert_eq!(asset.get_icon(), Some("word"));
    }
}
