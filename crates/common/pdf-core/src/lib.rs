//! OS-agnostic PDF parsing for Eurora.
//!
//! This crate is a thin, byte-oriented facade over
//! [`pdf-inspector`](https://github.com/firecrawl/pdf-inspector). It owns:
//!
//! - [`ParsedPdf`] — an owned, serializable summary of a parsed PDF
//!   (Markdown, page count, classification, optional title).
//! - [`PdfTypeKind`] — a stable, serde-friendly mirror of pdf-inspector's
//!   [`pdf_inspector::PdfType`] so `ParsedPdf` can be persisted or sent
//!   over the wire without leaking the upstream type's representation.
//! - [`PdfCoreError`] — a `thiserror` error that flattens pdf-inspector's
//!   IO / parse / encryption / not-a-pdf cases.
//!
//! The crate is deliberately byte-oriented: callers hand in a buffer and
//! receive an owned struct. Filesystem access and async scheduling belong
//! one layer up (see `euro-pdf` on the desktop side); the backend can call
//! [`parse_bytes`] directly on a download.

mod error;
mod kind;
mod parsed;

pub use error::PdfCoreError;
pub use kind::PdfTypeKind;
pub use parsed::ParsedPdf;

/// Parse a PDF from an in-memory buffer.
///
/// Runs pdf-inspector's full pipeline (detect → extract → markdown). Returns
/// [`ParsedPdf`] with `markdown == None` for scanned/image-based PDFs that
/// have no extractable text — callers should treat this as "PDF was opened
/// successfully but no text was available", not as a parser failure.
///
/// # Errors
///
/// Returns [`PdfCoreError`] when the buffer is not a PDF, the document is
/// encrypted, or the underlying parser fails.
pub fn parse_bytes(buffer: &[u8]) -> Result<ParsedPdf, PdfCoreError> {
    let result = pdf_inspector::process_pdf_mem(buffer)?;
    Ok(ParsedPdf::from(result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bytes_rejects_non_pdf_buffer() {
        let err = parse_bytes(b"this is definitely not a pdf").unwrap_err();
        assert!(
            matches!(err, PdfCoreError::NotAPdf(_)),
            "expected NotAPdf, got {err:?}",
        );
    }

    #[test]
    fn parse_bytes_rejects_empty_buffer() {
        let err = parse_bytes(&[]).unwrap_err();
        // Empty buffers surface as either NotAPdf or InvalidStructure
        // depending on pdf-inspector's validation order; either is correct.
        assert!(
            matches!(
                err,
                PdfCoreError::NotAPdf(_) | PdfCoreError::InvalidStructure
            ),
            "expected NotAPdf or InvalidStructure, got {err:?}",
        );
    }
}
