//! Activity strategy backing the Microsoft Word integration.
//!
//! Activates whenever the focus tracker reports the Word process. The
//! strategy locates the connected Office add-in via the bridge's
//! [`BridgeService::find_clients_by_kind`] lookup (using
//! [`euro_office::MICROSOFT_WORD_KIND`]) — the OS PID of the Word
//! window and the add-in's session pid are unrelated, which is exactly
//! what the `app_kind` registration field exists for.
//!
//! The add-in's `GET_ASSETS` response carries a [`WordDocumentAsset`]
//! directly (no `NativeMessage` envelope, see `euro-office`'s crate
//! docs). The strategy wraps it into a [`WordAsset`] (assigning a
//! UUID) before handing it to the rest of the activity pipeline.

use std::sync::{
    Arc, RwLock,
    atomic::{AtomicU32, Ordering},
};

use async_trait::async_trait;
use euro_browser::BridgeService;
use euro_office::{OfficeApp, WordAsset, WordDocumentAsset, fetch_word_asset};
use focus_tracker::FocusedWindow;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::{
    Activity, ActivityError,
    error::ActivityResult,
    strategies::{
        ActivityReport, ActivityStrategy, ActivityStrategyFunctionality, StrategyMetadata,
        StrategySupport,
    },
    types::{ActivityAsset, ActivitySnapshot},
};

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct WordStrategy {
    #[serde(skip)]
    sender: Option<mpsc::UnboundedSender<ActivityReport>>,

    #[serde(skip)]
    bridge_service: Option<&'static BridgeService>,

    /// OS pid of the focused Word process, captured from the focus
    /// tracker. Used as `process_id` on emitted activities. Distinct
    /// from the add-in's session pid, which lives in the bridge
    /// registry and is never exposed on `Activity` records.
    #[serde(skip)]
    focused_word_pid: Arc<AtomicU32>,

    /// Document name from the most recent successful `GET_ASSETS`
    /// fetch. Used as a fallback title when subsequent metadata
    /// requests fail (e.g. the add-in briefly disconnects).
    #[serde(skip)]
    last_document_name: Arc<RwLock<Option<String>>>,
}

impl WordStrategy {
    pub async fn new() -> ActivityResult<Self> {
        let mut strategy = WordStrategy::default();
        strategy.initialize_service().await?;
        Ok(strategy)
    }

    async fn initialize_service(&mut self) -> ActivityResult<()> {
        let service = BridgeService::get_or_init().await;
        self.bridge_service = Some(service);
        Ok(())
    }

    fn require_service(&self) -> ActivityResult<&'static BridgeService> {
        self.bridge_service
            .ok_or_else(|| ActivityError::strategy("Bridge service not initialized"))
    }

    /// Refresh the cached document name from a fresh `GET_ASSETS` and
    /// return the wire payload, or `None` when no add-in is connected /
    /// the request fails.
    async fn refresh_document(&self) -> Option<WordDocumentAsset> {
        let service = self.bridge_service?;
        let asset = fetch_word_asset(service).await?;
        if let Ok(mut guard) = self.last_document_name.write() {
            *guard = Some(asset.document_name.clone());
        }
        Some(asset)
    }

    fn cached_document_name(&self) -> Option<String> {
        self.last_document_name
            .read()
            .ok()
            .and_then(|guard| guard.clone())
    }

    /// Build the [`Activity`] we report when Word becomes the focused
    /// window. Prefers the live document name from the add-in; falls
    /// back to the cached name and, finally, to the OS process name so
    /// the timeline always has a coherent entry even if the add-in
    /// hasn't connected yet.
    async fn build_activity_for_focus(&self, focus_window: &FocusedWindow) -> Activity {
        let live_name = self
            .refresh_document()
            .await
            .map(|asset| asset.document_name);
        let document_name = live_name
            .or_else(|| self.cached_document_name())
            .filter(|name| !name.is_empty());

        let activity_name = document_name
            .clone()
            .unwrap_or_else(|| focus_window.process_name.clone());
        let title = document_name.or_else(|| focus_window.window_title.clone());

        Activity::new(
            activity_name,
            title,
            focus_window.icon.clone(),
            focus_window.process_name.clone(),
            focus_window.process_id,
            vec![],
        )
    }

    async fn emit_activity_for_focus(&self, focus_window: &FocusedWindow) -> ActivityResult<()> {
        let sender = self
            .sender
            .as_ref()
            .ok_or_else(|| ActivityError::strategy("Sender not initialized"))?
            .clone();

        let activity = self.build_activity_for_focus(focus_window).await;

        if sender.send(ActivityReport::NewActivity(activity)).is_err() {
            tracing::warn!("Word strategy: receiver dropped while emitting activity");
        }
        Ok(())
    }
}

