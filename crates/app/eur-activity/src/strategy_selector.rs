//! Strategy selection module
//!
//! This module provides functionality for selecting the appropriate activity strategy
//! based on the process name and other factors.

use crate::BrowserStrategy;
use anyhow::Result;

/// Select the appropriate strategy based on the process name
///
/// This function examines the process name and returns the most appropriate
/// strategy for tracking that process. It follows these rules:
///
/// - Browser processes (firefox, chrome, chromium, etc.) use BrowserStrategy
/// - Other processes currently default to BrowserStrategy but can be extended
///   to use more specialized strategies in the future
///
/// # Arguments
/// * `process_name` - The name of the process
/// * `display_name` - The display name to use for the activity
/// * `icon` - The icon data as a base64 encoded string
///
/// # Returns
/// A BrowserStrategy instance (currently the only implemented strategy)
/// In the future, this could be extended to return different strategy types
/// based on an enum or other mechanism
pub async fn select_strategy_for_process(
    process_name: &str,
    display_name: String,
    icon: String,
) -> Result<BrowserStrategy> {
    // Convert process name to lowercase for case-insensitive matching
    let proc_lower = process_name.to_lowercase();

    // Match against known process names
    // Currently we only have BrowserStrategy implemented, but this pattern
    // allows for easy extension in the future when more strategies are added
    match proc_lower.as_str() {
        // Browser processes
        "firefox" | "firefox-bin" | "firefox-esr" | "chrome" | "chromium" | "chromium-browser"
        | "brave" | "brave-browser" | "opera" | "vivaldi" | "edge" | "msedge" | "safari" => {
            // Log that we're using a browser strategy
            eprintln!("Using browser strategy for process: {}", process_name);
            BrowserStrategy::new(display_name, icon, process_name.to_string()).await
        }

        // Default case - for now use BrowserStrategy
        // In the future, this could be extended to use different strategies
        // based on the process type (e.g., document editors, media players, etc.)
        _ => {
            // Log that we're using the default strategy
            eprintln!("Using default strategy for process: {}", process_name);
            BrowserStrategy::new(display_name, icon, process_name.to_string()).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests would be added here
    // Note: These would need to be async tests
}
