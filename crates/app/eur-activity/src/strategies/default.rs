//! Default strategy implementation for unsupported applications

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::{
    ActivityError,
    error::ActivityResult,
    strategies::{ActivityStrategyFunctionality, StrategyMetadata, StrategySupport},
    types::{ActivityAsset, ActivitySnapshot},
};

/// Default strategy for applications that don't have specific implementations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DefaultStrategy;

#[async_trait]
impl StrategySupport for DefaultStrategy {
    fn get_supported_processes() -> Vec<&'static str> {
        // Default strategy doesn't explicitly support any processes
        // It will be used as fallback for any unsupported process
        vec![]
    }
}

#[async_trait]
impl ActivityStrategyFunctionality for DefaultStrategy {
    async fn retrieve_assets(&mut self) -> ActivityResult<Vec<ActivityAsset>> {
        debug!("Retrieving assets for default strategy");

        Ok(vec![])
    }

    async fn retrieve_snapshots(&mut self) -> ActivityResult<Vec<ActivitySnapshot>> {
        debug!("Retrieving snapshots for default strategy");
        Ok(vec![])
    }

    async fn get_metadata(&mut self) -> ActivityResult<StrategyMetadata> {
        Ok(StrategyMetadata::default())
    }

    async fn get_icon(&mut self) -> ActivityResult<image::RgbaImage> {
        Err(ActivityError::Unknown(
            "Custom icon not implemented for default strategy".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_processes() {
        let processes = DefaultStrategy::get_supported_processes();
        // Default strategy doesn't explicitly support any processes
        assert!(processes.is_empty());
    }
}
