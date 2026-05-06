use std::path::PathBuf;
use thiserror::Error;

/// Errors surfaced by the desktop PDF pipeline.
///
/// Wraps [`pdf_core::PdfCoreError`] and adds variants for filesystem-level
/// failures that only matter when reading from disk.
#[derive(Debug, Error)]
pub enum PdfError {
    #[error("PDF file not found: {0}")]
    NotFound(PathBuf),

    #[error("path is not a PDF: {0}")]
    NotAPdfPath(PathBuf),

    #[error("I/O error reading {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("PDF parser failed: {0}")]
    Parse(#[from] pdf_core::PdfCoreError),

    /// `tokio::task::spawn_blocking` returned a panicked or cancelled handle.
    ///
    /// In practice this only fires on parser panics, which we surface
    /// rather than swallow so callers can decide whether to retry.
    #[error("PDF parser task failed to complete: {0}")]
    Join(#[from] tokio::task::JoinError),
}

impl PdfError {
    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}
