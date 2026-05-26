use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

use crate::{PdfAsset, PdfError, parse::parse_path};

/// `(path, mtime)` keyed cache for parsed PDFs.
///
/// Reserved for the forthcoming `office::pdf::*` adapter: repeated reads of the
/// same focused document avoid re-parsing the multi-page body. We key on
/// modification time so an in-place edit (e.g. annotation save in Preview)
/// invalidates the cache and the next read re-parses, but a stable file is
/// parsed only once.
#[derive(Debug, Default)]
pub struct PdfCache {
    inner: Mutex<HashMap<PathBuf, CachedEntry>>,
}

#[derive(Debug, Clone)]
struct CachedEntry {
    mtime: Option<SystemTime>,
    asset: PdfAsset,
}

impl PdfCache {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Look up — or compute and store — the [`PdfAsset`] for `path`.
    ///
    /// On a cache hit the stored asset is cloned and returned. On a miss
    /// (or stale `mtime`) the file is re-parsed via [`parse_path`] and the
    /// fresh asset is inserted before returning.
    ///
    /// # Errors
    ///
    /// Forwards every error from [`parse_path`]. The cache is only updated
    /// on success, so a failed parse never poisons the slot.
    pub async fn get_or_parse(&self, path: &Path) -> Result<PdfAsset, PdfError> {
        let mtime = file_mtime(path).await;

        if let Some(hit) = self.lookup(path, mtime) {
            return Ok(hit);
        }

        let asset = parse_path(path).await?;
        self.insert(path.to_owned(), mtime, asset.clone());
        Ok(asset)
    }

    /// Remove a path from the cache; useful when a strategy stops tracking.
    pub fn invalidate(&self, path: &Path) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.remove(path);
        }
    }

    /// Drop every cached entry.
    pub fn clear(&self) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.clear();
        }
    }

    fn lookup(&self, path: &Path, mtime: Option<SystemTime>) -> Option<PdfAsset> {
        let guard = self.inner.lock().ok()?;
        let entry = guard.get(path)?;
        if entry.mtime == mtime {
            Some(entry.asset.clone())
        } else {
            None
        }
    }

    fn insert(&self, path: PathBuf, mtime: Option<SystemTime>, asset: PdfAsset) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.insert(path, CachedEntry { mtime, asset });
        }
    }
}

async fn file_mtime(path: &Path) -> Option<SystemTime> {
    tokio::fs::metadata(path)
        .await
        .ok()
        .and_then(|m| m.modified().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PdfTypeKind;

    fn sample_asset(path: &str) -> PdfAsset {
        PdfAsset::builder()
            .path(path.to_owned())
            .document_name("doc")
            .pdf_type(PdfTypeKind::TextBased)
            .page_count(1)
            .build()
    }

    #[tokio::test]
    async fn invalidate_removes_entry() {
        let cache = PdfCache::new();
        let path = PathBuf::from("/tmp/x.pdf");
        let asset = sample_asset("/tmp/x.pdf");

        cache.insert(path.clone(), None, asset.clone());
        assert!(cache.lookup(&path, None).is_some());

        cache.invalidate(&path);
        assert!(cache.lookup(&path, None).is_none());
    }

    #[tokio::test]
    async fn lookup_misses_when_mtime_changes() {
        let cache = PdfCache::new();
        let path = PathBuf::from("/tmp/x.pdf");
        let asset = sample_asset("/tmp/x.pdf");

        let t0 = SystemTime::UNIX_EPOCH;
        let t1 = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1);

        cache.insert(path.clone(), Some(t0), asset);
        assert!(cache.lookup(&path, Some(t0)).is_some());
        assert!(
            cache.lookup(&path, Some(t1)).is_none(),
            "newer mtime must miss the cache",
        );
    }

    #[tokio::test]
    async fn clear_drops_every_entry() {
        let cache = PdfCache::new();
        cache.insert(PathBuf::from("/a.pdf"), None, sample_asset("/a.pdf"));
        cache.insert(PathBuf::from("/b.pdf"), None, sample_asset("/b.pdf"));

        cache.clear();
        assert!(cache.lookup(Path::new("/a.pdf"), None).is_none());
        assert!(cache.lookup(Path::new("/b.pdf"), None).is_none());
    }
}
