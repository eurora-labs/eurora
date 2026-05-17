//! Fallback activity strategy for applications without a specialised
//! integration.
//!
//! Each instance is bound to a single [`FocusedWindow`] at construction;
//! there is no "blank" `DefaultStrategy`. When the user moves focus to a
//! new process that none of the specialised strategies claim, the
//! dispatcher discards the current strategy and builds a fresh
//! `DefaultStrategy` for the new window — so the stored `FocusedWindow`
//! is always the one this strategy is responsible for.
//!
//! ### What it contributes
//!
//! When chat context is collected, [`retrieve_snapshots`] captures the
//! focused window via `xcap` and emits a [`ScreenshotSnapshot`] so the
//! image lands on the LLM as an `ImageContentBlock` alongside the
//! activity metadata.
//!
//! ### Privacy
//!
//! Capture only happens inside [`retrieve_snapshots`], which is itself
//! only invoked from
//! `crates/app/euro-timeline/src/collector.rs::refresh_current_activity`
//! — and that runs on demand when a chat turn collects context. If the
//! user removes the activity chip from the chat composer, the refresh
//! never fires for it and no screenshot is taken. There is no background
//! capture loop.
//!
//! ### Failure handling
//!
//! Capture is best-effort. A failure (denied permission on macOS, no
//! Wayland portal grant, child-process window the compositor doesn't
//! expose, …) is logged once per instance at `warn` and then at `debug`
//! for subsequent attempts, and the snapshot list comes back empty
//! rather than aborting the chat turn.

use async_trait::async_trait;
use euro_vision::capture;
use focus_tracker::FocusedWindow;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::mpsc;

use crate::{
    error::ActivityResult,
    snapshots::DefaultSnapshot,
    strategies::{ActivityReport, ActivityStrategyFunctionality, StrategyMetadata},
    types::{Activity, ActivityAsset, ActivitySnapshot},
};

#[derive(Clone)]
pub struct DefaultStrategy {
    /// The window this strategy is responsible for. Set at construction
    /// and (defensively) refreshed by `start_tracking`; never cleared
    /// during the strategy's lifetime.
    focused_window: FocusedWindow,

    /// Channel back to the collector. Populated by `start_tracking` so
    /// the construction site doesn't need to know about IPC.
    sender: Option<mpsc::UnboundedSender<ActivityReport>>,

    /// Flipped to `true` the first time a capture fails. Subsequent
    /// failures degrade to `debug` logging so we don't flood the log on
    /// systems where capture is unavailable. Wrapped in `Arc` so
    /// `Clone` instances share the warn state.
    capture_warned: Arc<AtomicBool>,
}

impl DefaultStrategy {
    pub fn new(focused_window: FocusedWindow) -> Self {
        Self {
            focused_window,
            sender: None,
            capture_warned: Arc::new(AtomicBool::new(false)),
        }
    }

    fn build_activity(&self) -> Activity {
        Activity::new(
            self.focused_window.window_title.clone().unwrap_or_default(),
            self.focused_window.window_title.clone(),
            self.focused_window.icon.clone(),
            self.focused_window.process_name.clone(),
            self.focused_window.process_id,
            vec![],
        )
    }

    fn note_capture_failure(&self, reason: impl std::fmt::Display) {
        let pid = self.focused_window.process_id;
        if self
            .capture_warned
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            tracing::warn!(
                "default strategy: window capture failed for pid {pid} ({reason}); falling back to no snapshot. On Wayland this requires an xdg-desktop-portal grant; on macOS, Screen Recording permission."
            );
        } else {
            tracing::debug!(
                "default strategy: window capture still failing for pid {pid} ({reason})"
            );
        }
    }
}

impl std::fmt::Debug for DefaultStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DefaultStrategy")
            .field("focused_window", &self.focused_window)
            .field("has_sender", &self.sender.is_some())
            .finish()
    }
}

#[async_trait]
impl ActivityStrategyFunctionality for DefaultStrategy {
    fn can_handle_process(&self, _focus_window: &FocusedWindow) -> bool {
        false
    }

    async fn start_tracking(
        &mut self,
        focus_window: &FocusedWindow,
        sender: mpsc::UnboundedSender<ActivityReport>,
    ) -> ActivityResult<()> {
        tracing::debug!(
            "Default strategy starting tracking for: {:?}",
            focus_window.process_name
        );

        // The dispatcher already built us with this window, but accept
        // the freshest view in case the focus tracker has refined it
        // (e.g. window title update) between construction and here.
        self.focused_window = focus_window.clone();
        self.sender = Some(sender.clone());

        let _ = sender.send(ActivityReport::NewActivity(self.build_activity()));

        Ok(())
    }

