//! Desktop-side PDF integration for Eurora.
//!
//! `pdf-core` does the parsing; this crate handles everything that requires
//! a filesystem and a Tokio runtime:
//!
//! - [`PdfAsset`] — the activity-pipeline value type. A stable UUID, the
//!   filesystem path the PDF was loaded from, the parsed Markdown, and the
//!   document classification. Trait implementations binding it to
//!   `euro-activity` (`AssetFunctionality`, `SaveableAsset`) live in
//!   `euro-activity` to avoid pulling activity-side deps into this crate.
//! - [`parse_path`] — read a PDF off disk and return a fully populated
//!   [`PdfAsset`]. Runs the CPU-bound parser inside
//!   `tokio::task::spawn_blocking` so callers can `await` it from any
//!   strategy without blocking the runtime.
//! - [`PdfCache`] — `(path, mtime)` keyed cache so repeated
//!   `retrieve_assets()` calls do not re-parse a stable PDF.
//! - [`classify_path`] / [`PreviewableKind`] — cheap MIME-style check used
//!   by viewer strategies (e.g. macOS Preview, which opens images and
//!   PDFs through the same window) to decide whether a path is a PDF
//!   before paying the parse cost.

mod asset;
mod cache;
mod classify;
mod error;
mod parse;

pub use asset::PdfAsset;
pub use cache::PdfCache;
pub use classify::{PreviewableKind, classify_path, looks_like_pdf};
pub use error::PdfError;
pub use parse::parse_path;

// Re-exported for callers that pattern-match on the document
// classification without depending on `pdf-core` directly.
pub use pdf_core::PdfTypeKind;
