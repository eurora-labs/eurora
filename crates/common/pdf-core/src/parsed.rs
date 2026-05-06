use serde::{Deserialize, Serialize};

use crate::PdfTypeKind;

/// Owned, serializable summary of a parsed PDF.
///
/// Built from [`pdf_inspector::PdfProcessResult`] but strips the upstream
/// fields we do not persist (timing data, layout-complexity flags) and uses
/// our own [`PdfTypeKind`] so the struct is fully serde-compatible.
///
/// `markdown` is `None` when pdf-inspector classifies the document as scanned
/// or image-based — the document parsed successfully, but there was no
/// extractable text. Callers should branch on this rather than on
/// [`PdfTypeKind`] alone, because mixed PDFs can also yield empty Markdown if
/// every text-bearing page failed extraction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParsedPdf {
    pub pdf_type: PdfTypeKind,
    pub markdown: Option<String>,
    pub page_count: u32,
    pub title: Option<String>,
}

impl ParsedPdf {
    /// `true` when the parser produced non-empty Markdown.
    #[must_use]
    pub fn has_text(&self) -> bool {
        self.markdown
            .as_deref()
            .is_some_and(|text| !text.trim().is_empty())
    }
}

impl From<pdf_inspector::PdfProcessResult> for ParsedPdf {
    fn from(result: pdf_inspector::PdfProcessResult) -> Self {
        Self {
            pdf_type: result.pdf_type.into(),
            // Treat empty-string markdown as "no text" so consumers do not
            // have to special-case the difference between `Some("")` and
            // `None` themselves.
            markdown: result.markdown.filter(|md| !md.trim().is_empty()),
            page_count: result.page_count,
            title: result.title.filter(|t| !t.trim().is_empty()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_text_treats_whitespace_as_empty() {
        let parsed = ParsedPdf {
            pdf_type: PdfTypeKind::TextBased,
            markdown: Some("   \n\t  ".into()),
            page_count: 1,
            title: None,
        };
        assert!(!parsed.has_text());
    }

    #[test]
    fn has_text_true_for_real_content() {
        let parsed = ParsedPdf {
            pdf_type: PdfTypeKind::TextBased,
            markdown: Some("# Hello\n\nWorld.".into()),
            page_count: 1,
            title: None,
        };
        assert!(parsed.has_text());
    }
}