impl std::fmt::Debug for WordStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WordStrategy")
            .field(
                "focused_word_pid",
                &self.focused_word_pid.load(Ordering::Relaxed),
            )
            .field("has_sender", &self.sender.is_some())
            .field("bridge_service_initialized", &self.bridge_service.is_some())
            .finish()
    }
}

#[async_trait]
impl StrategySupport for WordStrategy {
    fn matches_process(process_name: &str) -> bool {
        OfficeApp::from_process_name(process_name) == Some(OfficeApp::Word)
    }

    async fn create() -> ActivityResult<ActivityStrategy> {
        Ok(ActivityStrategy::WordStrategy(WordStrategy::new().await?))
    }
}

#[async_trait]
impl ActivityStrategyFunctionality for WordStrategy {
    fn can_handle_process(&self, focus_window: &FocusedWindow) -> bool {
        WordStrategy::matches_process(&focus_window.process_name)
    }

    async fn start_tracking(
        &mut self,
        focus_window: &FocusedWindow,
        sender: mpsc::UnboundedSender<ActivityReport>,
    ) -> ActivityResult<()> {
        tracing::debug!(
            "Word strategy starting tracking for: {}",
            focus_window.process_name
        );

        self.sender = Some(sender);
        self.focused_word_pid
            .store(focus_window.process_id, Ordering::Relaxed);

        self.emit_activity_for_focus(focus_window).await
    }

    async fn handle_process_change(
        &mut self,
        focus_window: &FocusedWindow,
    ) -> ActivityResult<bool> {
        tracing::debug!(
            "Word strategy handling process change to: {}",
            focus_window.process_name
        );

        if !self.can_handle_process(focus_window) {
            self.stop_tracking().await?;
            return Ok(false);
        }

        let already_active =
            self.focused_word_pid.load(Ordering::Relaxed) == focus_window.process_id;
        if already_active {
            return Ok(true);
        }

        self.focused_word_pid
            .store(focus_window.process_id, Ordering::Relaxed);
        self.emit_activity_for_focus(focus_window).await?;
        Ok(true)
    }

    async fn stop_tracking(&mut self) -> ActivityResult<()> {
        tracing::debug!("Word strategy stopping tracking");
        self.focused_word_pid.store(0, Ordering::Relaxed);
        if let Ok(mut guard) = self.last_document_name.write() {
            *guard = None;
        }
        Ok(())
    }

    async fn retrieve_assets(&mut self) -> ActivityResult<Vec<ActivityAsset>> {
        let Some(service) = self.bridge_service else {
            return Ok(vec![]);
        };
        let Some(wire) = fetch_word_asset(service).await else {
            return Ok(vec![]);
        };

        if let Ok(mut guard) = self.last_document_name.write() {
            *guard = Some(wire.document_name.clone());
        }

        let asset = WordAsset::from(wire);
        Ok(vec![ActivityAsset::WordAsset(asset)])
    }

    async fn retrieve_snapshots(&mut self) -> ActivityResult<Vec<ActivitySnapshot>> {
        Ok(vec![])
    }

    async fn get_metadata(&mut self) -> ActivityResult<StrategyMetadata> {
        let _ = self.require_service()?;

        let title = match self.refresh_document().await {
            Some(asset) => Some(asset.document_name),
            None => self.cached_document_name(),
        };

        Ok(StrategyMetadata {
            url: None,
            title,
            icon: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_word_process_per_target_os() {
        #[cfg(target_os = "windows")]
        {
            assert!(WordStrategy::matches_process("WINWORD.EXE"));
            assert!(WordStrategy::matches_process("winword.exe"));
        }

        #[cfg(target_os = "linux")]
        assert!(WordStrategy::matches_process("winword"));

        #[cfg(target_os = "macos")]
        assert!(WordStrategy::matches_process("Microsoft Word"));
    }

    #[test]
    fn does_not_match_unknown_process() {
        assert!(!WordStrategy::matches_process(""));
        assert!(!WordStrategy::matches_process("not-word"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn does_not_match_other_office_apps_on_macos() {
        assert!(!WordStrategy::matches_process("Microsoft Excel"));
        assert!(!WordStrategy::matches_process("Microsoft PowerPoint"));
    }
}
