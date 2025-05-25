//! Activity reporting module
//!
//! This module provides functionality for tracking and reporting activities.
//! It defines the Activity trait and the ActivityReporter struct, which
//! can be used to collect data from activities and store it in a timeline.
use std::collections::HashMap;

use tracing::info;

// use eur_timeline::TimelineRef;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use eur_prompt_kit::LLMMessage;
use serde::{Deserialize, Serialize};
pub mod browser_activity;
pub mod default_activity;
use anyhow::{Context, Result};
pub use browser_activity::BrowserStrategy;
use default_activity::DefaultStrategy;

#[taurpc::ipc_type]
pub struct ContextChip {
    pub id: String,
    pub extension_id: String,
    pub name: String,
    pub attrs: HashMap<String, String>,
    pub icon: Option<String>,
    pub position: Option<u32>,
}
#[derive(Serialize, Deserialize)]
pub struct DisplayAsset {
    pub name: String,
    // image base64
    pub icon: String,
}

impl DisplayAsset {
    pub fn new(name: String, icon: String) -> Self {
        Self { name, icon }
    }
}

pub trait ActivityAsset: Send + Sync {
    fn get_name(&self) -> &String;
    fn get_icon(&self) -> Option<&String>;

    fn construct_message(&self) -> LLMMessage;
    fn get_context_chip(&self) -> Option<ContextChip>;

    // fn get_display(&self) -> DisplayAsset;
}

pub trait ActivitySnapshot: Send + Sync {
    fn construct_message(&self) -> LLMMessage;

    fn get_updated_at(&self) -> u64;
    fn get_created_at(&self) -> u64;
}

pub struct Activity {
    /// Name of the activity
    pub name: String,

    /// Icon representing the activity
    pub icon: String,

    /// Process name of the activity
    pub process_name: String,

    /// Start time (Unix timestamp)
    pub start: DateTime<Utc>,

    /// End time (Unix timestamp)
    pub end: Option<DateTime<Utc>>,

    // /// Snapshots of the activity
    pub snapshots: Vec<Box<dyn ActivitySnapshot>>,
    /// Assets associated with the activity
    pub assets: Vec<Box<dyn ActivityAsset>>,
}

impl Activity {
    /// Create a new activity
    pub fn new(
        name: String,
        icon: String,
        process_name: String,
        assets: Vec<Box<dyn ActivityAsset>>,
    ) -> Self {
        Self {
            name,
            icon,
            process_name,
            start: Utc::now(),
            end: None,
            assets,
            snapshots: Vec::new(),
        }
    }

    pub fn get_display_assets(&self) -> Vec<DisplayAsset> {
        self.assets
            .iter()
            .filter_map(|asset| {
                if let Some(icon) = asset.get_icon() {
                    Some(DisplayAsset::new(asset.get_name().clone(), icon.clone()))
                } else {
                    Some(DisplayAsset::new(
                        asset.get_name().clone(),
                        self.icon.clone(),
                    ))
                }
            })
            .collect()
    }
    pub fn get_context_chips(&self) -> Vec<ContextChip> {
        self.assets
            .iter()
            .filter_map(|asset| asset.get_context_chip())
            .collect()
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

    // Check if this is a browser process
    if BrowserStrategy::get_supported_processes().contains(&process_name) {
        // For browser processes, create the BrowserStrategy directly
        // This avoids the need to block on an async function
        info!(
            "Creating BrowserStrategy for browser process: {}",
            process_name
        );
        let strategy = BrowserStrategy::new(process_name.to_string(), display_name, icon)
            .await
            .context(format!(
                "Failed to create browser strategy for process: {}",
                process_name
            ))?;
        return Ok(Box::new(strategy) as Box<dyn ActivityStrategy>);
    }

    DefaultStrategy::new(display_name, icon, process_name.to_string())
        .context(format!(
            "Failed to create default strategy for process: {}",
            process_name
        ))
        .map(|strategy| Box::new(strategy) as Box<dyn ActivityStrategy>)
}

/// Activity trait defines methods that must be implemented by activities
/// that can be tracked and reported.
#[async_trait]
pub trait ActivityStrategy: Send + Sync {
    /// Retrieve assets associated with this activity
    ///
    /// This method is called once when collection starts to gather
    /// initial assets related to the activity.
    async fn retrieve_assets(&mut self) -> Result<Vec<Box<dyn ActivityAsset>>>;

    /// Retrieve snapshots associated with this activity
    ///
    /// This method is called periodically to gather snapshots of the
    /// activity. The returned snapshots should represent the
    /// current state of the activity.
    async fn retrieve_snapshots(&mut self) -> Result<Vec<Box<dyn ActivitySnapshot>>>;

    /// Gather the current state of the activity
    ///
    /// This method is called periodically to collect the current state
    /// of the activity. The returned string should represent the state
    /// in a format that can be parsed and stored in the timeline.
    fn gather_state(&self) -> String;

    /// Get name of the activity
    fn get_name(&self) -> &String;
    /// Get icon of the activity
    fn get_icon(&self) -> &String;
    /// Get process name of the activity
    fn get_process_name(&self) -> &String;
}

#[cfg(test)]
mod tests {}
