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
//! The strategy emits a single `NewActivity` report at construction time
//! and otherwise has no per-turn work to do — the LLM pulls any
//! contextual data it needs through granular tools, not through this
//! strategy.

use agent_chain_core::messages::ContentBlocks;
use async_trait::async_trait;
use focus_tracker::FocusedWindow;
use serde_json::Value;
use thread_core::{ToolBackendCall, ToolErrorWire, WireToolDescriptor};
use tokio::sync::mpsc;

use crate::{
    error::ActivityResult,
    strategies::{ActivityReport, ActivityStrategyFunctionality, StrategyMetadata},
    types::ActivitySession,
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
}

impl DefaultStrategy {
    pub fn new(focused_window: FocusedWindow) -> Self {
        Self {
            focused_window,
            sender: None,
        }
    }

    fn build_session(&self) -> ActivitySession {
        ActivitySession::new_process(
            self.focused_window.process_name.clone(),
            self.focused_window.process_id,
            self.focused_window.window_title.clone(),
            self.focused_window.icon.clone(),
        )
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

        let _ = sender.send(ActivityReport::NewActivity(self.build_session()));

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

    async fn get_metadata(&self) -> ActivityResult<StrategyMetadata> {
        Ok(StrategyMetadata {
            url: None,
            title: self.focused_window.window_title.clone(),
            icon: self.focused_window.icon.clone(),
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
        let strategy = DefaultStrategy::new(window(42, "code", Some("main.rs")));
        let metadata = strategy.get_metadata().await.unwrap();
        assert_eq!(metadata.title.as_deref(), Some("main.rs"));
        assert!(metadata.url.is_none());
    }

    #[tokio::test]
    async fn start_tracking_emits_session_with_normalized_identity() {
        let mut strategy = DefaultStrategy::new(window(42, "Code.exe", Some("main.rs")));
        let (tx, mut rx) = mpsc::unbounded_channel::<ActivityReport>();

        strategy
            .start_tracking(&window(42, "Code.exe", Some("main.rs")), tx)
            .await
            .unwrap();

        let report = rx.recv().await.expect("activity report");
        match report {
            ActivityReport::NewActivity(session) => {
                assert_eq!(session.process_id, 42);
                assert_eq!(session.process_name, "Code.exe");
                assert_eq!(session.window_title.as_deref(), Some("main.rs"));
                // Identity normalises the process name; display defaults
                // to capitalize_first(key).
                assert_eq!(session.activity.key, "code");
                assert_eq!(session.activity.display_name, "Code");
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
            ActivityReport::NewActivity(session) => {
                assert_eq!(session.window_title.as_deref(), Some("main.rs"));
            }
            other => panic!("expected NewActivity, got {other:?}"),
        }

        let metadata = strategy.get_metadata().await.unwrap();
        assert_eq!(metadata.title.as_deref(), Some("main.rs"));
    }
}
