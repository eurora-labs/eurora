//! Activity-pipeline trait implementations for [`PdfAsset`].
//!
//! The struct itself lives in `euro-pdf` next to the parser; the trait
//! impls live here for the same reason as Word: the traits
//! ([`AssetFunctionality`], [`SaveableAsset`]) are owned by `euro-activity`
//! and we don't want `euro-pdf` to pull in `agent-chain-core` and the rest
//! of the activity stack.

use std::collections::HashMap;

use agent_chain_core::messages::{ContentBlocks, PlainTextContentBlock};
use async_trait::async_trait;
use euro_pdf::PdfAsset;
use serde_json::json;

use crate::{ActivityResult, storage::SaveableAsset, types::AssetFunctionality};

/// MIME type used for the rendered PDF block.
///
/// pdf-inspector emits CommonMark Markdown, so `text/markdown` is the
/// honest declaration even though many LLM backends collapse it to plain
/// text on ingest.
const PDF_MARKDOWN_MIME: &str = "text/markdown";

/// Placeholder body used when pdf-inspector cannot extract any text from
/// the document (scanned PDFs, image-only pages, encoding failures). The
/// asset still flows through the pipeline so the user-visible chip
/// appears, but the LLM gets a precise signal rather than empty content.
const NO_EXTRACTABLE_TEXT: &str =
    "[This PDF is scanned or image-based and contains no extractable text.]";

impl AssetFunctionality for PdfAsset {
    fn get_id(&self) -> &str {
        &self.id
    }

    fn get_name(&self) -> &str {
        &self.document_name
    }

    fn get_icon(&self) -> Option<&str> {
        Some("pdf")
    }

    fn construct_messages(&self) -> ContentBlocks {
        let text = self
            .markdown
            .as_deref()
            .filter(|m| !m.trim().is_empty())
            .map_or_else(|| NO_EXTRACTABLE_TEXT.to_owned(), ToOwned::to_owned);

        let extras = HashMap::from([
            ("file_path".to_owned(), json!(self.path)),
            ("pdf_type".to_owned(), json!(self.pdf_type)),
            ("page_count".to_owned(), json!(self.page_count)),
        ]);

        let block = PlainTextContentBlock::builder()
            .context(format!(
                "Contents of the PDF document titled: '{}'",
                self.document_name
            ))
            .title(format!("{}.md", self.document_name))
            .mime_type(PDF_MARKDOWN_MIME.to_owned())
            .text(text)
            .extras(extras)
            .build();

        vec![block.into()].into()
    }
}

#[async_trait]
impl SaveableAsset for PdfAsset {
    fn get_asset_type(&self) -> &'static str {
        "PdfAsset"
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
    use euro_pdf::{PdfAsset, PdfTypeKind};

    use crate::storage::SaveableAsset;
    use crate::types::AssetFunctionality;

    fn asset_with(markdown: Option<&str>, pdf_type: PdfTypeKind) -> PdfAsset {
        PdfAsset::builder()
            .id("pdf-test-id")
            .path("/tmp/Lecture Notes.pdf")
            .document_name("Lecture Notes")
            .pdf_type(pdf_type)
            .maybe_markdown(markdown.map(ToOwned::to_owned))
            .page_count(7)
            .build()
    }

    #[test]
    fn metadata_accessors_report_document_name_and_pdf_icon() {
        let asset = asset_with(Some("# Body"), PdfTypeKind::TextBased);
        assert_eq!(asset.get_id(), "pdf-test-id");
        assert_eq!(asset.get_name(), "Lecture Notes");
        assert_eq!(asset.get_icon(), Some("pdf"));
    }

    #[test]
    fn construct_messages_emits_one_markdown_block_with_extras() {
        let asset = asset_with(Some("# Heading\n\nBody."), PdfTypeKind::TextBased);
        let blocks = asset.construct_messages();

        assert_eq!(blocks.len(), 1);
        let ContentBlock::PlainText(block) = &blocks[0] else {
            panic!("expected a PlainText block, got {:?}", blocks[0]);
        };
        assert_eq!(block.mime_type, "text/markdown");
        assert_eq!(block.title.as_deref(), Some("Lecture Notes.md"));
        assert!(
            block
                .text
                .as_deref()
                .is_some_and(|t| t.contains("# Heading"))
        );

        let extras = block.extras.as_ref().expect("extras populated");
        assert_eq!(
            extras.get("file_path").and_then(|v| v.as_str()),
            Some("/tmp/Lecture Notes.pdf"),
        );
        assert_eq!(
            extras.get("pdf_type").and_then(|v| v.as_str()),
            Some("text_based"),
        );
        assert_eq!(
            extras.get("page_count").and_then(serde_json::Value::as_u64),
            Some(7),
        );
    }

    #[test]
    fn construct_messages_uses_placeholder_for_scanned_pdfs() {
        let asset = asset_with(None, PdfTypeKind::Scanned);
        let blocks = asset.construct_messages();
        let ContentBlock::PlainText(block) = &blocks[0] else {
            panic!("expected a PlainText block");
        };
        assert!(
            block
                .text
                .as_deref()
                .is_some_and(|t| t.contains("scanned or image-based")),
            "expected scanned-pdf placeholder, got {:?}",
            block.text,
        );
    }

    #[test]
    fn construct_messages_treats_whitespace_only_markdown_as_empty() {
        let asset = asset_with(Some("   \n\t  "), PdfTypeKind::TextBased);
        let blocks = asset.construct_messages();
        let ContentBlock::PlainText(block) = &blocks[0] else {
            panic!("expected a PlainText block");
        };
        assert!(
            block
                .text
                .as_deref()
                .is_some_and(|t| t.contains("scanned or image-based")),
        );
    }

    #[tokio::test]
    async fn saveable_asset_round_trips_to_json() {
        let asset = asset_with(Some("body"), PdfTypeKind::TextBased);
        assert_eq!(asset.get_asset_type(), "PdfAsset");
        assert_eq!(asset.get_unique_id(), "pdf-test-id");
        assert_eq!(asset.get_display_name(), "Lecture Notes");

        let bytes = asset.serialize_content().await.expect("serialize");
        let round: PdfAsset = serde_json::from_slice(&bytes).expect("deserialize");
        assert_eq!(round, asset);
    }
}
