//! Strategy implementations for different activity types
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
    fn gather_state(&self) -> String;
    fn get_name(&self) -> &str;
    fn get_icon(&self) -> &str;
    fn get_process_name(&self) -> &str;
}
