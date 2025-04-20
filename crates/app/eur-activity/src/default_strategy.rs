//! Default strategy module
//!
//! This module provides a default strategy factory that can be used as a fallback
//! for processes that don't match any specific strategy.

use crate::{ActivityAsset, ActivityStrategy, StrategyFactory};
use anyhow::Result;
use async_trait::async_trait;
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
