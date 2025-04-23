//! Strategy module
//!
//! This module provides a minimal approach to strategy selection using MultiKeyMap.

use crate::{
    ActivityStrategy, browser_activity::BrowserStrategy, default_activity::DefaultStrategy,
};
use anyhow::{Context, Result};
use multi_key_map::MultiKeyMap;
use once_cell::sync::Lazy;
use std::sync::Arc;
use tracing::info;

/// Global strategy registry that is initialized only once
static REGISTRY: Lazy<StrategyRegistry> = Lazy::new(StrategyRegistry::default);

/// Type alias for strategy creation functions
type StrategyCreator =
    Arc<dyn Fn(&str, String, String) -> Result<Box<dyn ActivityStrategy>> + Send + Sync>;

/// Registry for activity strategies
///
/// This struct maintains a map of process names to strategy creators
/// and provides methods for registering and creating strategies.
pub struct StrategyRegistry {
    // Map from process name to strategy creator
    strategies: MultiKeyMap<String, StrategyCreator>,
    // Default strategy creator to use when no specific strategy is found
    default_creator: Option<StrategyCreator>,
}

impl StrategyRegistry {
    /// Create a new empty strategy registry
    pub fn new() -> Self {
        Self {
            strategies: MultiKeyMap::new(),
            default_creator: None,
        }
    }

    /// Register a strategy creator for specific process names
    ///
    /// # Arguments
    /// * `process_names` - List of process names this strategy supports
    /// * `creator` - Function that creates a strategy for these processes
    pub fn register<I, S>(&mut self, process_names: I, creator: StrategyCreator)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for name in process_names {
            self.strategies.insert(name.into(), Arc::clone(&creator));
        }
    }

    /// Set the default strategy creator to use when no specific strategy is found
    ///
    /// # Arguments
    /// * `creator` - Function that creates a default strategy
    pub fn set_default(&mut self, creator: StrategyCreator) {
        self.default_creator = Some(creator);
    }

    /// Set a strategy for the given process
    ///
    /// # Arguments
    /// * `process_name` - The name of the process
    /// * `display_name` - The display name to use for the activity
    /// * `icon` - The icon data as a base64 encoded string
    ///
    /// # Returns
    /// A Box<dyn ActivityStrategy> if a suitable strategy is found, or an error if no strategy supports the process
    pub async fn set_strategy(
        &self,
        process_name: &str,
        display_name: String,
        icon: String,
    ) -> Result<Box<dyn ActivityStrategy>> {
        // Try to find a creator for this process
        if let Some(creator) = self.strategies.get(process_name) {
            return creator(process_name, display_name, icon).context("Failed to create strategy");
        }

        // If no creator was found, try the default creator
        if let Some(default_creator) = &self.default_creator {
            info!("Using default strategy for process: {}", process_name);
            return default_creator(process_name, display_name, icon)
                .context("Failed to create default strategy");
        }

        // If no strategy supports this process, return an error
        Err(anyhow::anyhow!(
            "No strategy found for process: {}",
            process_name
        ))
    }

    /// Create a default registry with all built-in strategies
    pub fn default() -> Self {
        let mut registry = Self::new();

        // Register browser strategy for browser processes
        // We can't use block_on inside an async context, so we'll create a placeholder strategy
        // that will be replaced with the real strategy when select_strategy_for_process is called
        let browser_creator: StrategyCreator = Arc::new(|process_name, display_name, icon| {
            // Return a placeholder that will be replaced with the real strategy
            Err(anyhow::anyhow!(
                "BrowserStrategy requires async initialization"
            ))
        });

        // Use the BrowserStrategy's supported processes list
        let browser_processes = BrowserStrategy::get_supported_processes();
        registry.register(browser_processes, browser_creator);

        // Set default strategy creator as fallback
        let default_creator: StrategyCreator = Arc::new(|process_name, display_name, icon| {
            let strategy = DefaultStrategy::new(display_name, icon, process_name.to_string())?;
            Ok(Box::new(strategy))
        });

        registry.set_default(default_creator);

        registry
    }
}

impl Default for StrategyRegistry {
    fn default() -> Self {
        Self::default()
    }
}

/// Select the appropriate strategy based on the process name
///
/// This function is a convenience wrapper around StrategyRegistry::create_strategy.
///
/// # Arguments
/// * `process_name` - The name of the process
/// * `display_name` - The display name to use for the activity
/// * `icon` - The icon data as a base64 encoded string
///
/// # Returns
/// A Box<dyn ActivityStrategy> if a suitable strategy is found, or an error if no strategy supports the process
pub async fn select_strategy_for_process(
    process_name: &str,
    display_name: String,
    icon: String,
) -> Result<Box<dyn ActivityStrategy>> {
    // Log the process name
    info!("Selecting strategy for process: {}", process_name);

    // Check if this is a browser process using the BrowserStrategy's supported processes
    let is_browser = BrowserStrategy::get_supported_processes().contains(&process_name);

    // If it's a browser process, create a BrowserStrategy directly
    if is_browser {
        let strategy = BrowserStrategy::new(display_name, icon, process_name.to_string()).await?;

        return Ok(Box::new(strategy) as Box<dyn ActivityStrategy>);
    }

    // For non-browser processes, use the global registry
    // This avoids recreating the registry on each call
    REGISTRY
        .set_strategy(process_name, display_name, icon)
        .await
        .context(format!(
            "Failed to select strategy for process: {}",
            process_name
        ))
}
