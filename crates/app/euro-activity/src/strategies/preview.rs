//! Activity strategy for macOS Preview.app.
//!
//! Activates whenever the focus tracker reports the macOS Preview process.
//! Preview opens both PDFs and image files through the same window class —
//! we ignore the latter (see [`PreviewableKind`]) and only emit activities
//! for actual PDFs.
//!
//! Document path discovery uses
//! [`focus_tracker::focused_document_url`], which queries the macOS
//! Accessibility API's `AXDocument` attribute on Preview's focused window.
//! That attribute returns a `file://` URL only after the user has saved /
//! opened a real on-disk PDF; freshly-imported scans or unsaved markups
//! return `None`, so a missing URL is a soft signal, not an error.
//!
//! Parsing remains the responsibility of `euro-pdf`. This strategy only
//! caches the focused-document path so the timeline's activity row carries
//! a meaningful name immediately — extracting the document body is a
//! future `office::pdf::*` adapter's concern.
//!
//! Other PDF viewers (Adobe Reader, Skim, …) will land as sibling
//! strategies that share the per-app focus-resolution glue while
//! delegating any content extraction to dedicated adapter tools.

use std::path::PathBuf;
use std::sync::{
    Arc, RwLock,
    atomic::{AtomicU32, Ordering},
};

use agent_chain_core::messages::ContentBlocks;
use async_trait::async_trait;
use euro_pdf::{PreviewableKind, classify_path};
use focus_tracker::{FocusTrackerError, FocusedWindow};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thread_core::{ToolBackendCall, ToolErrorWire, WireToolDescriptor};
use tokio::sync::mpsc;
use url::Url;

use crate::{
    ActivityError, ActivitySession,
    error::ActivityResult,
    strategies::{
        ActivityReport, ActivityStrategy, ActivityStrategyFunctionality, StrategyMetadata,
        StrategySupport,
    },
};

/// Process name reported by the focus tracker on macOS for Preview.app.
///
/// Preview is macOS-only; on other platforms [`PreviewStrategy::matches_process`]
/// always returns `false`, so the strategy compiles on every target but
/// never matches off macOS.
#[cfg(target_os = "macos")]
const PREVIEW_PROCESS_NAME: &str = "Preview";

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct PreviewStrategy {
    #[serde(skip)]
    sender: Option<mpsc::UnboundedSender<ActivityReport>>,

    /// OS pid of the focused Preview process. `0` when the strategy is
    /// not currently tracking.
    #[serde(skip)]
    focused_pid: Arc<AtomicU32>,

    /// Filesystem path of the most recently observed PDF. Used as a
    /// fallback when the Accessibility API briefly fails to return the
    /// document URL (e.g. during window transitions) and to derive
    /// metadata titles without re-running the parser.
    #[serde(skip)]
    last_pdf_path: Arc<RwLock<Option<PathBuf>>>,
}

impl PreviewStrategy {
    pub fn new() -> Self {
        Self::default()
    }

    fn cached_path(&self) -> Option<PathBuf> {
        self.last_pdf_path
            .read()
            .ok()
            .and_then(|guard| guard.clone())
    }

    fn store_path(&self, path: Option<PathBuf>) {
        if let Ok(mut guard) = self.last_pdf_path.write() {
            *guard = path;
        }
    }

    /// Resolve the focused PDF path for the currently tracked Preview pid.
    ///
    /// Returns:
    /// - `Ok(Some(path))` when Preview's focused window exposes a `file://`
    ///   URL pointing at a `.pdf` file.
    /// - `Ok(None)` when there is no tracked pid, no document URL,
    ///   the URL is non-local, or the path classifies as an image / other
    ///   (Preview opens images through the same window).
    /// - `Err(...)` only when the Accessibility API itself denies access
    ///   (so callers know the failure is policy, not absence).
    fn current_pdf_path(&self) -> ActivityResult<Option<PathBuf>> {
        let pid = self.focused_pid.load(Ordering::Relaxed);
        if pid == 0 {
            return Ok(None);
        }

        let url_str = match focus_tracker::focused_document_url(pid) {
            Ok(value) => value,
            Err(FocusTrackerError::PermissionDenied { context }) => {
                tracing::warn!(
                    "Preview strategy: accessibility access denied while resolving document URL ({context})"
                );
                return Ok(None);
            }
            Err(err) => {
                return Err(ActivityError::strategy(format!(
                    "failed to read AXDocument from Preview: {err}"
                )));
            }
        };

        let Some(url_str) = url_str else {
            return Ok(None);
        };

        let Some(path) = file_url_to_path(&url_str) else {
            tracing::debug!("Preview strategy: ignoring non-file document URL: {url_str}");
            return Ok(None);
        };

        match classify_path(&path) {
            PreviewableKind::Pdf => Ok(Some(path)),
            kind => {
                tracing::debug!("Preview strategy: ignoring {kind:?} document at {path:?}",);
                Ok(None)
            }
        }
    }

