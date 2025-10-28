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

    fn gather_state(&self) -> String;
    fn get_name(&self) -> &str;
    fn get_icon(&self) -> &str;
    fn get_process_name(&self) -> &str;
}

/// Trait for strategies to declare which processes they support
#[async_trait]
pub trait StrategySupport {
    /// Returns list of exact process names this strategy supports
    fn get_supported_processes() -> Vec<&'static str>;

    /// Create a new instance of this strategy
    async fn create_strategy(
        process_name: String,
        display_name: String,
        icon: String,
    ) -> ActivityResult<ActivityStrategy>;
}

/// Simple strategy selection function that finds the first strategy supporting the given process
pub async fn select_strategy_for_process(
    process_name: &str,
    display_name: String,
    icon: String,
) -> ActivityResult<ActivityStrategy> {
    // Check BrowserStrategy first
    if BrowserStrategy::get_supported_processes().contains(&process_name) {
        return BrowserStrategy::create_strategy(process_name.to_string(), display_name, icon)
            .await;
    }

    // Fall back to DefaultStrategy for any unsupported process
    DefaultStrategy::create_strategy(process_name.to_string(), display_name, icon).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_select_browser_strategy() {
        let strategy = select_strategy_for_process(
            "firefox",
            "Firefox".to_string(),
            "firefox-icon".to_string(),
        )
        .await
        .unwrap();

        assert_eq!(strategy.get_process_name(), "firefox");
        assert_eq!(strategy.get_name(), "Firefox");
    }

    #[tokio::test]
    async fn test_select_default_strategy() {
        let strategy = select_strategy_for_process(
            "notepad",
            "Notepad".to_string(),
            "notepad-icon".to_string(),
        )
        .await
        .unwrap();

        assert_eq!(strategy.get_process_name(), "notepad");
        assert_eq!(strategy.get_name(), "Notepad");
    }

    #[test]
    fn test_browser_supported_processes() {
        let processes = BrowserStrategy::get_supported_processes();
        assert!(!processes.is_empty());
        // Should contain browser process names based on OS
    }
}
