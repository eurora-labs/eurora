//! Strategy selection module
//!
//! This module provides functionality for selecting the appropriate activity strategy
//! based on the process name and other factors.

use crate::{ActivityStrategy, StrategyRegistry, StrategyWrapper};
use anyhow::{Context, Result};
use tracing::info;

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
    use super::*;

    // Tests would be added here
    // Note: These would need to be async tests
}
