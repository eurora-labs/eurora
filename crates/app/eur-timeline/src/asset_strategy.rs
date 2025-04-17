//! Asset Strategy Pattern Implementation
//!
//! This module implements the Asset Strategy pattern for retrieving assets
//! from different sources. It provides a flexible way to switch between
//! different asset retrieval strategies at runtime.

use anyhow::Result;
use std::sync::Arc;

/// The AssetStrategy trait defines the interface for all asset retrieval strategies.
pub trait AssetStrategy: Send + Sync {
    /// Execute the strategy to retrieve assets.
    fn execute(&self) -> Result<Vec<u8>>;
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
    fn execute(&self) -> Result<Vec<u8>> {
        // Basic implementation - will be expanded later
        // For now, just return an empty vector
        Ok(Vec::new())
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
        match process_name {
            "browser" => {
                let strategy = Arc::new(BrowserStrategy::new());
                self.set_strategy(strategy);
            }
            // Add more strategies as needed
            _ => {
                // Default to no strategy
                self.strategy = None;
            }
        }
    }

    /// Retrieve assets using the current strategy.
    pub fn retrieve_assets(&self) -> Result<Vec<u8>> {
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
