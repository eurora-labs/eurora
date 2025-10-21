//! Strategy implementations for different activity types

pub mod browser;
pub mod default;
pub mod processes;

pub use browser::BrowserStrategy;
pub use default::DefaultStrategy;

use crate::{
    error::ActivityResult,
    types::{ActivityAsset, ActivitySnapshot},
};

/// Enum containing all possible activity strategies
#[derive(Debug, Clone)]
pub enum ActivityStrategy {
    Browser(BrowserStrategy),
    Default(DefaultStrategy),
}

impl ActivityStrategy {
    /// Retrieve assets associated with this activity
    pub async fn retrieve_assets(&mut self) -> ActivityResult<Vec<ActivityAsset>> {
        match self {
            ActivityStrategy::Browser(strategy) => strategy.retrieve_assets().await,
            ActivityStrategy::Default(strategy) => strategy.retrieve_assets().await,
        }
    }

    /// Retrieve snapshots associated with this activity
    pub async fn retrieve_snapshots(&mut self) -> ActivityResult<Vec<ActivitySnapshot>> {
        match self {
            ActivityStrategy::Browser(strategy) => strategy.retrieve_snapshots().await,
            ActivityStrategy::Default(strategy) => strategy.retrieve_snapshots().await,
        }
    }

    /// Gather the current state of the activity
    pub fn gather_state(&self) -> String {
        match self {
            ActivityStrategy::Browser(strategy) => strategy.gather_state(),
            ActivityStrategy::Default(strategy) => strategy.gather_state(),
        }
    }

    /// Get name of the activity
    pub fn get_name(&self) -> &str {
        match self {
            ActivityStrategy::Browser(strategy) => &strategy.name,
            ActivityStrategy::Default(strategy) => &strategy.name,
        }
    }

    /// Get icon of the activity
    pub fn get_icon(&self) -> &str {
        match self {
            ActivityStrategy::Browser(strategy) => &strategy.icon,
            ActivityStrategy::Default(strategy) => &strategy.icon,
        }
    }

    /// Get process name of the activity
    pub fn get_process_name(&self) -> &str {
        match self {
            ActivityStrategy::Browser(strategy) => &strategy.process_name,
            ActivityStrategy::Default(strategy) => &strategy.process_name,
        }
    }
}
