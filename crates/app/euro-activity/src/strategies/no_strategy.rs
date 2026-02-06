//! No strategy implementation for when snapshots should be skipped
//!
//! This strategy is used when the focused process is the application itself
//! (Eurora) to avoid unnecessary snapshot retrieval calls.

use async_trait::async_trait;
use focus_tracker::FocusedWindow;
use log::debug;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::{
    error::ActivityResult,
    processes::{Eurora, ProcessFunctionality},
    strategies::{
        ActivityReport, ActivityStrategyFunctionality, StrategyMetadata, StrategySupport,
    },
    types::{ActivityAsset, ActivitySnapshot},
};

/// No-op strategy that returns empty results efficiently
/// Used when no snapshot tracking is needed (e.g., when Eurora is focused)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NoStrategy;

#[async_trait]
impl StrategySupport for NoStrategy {
    fn get_supported_processes() -> Vec<&'static str> {
        vec![Eurora.get_name()]
    }
}

#[async_trait]
impl ActivityStrategyFunctionality for NoStrategy {
    fn can_handle_process(&self, focus_window: &FocusedWindow) -> bool {
        // Check if the process is in the supported processes list
        NoStrategy::get_supported_processes().contains(&focus_window.process_name.as_str())
    }

    async fn start_tracking(
        &mut self,
        focus_window: &focus_tracker::FocusedWindow,
        _sender: mpsc::UnboundedSender<ActivityReport>,
    ) -> ActivityResult<()> {
        debug!(
            "NoStrategy: not starting tracking for {:?}",
            focus_window.process_name
        );
        // Intentionally do nothing - this strategy is for processes we want to ignore
        Ok(())
    }

    async fn handle_process_change(
        &mut self,
        focus_window: &FocusedWindow,
    ) -> ActivityResult<bool> {
        debug!(
            "NoStrategy: handling process change to: {}",
            focus_window.process_name
        );
        // Only continue if the new process is one we can handle (Eurora)
        Ok(self.can_handle_process(focus_window))
    }

    async fn stop_tracking(&mut self) -> ActivityResult<()> {
        debug!("NoStrategy: stopping tracking (no-op)");
        Ok(())
    }

    async fn retrieve_assets(&mut self) -> ActivityResult<Vec<ActivityAsset>> {
        debug!("NoStrategy: skipping asset retrieval");
        Ok(vec![])
    }

    async fn retrieve_snapshots(&mut self) -> ActivityResult<Vec<ActivitySnapshot>> {
        debug!("NoStrategy: skipping snapshot retrieval");
        Ok(vec![])
    }

    async fn get_metadata(&mut self) -> ActivityResult<StrategyMetadata> {
        Ok(StrategyMetadata::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_no_strategy_returns_empty() {
        let mut strategy = NoStrategy;

        let assets = strategy.retrieve_assets().await.unwrap();
        assert!(assets.is_empty());

        let snapshots = strategy.retrieve_snapshots().await.unwrap();
        assert!(snapshots.is_empty());
    }
}
