use async_trait::async_trait;
use focus_tracker::FocusedWindow;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::{
    error::ActivityResult,
    strategies::{
        ActivityReport, ActivityStrategy, ActivityStrategyFunctionality, StrategyMetadata,
        StrategySupport,
    },
    types::{Activity, ActivityAsset, ActivitySnapshot},
};

/// Fallback for applications that don't have specific strategy implementations.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DefaultStrategy;

#[async_trait]
impl StrategySupport for DefaultStrategy {
    fn get_supported_processes() -> Vec<&'static str> {
        vec![]
    }

    async fn create() -> ActivityResult<ActivityStrategy> {
        Ok(ActivityStrategy::DefaultStrategy(DefaultStrategy))
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

        let activity = Activity::new(
            focus_window.window_title.clone().unwrap_or_default(),
            focus_window.icon.clone(),
            focus_window.process_name.clone(),
            vec![],
        );

        let _ = sender.send(ActivityReport::NewActivity(activity));

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
        Ok(false)
    }

    async fn stop_tracking(&mut self) -> ActivityResult<()> {
        tracing::debug!("Default strategy stopping tracking");
        Ok(())
    }

    async fn retrieve_assets(&mut self) -> ActivityResult<Vec<ActivityAsset>> {
        tracing::debug!("Retrieving assets for default strategy");
        Ok(vec![])
    }

    async fn retrieve_snapshots(&mut self) -> ActivityResult<Vec<ActivitySnapshot>> {
        tracing::debug!("Retrieving snapshots for default strategy");
        Ok(vec![])
    }

    async fn get_metadata(&mut self) -> ActivityResult<StrategyMetadata> {
        Ok(StrategyMetadata::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_processes() {
        let processes = DefaultStrategy::get_supported_processes();
        assert!(processes.is_empty());
    }
}
