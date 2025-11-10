//! No strategy implementation for when snapshots should be skipped
//!
//! This strategy is used when the focused process is the application itself
//! (Eurora) to avoid unnecessary snapshot retrieval calls.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::{
    error::ActivityResult,
    strategies::{ActivityStrategyFunctionality, StrategyMetadata, StrategySupport},
    types::{ActivityAsset, ActivitySnapshot},
};

/// No-op strategy that returns empty results efficiently
/// Used when no snapshot tracking is needed (e.g., when Eurora is focused)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NoStrategy;

#[async_trait]
impl StrategySupport for NoStrategy {
    fn get_supported_processes() -> Vec<&'static str> {
        // NoStrategy doesn't explicitly support any processes
        // It's used programmatically when needed
        vec![]
    }
}

#[async_trait]
impl ActivityStrategyFunctionality for NoStrategy {
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

    async fn close_strategy(&mut self) -> ActivityResult<()> {
        Ok(())
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