    async fn handle_process_change(
        &mut self,
        focus_window: &FocusedWindow,
    ) -> ActivityResult<bool> {
        tracing::debug!(
            "Default strategy handling process change to: {}",
            focus_window.process_name
        );
        // Always defer to the dispatcher: returning `false` causes the
        // collector to rebuild via `ActivityStrategy::new(focus_window)`,
        // which will pick a specialised strategy if one matches or a
        // fresh `DefaultStrategy` bound to the new window if not.
        Ok(false)
    }

    async fn stop_tracking(&mut self) -> ActivityResult<()> {
        tracing::debug!("Default strategy stopping tracking");
        self.sender = None;
        Ok(())
    }

    async fn retrieve_assets(&mut self) -> ActivityResult<Vec<ActivityAsset>> {
        // Screenshots belong in snapshots; this strategy contributes no
        // persistent assets.
        Ok(vec![])
    }

    async fn retrieve_snapshots(&mut self) -> ActivityResult<Vec<ActivitySnapshot>> {
        let pid = self.focused_window.process_id;
        match capture::capture_window_by_pid(pid).await {
            Ok(Some(image)) => Ok(vec![ActivitySnapshot::DefaultSnapshot(
                DefaultSnapshot::new(
                    self.focused_window.process_name.clone(),
                    self.focused_window.window_title.clone(),
                    image,
                ),
            )]),
            Ok(None) => {
                self.note_capture_failure("no matching window");
                Ok(vec![])
            }
            Err(err) => {
                self.note_capture_failure(err);
                Ok(vec![])
            }
        }
    }

    async fn get_metadata(&mut self) -> ActivityResult<StrategyMetadata> {
        Ok(StrategyMetadata {
            url: None,
            title: self.focused_window.window_title.clone(),
            icon: self.focused_window.icon.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(pid: u32, name: &str, title: Option<&str>) -> FocusedWindow {
        FocusedWindow {
            process_id: pid,
            process_name: name.to_string(),
            window_title: title.map(ToOwned::to_owned),
            icon: None,
        }
    }

    #[tokio::test]
    async fn retrieve_assets_is_always_empty() {
        let mut strategy = DefaultStrategy::new(window(42, "notes", Some("Untitled")));
        let assets = strategy.retrieve_assets().await.unwrap();
        assert!(assets.is_empty());
    }

    #[tokio::test]
    async fn handle_process_change_returns_false_to_force_redispatch() {
        let mut strategy = DefaultStrategy::new(window(42, "notes", None));
        // The argument represents the *new* focus the dispatcher is
        // asking about; the strategy should bow out so a fresh one gets
        // built for it.
        let result = strategy
            .handle_process_change(&window(7, "something-else", None))
            .await
            .unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn get_metadata_reports_window_title_from_constructor() {
        let mut strategy = DefaultStrategy::new(window(42, "code", Some("main.rs")));
        let metadata = strategy.get_metadata().await.unwrap();
        assert_eq!(metadata.title.as_deref(), Some("main.rs"));
        assert!(metadata.url.is_none());
    }

    #[tokio::test]
    async fn start_tracking_emits_activity_with_window_fields() {
        let mut strategy = DefaultStrategy::new(window(42, "code", Some("main.rs")));
        let (tx, mut rx) = mpsc::unbounded_channel::<ActivityReport>();

        strategy
            .start_tracking(&window(42, "code", Some("main.rs")), tx)
            .await
            .unwrap();

        let report = rx.recv().await.expect("activity report");
        match report {
            ActivityReport::NewActivity(activity) => {
                assert_eq!(activity.process_id, 42);
                assert_eq!(activity.process_name, "code");
                assert_eq!(activity.title.as_deref(), Some("main.rs"));
                assert_eq!(activity.name, "main.rs");
            }
            other => panic!("expected NewActivity, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn start_tracking_absorbs_refreshed_window_view() {
        // The dispatcher built us with a stale window (no title yet);
        // by the time start_tracking runs, the focus tracker has a
        // window_title. We should pick up the newer view so subsequent
        // snapshots / metadata use it.
        let mut strategy = DefaultStrategy::new(window(42, "code", None));
        let (tx, mut rx) = mpsc::unbounded_channel::<ActivityReport>();

        strategy
            .start_tracking(&window(42, "code", Some("main.rs")), tx)
            .await
            .unwrap();

        let report = rx.recv().await.expect("activity report");
        match report {
            ActivityReport::NewActivity(activity) => {
                assert_eq!(activity.title.as_deref(), Some("main.rs"));
            }
            other => panic!("expected NewActivity, got {other:?}"),
        }

        let metadata = strategy.get_metadata().await.unwrap();
        assert_eq!(metadata.title.as_deref(), Some("main.rs"));
    }
}
