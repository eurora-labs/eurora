//! Simplified strategy implementations for different activity types

use async_trait::async_trait;
use enum_dispatch::enum_dispatch;

pub mod browser;
pub mod default;
pub mod processes;

pub use browser::BrowserStrategy;
pub use default::DefaultStrategy;

use crate::{
    error::ActivityResult,
    types::{ActivityAsset, ActivitySnapshot},
};

#[derive(Debug, Clone, Default)]
pub struct StrategyMetadata {
    pub icon_base64: Option<String>,
}

/// Enum containing all possible activity strategies
#[enum_dispatch(ActivityStrategyFunctionality)]
#[derive(Debug, Clone)]
pub enum ActivityStrategy {
    BrowserStrategy,
    DefaultStrategy,
}

#[async_trait]
#[enum_dispatch]
pub trait ActivityStrategyFunctionality {
    async fn retrieve_assets(&mut self) -> ActivityResult<Vec<ActivityAsset>>;
    async fn retrieve_snapshots(&mut self) -> ActivityResult<Vec<ActivitySnapshot>>;
    async fn get_metadata(&mut self) -> ActivityResult<StrategyMetadata>;
}

impl ActivityStrategy {
    pub async fn new(process_name: &str) -> ActivityResult<ActivityStrategy> {
        if BrowserStrategy::get_supported_processes().contains(&process_name) {
            return Ok(ActivityStrategy::BrowserStrategy(
                BrowserStrategy::new().await?,
            ));
        }

        Ok(ActivityStrategy::DefaultStrategy(DefaultStrategy::default()))
    }
}

/// Trait for strategies to declare which processes they support
#[async_trait]
pub trait StrategySupport {
    /// Returns list of exact process names this strategy supports
    fn get_supported_processes() -> Vec<&'static str>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_supported_processes() {
        let processes = BrowserStrategy::get_supported_processes();
        assert!(!processes.is_empty());
        // Should contain browser process names based on OS
    }
}
