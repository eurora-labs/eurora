use std::path::PathBuf;

use bon::Builder;
use pdf_core::{ParsedPdf, PdfTypeKind};
use serde::{Deserialize, Serialize};

/// Desktop-side representation of a PDF the user has open.
///
/// Built from a [`ParsedPdf`] (see [`crate::parse_path`]) and a filesystem
/// path. Carries a stable UUID so the activity pipeline can dedupe and
/// reference it independently of the user-visible filename.
///
/// `path` is stored as a [`PathBuf`] but serialized as a UTF-8 string —
/// non-UTF-8 paths are coerced via `to_string_lossy`. PDF assets are
/// transmitted to the backend as `PlainTextContentBlock`s (Markdown), where
/// the file path is metadata rather than a load-bearing identifier, so a
/// lossy round-trip is acceptable in exchange for a serde-clean wire shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Builder)]
#[builder(on(String, into))]
pub struct PdfAsset {
    #[builder(default = uuid::Uuid::new_v4().to_string())]
    pub id: String,

    /// Filesystem path the PDF was loaded from. Stored as a string to keep
    /// serde stable across platforms.
    pub path: String,

    /// Display title — derived from the PDF's metadata title when available,
    /// otherwise the filename without extension.
    pub document_name: String,

    /// pdf-inspector's classification of the document.
    pub pdf_type: PdfTypeKind,

    /// Markdown body. `None` when pdf-inspector could not extract text
    /// (scanned/image-based PDFs).
    pub markdown: Option<String>,

    pub page_count: u32,
}

impl PdfAsset {
    /// Build a [`PdfAsset`] from a parsed PDF and the path it was loaded from.
    ///
    /// The display name prefers the PDF's metadata title (when non-empty)
    /// and otherwise falls back to the filename's stem. We never use the
    /// extension because the user-visible chip already shows the icon.
    #[must_use]
    pub fn from_parsed(path: PathBuf, parsed: ParsedPdf) -> Self {
        let document_name = parsed.title.clone().unwrap_or_else(|| filename_stem(&path));

        Self::builder()
            .path(path.to_string_lossy().into_owned())
            .document_name(document_name)
            .pdf_type(parsed.pdf_type)
            .maybe_markdown(parsed.markdown)
            .page_count(parsed.page_count)
            .build()
    }

    /// Whether the asset carries non-empty Markdown.
    #[must_use]
    pub fn has_text(&self) -> bool {
        self.markdown
            .as_deref()
            .is_some_and(|m| !m.trim().is_empty())
    }
}

fn filename_stem(path: &std::path::Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn parsed_with_title(title: Option<&str>) -> ParsedPdf {
        ParsedPdf {
            pdf_type: PdfTypeKind::TextBased,
            markdown: Some("# Body".into()),
            page_count: 3,
            title: title.map(ToOwned::to_owned),
        }
    }

    #[test]
    fn from_parsed_prefers_metadata_title() {
        let asset = PdfAsset::from_parsed(
            PathBuf::from("/tmp/raw-filename.pdf"),
            parsed_with_title(Some("The Real Title")),
        );
        assert_eq!(asset.document_name, "The Real Title");
    }

    #[test]
    fn from_parsed_falls_back_to_file_stem() {
        let asset = PdfAsset::from_parsed(
            PathBuf::from("/tmp/Lecture Notes.pdf"),
            parsed_with_title(None),
        );
        assert_eq!(asset.document_name, "Lecture Notes");
    }

    #[test]
    fn builder_auto_mints_uuid_when_id_omitted() {
        let asset = PdfAsset::builder()
            .path("/tmp/x.pdf")
            .document_name("x")
            .pdf_type(PdfTypeKind::TextBased)
            .page_count(0)
            .build();
        assert!(uuid::Uuid::parse_str(&asset.id).is_ok());
    }

    #[test]
    fn round_trips_through_serde() {
        let asset =
            PdfAsset::from_parsed(PathBuf::from("/tmp/Doc.pdf"), parsed_with_title(Some("T")));
        let json = serde_json::to_string(&asset).expect("serialize");
        let round: PdfAsset = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(round, asset);
    }

    #[test]
    fn has_text_treats_whitespace_only_as_empty() {
        let mut asset =
            PdfAsset::from_parsed(PathBuf::from("/tmp/Doc.pdf"), parsed_with_title(None));
        asset.markdown = Some("   \n\t  ".into());
        assert!(!asset.has_text());
    }

    #[test]
    fn filename_stem_handles_no_extension() {
        assert_eq!(super::filename_stem(Path::new("/tmp/raw")), "raw");
        assert_eq!(super::filename_stem(Path::new("doc.pdf")), "doc");
    }
}
