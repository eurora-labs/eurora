use thiserror::Error;

/// PDF parsing errors surfaced by `pdf-core`.
///
/// Flattens [`pdf_inspector::PdfError`] so callers don't have to depend on
/// the upstream crate to pattern-match on failure modes.
#[derive(Debug, Error)]
pub enum PdfCoreError {
    #[error("I/O error while reading PDF: {0}")]
    Io(#[from] std::io::Error),

    #[error("PDF parsing error: {0}")]
    Parse(String),

    #[error("PDF is encrypted")]
    Encrypted,

    #[error("PDF has invalid structure")]
    InvalidStructure,

    #[error("Not a PDF: {0}")]
    NotAPdf(String),
}

impl From<pdf_inspector::PdfError> for PdfCoreError {
    fn from(value: pdf_inspector::PdfError) -> Self {
        match value {
            pdf_inspector::PdfError::Io(err) => Self::Io(err),
            pdf_inspector::PdfError::Parse(msg) => Self::Parse(msg),
            pdf_inspector::PdfError::Encrypted => Self::Encrypted,
            pdf_inspector::PdfError::InvalidStructure => Self::InvalidStructure,
            pdf_inspector::PdfError::NotAPdf(msg) => Self::NotAPdf(msg),
        }
    }
}
