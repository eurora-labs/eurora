//! Strategy registry module
//!
//! This module provides a registry for activity strategies, allowing them to be
//! registered and selected based on the process name.

use crate::{ActivityStrategy, StrategyFactory};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::sync::Arc;

/// Registry for activity strategies
///
/// This struct maintains a list of strategy factories that can be used to create
/// strategy instances for specific processes.
pub struct StrategyRegistry {
    factories: Vec<Arc<dyn StrategyFactory>>,
}

impl StrategyRegistry {
    /// Create a new empty strategy registry
    pub fn new() -> Self {
        Self {
            factories: Vec::new(),
        }
    }

    /// Register a strategy factory
    ///
    /// # Arguments
    /// * `factory` - The strategy factory to register
    pub fn register<F>(&mut self, factory: F)
    where
        F: StrategyFactory + 'static,
    {
        self.factories.push(Arc::new(factory));
    }

    /// Select a strategy for the given process
    ///
    /// This method iterates through all registered factories and returns the first
    /// strategy that supports the given process.
    ///
    /// # Arguments
    /// * `process_name` - The name of the process
    /// * `display_name` - The display name to use for the activity
    /// * `icon` - The icon data as a base64 encoded string
    ///
    /// # Returns
    /// A Box<dyn ActivityStrategy> if a suitable strategy is found, or an error if no strategy supports the process
    pub async fn select_strategy(
        &self,
        process_name: &str,
        display_name: String,
        icon: String,
    ) -> Result<Box<dyn ActivityStrategy>> {
        // Find the first factory that supports this process
        for factory in &self.factories {
            if factory.supports_process(process_name) {
                return factory
                    .create_strategy(process_name, display_name, icon)
                    .await
                    .context("Failed to create strategy");
            }
        }

        // If no factory supports this process, return an error
        Err(anyhow::anyhow!(
            "No strategy found for process: {}",
            process_name
        ))
    }

    /// Create a default registry with all built-in strategies
    ///
    /// This method creates a new registry and registers all built-in strategies.
    /// It also registers a DefaultStrategyFactory as a fallback for processes
    /// that don't match any specific strategy.
    pub fn default() -> Self {
        use crate::browser_activity::BrowserStrategyFactory;
        use crate::default_strategy::DefaultStrategyFactory;

        let mut registry = Self::new();

        // Register specific strategies first
        registry.register(BrowserStrategyFactory::new());
        // Register other built-in strategies here

        // Register the default strategy factory as a fallback
        // This should be registered last so it only matches if no other strategy does
        registry.register(DefaultStrategyFactory::new(BrowserStrategyFactory::new()));

        registry
    }
}

// Implement Default trait for StrategyRegistry
impl Default for StrategyRegistry {
    fn default() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Tests would be added here
}