    /// Build the [`ActivitySession`] reported when Preview becomes
    /// focused.
    ///
    /// The session rolls up to a process-keyed parent (one bucket per
    /// "Preview" process, not one per PDF). The cached filename feeds
    /// the per-session `window_title` so the rail shows
    /// "My Notes.pdf" rather than the bare process name.
    fn build_session_for_focus(&self, focus_window: &FocusedWindow) -> ActivitySession {
        let cached_name = self.cached_path().as_deref().and_then(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .map(ToOwned::to_owned)
        });
        let title = cached_name.or_else(|| focus_window.window_title.clone());

        ActivitySession::new_process(
            focus_window.process_name.clone(),
            focus_window.process_id,
            title,
            focus_window.icon.clone(),
        )
    }

    fn emit_activity_for_focus(&self, focus_window: &FocusedWindow) -> ActivityResult<()> {
        let sender = self
            .sender
            .as_ref()
            .ok_or_else(|| ActivityError::strategy("Sender not initialized"))?
            .clone();

        let session = self.build_session_for_focus(focus_window);

        if sender.send(ActivityReport::NewActivity(session)).is_err() {
            tracing::warn!("Preview strategy: receiver dropped while emitting activity");
        }
        Ok(())
    }
}

impl std::fmt::Debug for PreviewStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreviewStrategy")
            .field("focused_pid", &self.focused_pid.load(Ordering::Relaxed))
            .field("has_sender", &self.sender.is_some())
            .field(
                "last_pdf_path",
                &self
                    .last_pdf_path
                    .read()
                    .map(|guard| guard.clone())
                    .unwrap_or(None),
            )
            .finish()
    }
}

#[async_trait]
impl StrategySupport for PreviewStrategy {
    /// Match macOS Preview by its focus-tracker process name.
    ///
    /// Preview is macOS-only. We compile this matcher on every target so the
    /// `ActivityStrategy` enum stays platform-agnostic, but it returns
    /// `false` everywhere except macOS — so registering this strategy on
    /// Linux or Windows is a no-op rather than a compile error.
    fn matches_process(process_name: &str) -> bool {
        #[cfg(target_os = "macos")]
        {
            process_name == PREVIEW_PROCESS_NAME
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = process_name;
            false
        }
    }

    async fn create() -> ActivityResult<ActivityStrategy> {
        Ok(ActivityStrategy::PreviewStrategy(PreviewStrategy::new()))
    }
}

#[async_trait]
impl ActivityStrategyFunctionality for PreviewStrategy {
    fn can_handle_process(&self, focus_window: &FocusedWindow) -> bool {
        PreviewStrategy::matches_process(&focus_window.process_name)
    }

    async fn start_tracking(
        &mut self,
        focus_window: &FocusedWindow,
        sender: mpsc::UnboundedSender<ActivityReport>,
    ) -> ActivityResult<()> {
        tracing::debug!(
            "Preview strategy starting tracking for: {}",
            focus_window.process_name
        );

        self.sender = Some(sender);
        self.focused_pid
            .store(focus_window.process_id, Ordering::Relaxed);
        // Drop any path observed for a previous Preview session — the new
        // window may be focused on a different document.
        self.store_path(None);

        // Best-effort: pre-resolve the focused document so the Activity we
        // emit carries a meaningful name immediately.
        if let Ok(Some(path)) = self.current_pdf_path() {
            self.store_path(Some(path));
        }

        self.emit_activity_for_focus(focus_window)
    }

