use agent_chain_core::messages::ContentBlocks;
use async_trait::async_trait;
use euro_process::AppProcess;
use focus_tracker::FocusedWindow;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thread_core::{ToolBackendCall, ToolErrorWire, WireToolDescriptor};
use tokio::sync::mpsc;

use crate::{
    error::ActivityResult,
    strategies::{
        ActivityReport, ActivityStrategy, ActivityStrategyFunctionality, StrategyMetadata,
        StrategySupport,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NoStrategy;

#[async_trait]
impl StrategySupport for NoStrategy {
    fn matches_process(process_name: &str) -> bool {
        AppProcess::from_process_name(process_name).is_some()
    }

    async fn create() -> ActivityResult<ActivityStrategy> {
        Ok(ActivityStrategy::NoStrategy(NoStrategy))
    }
}

#[async_trait]
impl ActivityStrategyFunctionality for NoStrategy {
    fn can_handle_process(&self, focus_window: &FocusedWindow) -> bool {
        NoStrategy::matches_process(&focus_window.process_name)
    }

    async fn start_tracking(
        &mut self,
        focus_window: &focus_tracker::FocusedWindow,
        _sender: mpsc::UnboundedSender<ActivityReport>,
    ) -> ActivityResult<()> {
        tracing::debug!(
            "NoStrategy: not starting tracking for {:?}",
            focus_window.process_name
        );
        Ok(())
    }

    async fn handle_process_change(
        &mut self,
        focus_window: &FocusedWindow,
    ) -> ActivityResult<bool> {
        tracing::debug!(
            "NoStrategy: handling process change to: {}",
            focus_window.process_name
        );
        Ok(self.can_handle_process(focus_window))
    }

    async fn stop_tracking(&mut self) -> ActivityResult<()> {
        tracing::debug!("NoStrategy: stopping tracking (no-op)");
        Ok(())
    }

    async fn get_metadata(&self) -> ActivityResult<StrategyMetadata> {
        Ok(StrategyMetadata::default())
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
