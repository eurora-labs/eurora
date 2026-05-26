use std::path::Path;

/// What a viewer strategy is looking at when the focused-window file is
/// "openable in this app".
///
/// PDF viewers like macOS Preview also open images, PostScript, etc. Strategies
/// use this to ignore non-PDF documents explicitly rather than treating every
/// focused-window file as a candidate for the PDF parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PreviewableKind {
    /// Path points to a PDF (extension `.pdf`, case-insensitive).
    Pdf,
    /// Path points to an image format Preview-class apps render natively.
    Image,
    /// Anything else (text files, Office docs, unknown extensions).
    Other,
}

/// Classify a file path purely by its extension.
///
/// This is intentionally extension-only: viewer strategies query it tens of
/// times per minute on focus changes, so paying for a magic-byte sniff would
/// be wasteful. The downstream parser ([`crate::parse_path`]) does its own
/// validation when it actually opens the file.
#[must_use]
pub fn classify_path(path: &Path) -> PreviewableKind {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return PreviewableKind::Other;
    };
    let ext = ext.to_ascii_lowercase();
    if ext == "pdf" {
        return PreviewableKind::Pdf;
    }
    if matches!(
        ext.as_str(),
        "png"
            | "jpg"
            | "jpeg"
            | "gif"
            | "bmp"
            | "tif"
            | "tiff"
            | "webp"
            | "heic"
            | "heif"
            | "ico"
            | "icns"
    ) {
        return PreviewableKind::Image;
    }
    PreviewableKind::Other
}

/// Convenience predicate: does this path look like a PDF?
#[must_use]
pub fn looks_like_pdf(path: &Path) -> bool {
    matches!(classify_path(path), PreviewableKind::Pdf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn pdf_extensions_match_case_insensitively() {
        assert_eq!(
            classify_path(Path::new("/tmp/Doc.pdf")),
            PreviewableKind::Pdf
        );
        assert_eq!(
            classify_path(Path::new("/tmp/Doc.PDF")),
            PreviewableKind::Pdf
        );
        assert_eq!(
            classify_path(Path::new("/tmp/Doc.PdF")),
            PreviewableKind::Pdf
        );
        assert!(looks_like_pdf(Path::new("a.pdf")));
    }

    #[test]
    fn common_image_extensions_classify_as_image() {
        for ext in [
            "png", "jpg", "jpeg", "gif", "bmp", "tif", "tiff", "webp", "heic", "heif", "ico",
            "icns",
        ] {
            let path = format!("/tmp/Photo.{ext}");
            assert_eq!(
                classify_path(Path::new(&path)),
                PreviewableKind::Image,
                "expected {ext} to classify as Image",
            );
        }
    }

    #[test]
    fn other_or_no_extension_falls_through() {
        assert_eq!(
            classify_path(Path::new("/tmp/notes.txt")),
            PreviewableKind::Other
        );
        assert_eq!(classify_path(Path::new("/tmp/Doc")), PreviewableKind::Other);
        assert_eq!(classify_path(Path::new("")), PreviewableKind::Other);
    }
}
