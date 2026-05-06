use std::path::{Path, PathBuf};

use crate::{PdfAsset, PdfError, classify::looks_like_pdf};

/// Read a PDF off disk, parse it via `pdf-core`, and wrap the result in a
/// [`PdfAsset`].
///
/// The CPU-bound parser runs inside [`tokio::task::spawn_blocking`] so this
/// function is safe to `await` from any async context (notably activity
/// strategies, which run on the shared Tauri runtime).
///
/// # Errors
///
/// - [`PdfError::NotAPdfPath`] if the path does not have a `.pdf` extension.
///   The caller is expected to filter with [`crate::classify_path`] first;
///   the redundant check here turns a silent misuse into a typed error.
/// - [`PdfError::NotFound`] if the file is missing.
/// - [`PdfError::Io`] for any other read failure.
/// - [`PdfError::Parse`] for parser failures (encrypted PDFs, malformed
///   structure, etc.).
/// - [`PdfError::Join`] if the blocking task panics.
pub async fn parse_path(path: impl AsRef<Path>) -> Result<PdfAsset, PdfError> {
    let path: PathBuf = path.as_ref().to_owned();

    if !looks_like_pdf(&path) {
        return Err(PdfError::NotAPdfPath(path));
    }

    let bytes = match tokio::fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(PdfError::NotFound(path));
        }
        Err(err) => return Err(PdfError::io(path, err)),
    };

    let parse_path = path.clone();
    let parsed = tokio::task::spawn_blocking(move || pdf_core::parse_bytes(&bytes))
        .await?
        .map_err(|err| {
            tracing::debug!(
                path = %parse_path.display(),
                error = %err,
                "pdf-core failed to parse PDF",
            );
            PdfError::Parse(err)
        })?;

    Ok(PdfAsset::from_parsed(path, parsed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn parse_path_rejects_non_pdf_extension() {
        let err = parse_path("/tmp/notes.txt").await.unwrap_err();
        assert!(
            matches!(&err, PdfError::NotAPdfPath(p) if p == &PathBuf::from("/tmp/notes.txt")),
            "expected NotAPdfPath, got {err:?}",
        );
    }

    #[tokio::test]
    async fn parse_path_returns_not_found_for_missing_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let missing = dir.path().join("does-not-exist.pdf");

        let err = parse_path(&missing).await.unwrap_err();
        assert!(
            matches!(&err, PdfError::NotFound(p) if p == &missing),
            "expected NotFound, got {err:?}",
        );
    }

    #[tokio::test]
    async fn parse_path_surfaces_parser_errors_for_garbage_pdf() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("fake.pdf");
        tokio::fs::write(&path, b"not actually a pdf")
            .await
            .expect("write fake pdf");

        let err = parse_path(&path).await.unwrap_err();
        assert!(
            matches!(err, PdfError::Parse(_)),
            "expected Parse error, got {err:?}",
        );
    }
}
