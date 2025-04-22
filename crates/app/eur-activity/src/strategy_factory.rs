//! Default strategy module
//!
//! This module provides a default strategy factory that can be used as a fallback
//! for processes that don't match any specific strategy.

use crate::{ActivityAsset, ActivityStrategy, StrategyFactory};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::info;

/// Factory for creating default strategy instances
///
/// This factory always returns true for supports_process, making it suitable
/// as a fallback for processes that don't match any specific strategy.
pub struct DefaultStrategyFactory {
    /// The factory to delegate to for creating the actual strategy
    delegate: Box<dyn StrategyFactory>,
}

impl DefaultStrategyFactory {
    /// Create a new DefaultStrategyFactory with the given delegate
    ///
    /// # Arguments
    /// * `delegate` - The factory to delegate to for creating the actual strategy
    pub fn new<F>(delegate: F) -> Self
    where
        F: StrategyFactory + 'static,
    {
        Self {
            delegate: Box::new(delegate),
        }
    }
}

#[async_trait]
impl StrategyFactory for DefaultStrategyFactory {
    /// Always returns true, making this factory suitable as a fallback
    fn supports_process(&self, _process_name: &str) -> bool {
        true
    }

    /// Delegates to the wrapped factory to create a strategy
    async fn create_strategy(
        &self,
        process_name: &str,
        display_name: String,
        icon: String,
    ) -> Result<Box<dyn ActivityStrategy>> {
        info!("Using default strategy for process: {}", process_name);
        self.delegate
            .create_strategy(process_name, display_name, icon)
            .await
    }
}

pub struct StrategyWrapper {
    inner: Box<dyn ActivityStrategy>,
}

impl StrategyWrapper {
    /// Create a new StrategyWrapper around a boxed ActivityStrategy
    pub fn new(inner: Box<dyn ActivityStrategy>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl ActivityStrategy for StrategyWrapper {
    async fn retrieve_assets(&mut self) -> Result<Vec<Box<dyn ActivityAsset>>> {
        self.inner.retrieve_assets().await
    }

    fn gather_state(&self) -> String {
        self.inner.gather_state()
    }

    fn get_name(&self) -> &String {
        self.inner.get_name()
    }

    fn get_icon(&self) -> &String {
        self.inner.get_icon()
    }

    fn get_process_name(&self) -> &String {
        self.inner.get_process_name()
    }
}

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
        use crate::strategy_factory::DefaultStrategyFactory;

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

/// Select the appropriate strategy based on the process name
///
/// This function uses the StrategyRegistry to find and create the most appropriate
/// strategy for tracking the given process.
///
/// # Arguments
/// * `process_name` - The name of the process
/// * `display_name` - The display name to use for the activity
/// * `icon` - The icon data as a base64 encoded string
///
/// # Returns
/// A StrategyWrapper that implements ActivityStrategy and delegates to the selected strategy
pub async fn select_strategy_for_process(
    process_name: &str,
    display_name: String,
    icon: String,
) -> Result<StrategyWrapper> {
    // Create a default registry with all built-in strategies
    let registry = StrategyRegistry::default();

    // Log the process name
    info!("Selecting strategy for process: {}", process_name);

    // Use the registry to select a strategy
    let strategy = registry
        .select_strategy(process_name, display_name, icon)
        .await
        .context(format!(
            "Failed to select strategy for process: {}",
            process_name
        ))?;

    // Wrap the strategy in a StrategyWrapper
    Ok(StrategyWrapper::new(strategy))
}

#[cfg(test)]
mod tests {
    

    // Tests would be added here
    // Note: These would need to be async tests
}
