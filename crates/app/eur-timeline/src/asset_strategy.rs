//! Asset Strategy Pattern Implementation
//!
//! This module implements the Asset Strategy pattern for retrieving assets
//! from different sources. It provides a flexible way to switch between
//! different asset retrieval strategies at runtime.

use crate::activity::{ActivityAsset, AssetType};
use anyhow::Result;
use serde_json;
use std::sync::Arc;

/// The AssetStrategy trait defines the interface for all asset retrieval strategies.
pub trait AssetStrategy: Send + Sync {
    /// Execute the strategy to retrieve assets.
    fn execute(&self) -> Result<ActivityAsset>;
}

/// BrowserStrategy implements the AssetStrategy trait for retrieving assets from browsers.
pub struct BrowserStrategy {
    // Fields will be added as needed
}

impl BrowserStrategy {
    /// Create a new BrowserStrategy.
    pub fn new() -> Self {
        Self {}
    }
}

impl AssetStrategy for BrowserStrategy {
    fn execute(&self) -> Result<ActivityAsset> {
        // Basic implementation - will be expanded later
        // In a real implementation, this would retrieve actual browser data
        let data = serde_json::json!({
            "url": "https://example.com",
            "title": "Example Website",
            "content": "Example content from browser"
        });

        Ok(ActivityAsset::new(data, crate::activity::AssetType::Custom))
    }
}

/// AssetContext holds a reference to the current strategy and provides methods
/// to execute the strategy and switch between different strategies.
pub struct AssetContext {
    strategy: Option<Arc<dyn AssetStrategy>>,
}

impl AssetContext {
    /// Create a new AssetContext with no strategy set.
    pub fn new() -> Self {
        Self { strategy: None }
    }

    /// Set the strategy to use for asset retrieval.
    pub fn set_strategy(&mut self, strategy: Arc<dyn AssetStrategy>) {
        self.strategy = Some(strategy);
    }

    /// Set the strategy based on the process name.
    pub fn set_strategy_by_process_name(&mut self, process_name: &str) {
        match process_name.to_lowercase().as_str() {
            "browser" | "chrome" | "firefox" | "safari" | "edge" | "opera" => {
                let strategy = Arc::new(BrowserStrategy::new());
                self.set_strategy(strategy);
            }
            // Add more strategies as needed
            // For example, you could add strategies for different applications
            // "pdf_viewer" => { ... }
            // "video_player" => { ... }
            _ => {
                // For unknown process names, default to browser strategy for now
                // In a production environment, you might want to log this or handle differently
                let strategy = Arc::new(BrowserStrategy::new());
                self.set_strategy(strategy);
            }
        }
    }

    /// Retrieve assets using the current strategy.
    pub fn retrieve_assets(&self) -> Result<ActivityAsset> {
        match &self.strategy {
            Some(strategy) => strategy.execute(),
            None => Err(anyhow::anyhow!("No strategy set")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_strategy() {
        let strategy = BrowserStrategy::new();
        let result = strategy.execute();
        assert!(result.is_ok());
    }

    #[test]
    fn test_asset_context() {
        let mut context = AssetContext::new();

        // Test with no strategy
        let result = context.retrieve_assets();
        assert!(result.is_err());

        // Test with browser strategy
        let strategy = Arc::new(BrowserStrategy::new());
        context.set_strategy(strategy);
        let result = context.retrieve_assets();
        assert!(result.is_ok());

        // Test set_strategy_by_process_name
        let mut context = AssetContext::new();
        context.set_strategy_by_process_name("browser");
        let result = context.retrieve_assets();
        assert!(result.is_ok());
    }
}