    async fn handle_process_change(
        &mut self,
        focus_window: &FocusedWindow,
    ) -> ActivityResult<bool> {
        tracing::debug!(
            "Preview strategy handling process change to: {}",
            focus_window.process_name
        );

        if !self.can_handle_process(focus_window) {
            self.stop_tracking().await?;
            return Ok(false);
        }

        let already_active = self.focused_pid.load(Ordering::Relaxed) == focus_window.process_id;
        if already_active {
            // Same Preview process, but the user may have opened a
            // different document; refresh the cached path so the next
            // metadata read reflects reality.
            if let Ok(path) = self.current_pdf_path() {
                self.store_path(path);
            }
            return Ok(true);
        }

        self.focused_pid
            .store(focus_window.process_id, Ordering::Relaxed);
        self.store_path(None);
        if let Ok(Some(path)) = self.current_pdf_path() {
            self.store_path(Some(path));
        }
        self.emit_activity_for_focus(focus_window)?;
        Ok(true)
    }

    async fn stop_tracking(&mut self) -> ActivityResult<()> {
        tracing::debug!("Preview strategy stopping tracking");
        self.focused_pid.store(0, Ordering::Relaxed);
        self.store_path(None);
        Ok(())
    }

    async fn get_metadata(&self) -> ActivityResult<StrategyMetadata> {
        let path = match self.current_pdf_path()? {
            Some(p) => {
                self.store_path(Some(p.clone()));
                Some(p)
            }
            None => self.cached_path(),
        };

        let title = path.as_deref().and_then(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .map(ToOwned::to_owned)
        });

        Ok(StrategyMetadata {
            url: None,
            title,
            icon: None,
        })
    }

    async fn get_tools(&self) -> ActivityResult<Vec<WireToolDescriptor>> {
        Ok(vec![])
    }

    async fn get_context(&self) -> ActivityResult<ContentBlocks> {
        Ok(ContentBlocks::new())
    }

    async fn dispatch_tool(&self, call: ToolBackendCall) -> Result<Value, ToolErrorWire> {
        Err(ToolErrorWire::ContextUnavailable {
            tool: call.name,
            reason: "no tools available for this strategy".to_string(),
        })
    }
}

/// Convert a `file://` URL into a [`PathBuf`].
///
/// `AXDocument` returns a percent-encoded `file://` URL on macOS; we run it
/// through `url::Url` so a path like `My%20Notes.pdf` decodes correctly. Non-
/// `file` schemes and URLs that cannot be turned into a local path return
/// `None` — Preview can in principle open remote documents (e.g. via
/// Quick Look on a remote share), but we have no useful pipeline for those.
fn file_url_to_path(url_str: &str) -> Option<PathBuf> {
    let url = Url::parse(url_str).ok()?;
    if url.scheme() != "file" {
        return None;
    }
    url.to_file_path().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_preview_process_per_target_os() {
        #[cfg(target_os = "macos")]
        {
            assert!(PreviewStrategy::matches_process("Preview"));
            assert!(!PreviewStrategy::matches_process("preview"));
        }

        #[cfg(not(target_os = "macos"))]
        {
            assert!(!PreviewStrategy::matches_process("Preview"));
        }
    }

    #[test]
    fn does_not_match_unknown_process() {
        assert!(!PreviewStrategy::matches_process(""));
        assert!(!PreviewStrategy::matches_process("Microsoft Word"));
        assert!(!PreviewStrategy::matches_process("Safari"));
    }

    #[test]
    fn file_url_to_path_decodes_percent_encoded_paths() {
        let path = file_url_to_path("file:///Users/me/Lecture%20Notes.pdf")
            .expect("file URL should decode");
        assert_eq!(path, PathBuf::from("/Users/me/Lecture Notes.pdf"));
    }

    #[test]
    fn file_url_to_path_rejects_non_file_schemes() {
        assert!(file_url_to_path("https://example.com/x.pdf").is_none());
        assert!(file_url_to_path("about:blank").is_none());
    }

    #[test]
    fn file_url_to_path_rejects_garbage() {
        assert!(file_url_to_path("").is_none());
        assert!(file_url_to_path("not a url").is_none());
    }
}
