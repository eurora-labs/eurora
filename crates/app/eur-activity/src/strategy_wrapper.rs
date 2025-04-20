//! Strategy wrapper module
//!
//! This module provides a wrapper type that implements ActivityStrategy
//! and delegates to a boxed trait object.

use crate::ActivityAsset;
use crate::ActivityStrategy;
use anyhow::Result;
use async_trait::async_trait;

/// A wrapper around a Box<dyn ActivityStrategy> that implements ActivityStrategy
///
/// This allows us to use a trait object with APIs that expect a concrete type
/// that implements ActivityStrategy.
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
