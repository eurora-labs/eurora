use serde::{Deserialize, Serialize};

/// Serde-friendly mirror of [`pdf_inspector::PdfType`].
///
/// Defined here rather than re-exporting the upstream type because the
/// upstream `PdfType` does not derive `Serialize`/`Deserialize` and we want a
/// stable, persisted representation that does not change if pdf-inspector
/// adds variants in a non-breaking minor release.
///
/// Variants map one-to-one with the upstream enum today; new upstream variants
/// will land here as a `Other(String)` catch-all in the future if pdf-inspector
/// grows beyond the current four classifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PdfTypeKind {
    /// PDF has extractable text (`Tj`/`TJ` operators present).
    TextBased,
    /// PDF is scanned (no text operators, image-only pages).
    Scanned,
    /// PDF is image-heavy with no/minimal text.
    ImageBased,
    /// PDF mixes text-bearing and image-only pages.
    Mixed,
}

impl From<pdf_inspector::PdfType> for PdfTypeKind {
    fn from(value: pdf_inspector::PdfType) -> Self {
        match value {
            pdf_inspector::PdfType::TextBased => Self::TextBased,
            pdf_inspector::PdfType::Scanned => Self::Scanned,
            pdf_inspector::PdfType::ImageBased => Self::ImageBased,
            pdf_inspector::PdfType::Mixed => Self::Mixed,
        }
    }
}

impl PdfTypeKind {
    /// Whether pdf-inspector was able to extract Markdown for this PDF.
    ///
    /// Used by callers that want to differentiate "parser succeeded but the
    /// document has nothing to read" from "parser failed".
    #[must_use]
    pub fn has_extractable_text(self) -> bool {
        matches!(self, Self::TextBased | Self::Mixed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_upstream_covers_every_variant() {
        // If pdf-inspector adds a variant, this match becomes non-exhaustive
        // and the build fails — exactly the signal we want.
        for upstream in [
            pdf_inspector::PdfType::TextBased,
            pdf_inspector::PdfType::Scanned,
            pdf_inspector::PdfType::ImageBased,
            pdf_inspector::PdfType::Mixed,
        ] {
            let _: PdfTypeKind = upstream.into();
        }
    }

    #[test]
    fn has_extractable_text_matches_intent() {
        assert!(PdfTypeKind::TextBased.has_extractable_text());
        assert!(PdfTypeKind::Mixed.has_extractable_text());
        assert!(!PdfTypeKind::Scanned.has_extractable_text());
        assert!(!PdfTypeKind::ImageBased.has_extractable_text());
    }

    #[test]
    fn serde_uses_snake_case() {
        let json = serde_json::to_string(&PdfTypeKind::TextBased).unwrap();
        assert_eq!(json, "\"text_based\"");

        let parsed: PdfTypeKind = serde_json::from_str("\"image_based\"").unwrap();
        assert_eq!(parsed, PdfTypeKind::ImageBased);
    }
}
