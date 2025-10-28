//! Default strategy implementation for unsupported applications

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::{
    DefaultAsset, DefaultSnapshot,
    error::ActivityResult,
    strategies::{ActivityStrategy, ActivityStrategyFunctionality, StrategySupport},
    types::{ActivityAsset, ActivitySnapshot},
};

/// Default strategy for applications that don't have specific implementations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultStrategy {
    pub name: String,
    pub icon: String,
    pub process_name: String,
}

impl DefaultStrategy {
    /// Create a new default strategy
    pub fn new(name: String, icon: String, process_name: String) -> ActivityResult<Self> {
        debug!("Creating DefaultStrategy for process: {}", process_name);

        Ok(Self {
            name,
            icon,
            process_name,
        })
    }
}

#[async_trait]
impl StrategySupport for DefaultStrategy {
    fn get_supported_processes() -> Vec<&'static str> {
        // Default strategy doesn't explicitly support any processes
        // It will be used as fallback for any unsupported process
        vec![]
    }

    async fn create_strategy(
        process_name: String,
        display_name: String,
        icon: String,
    ) -> ActivityResult<ActivityStrategy> {
        let strategy = Self::new(display_name, icon, process_name)?;
        Ok(ActivityStrategy::DefaultStrategy(strategy))
    }
}

#[async_trait]
impl ActivityStrategyFunctionality for DefaultStrategy {
    async fn retrieve_assets(&mut self) -> ActivityResult<Vec<ActivityAsset>> {
        debug!("Retrieving assets for default strategy");

        let asset = DefaultAsset::simple(self.name.clone())
            .with_metadata("process_name".to_string(), self.process_name.clone())
            .with_metadata("strategy".to_string(), "default".to_string());

        Ok(vec![ActivityAsset::DefaultAsset(asset)])
    }

    async fn retrieve_snapshots(&mut self) -> ActivityResult<Vec<ActivitySnapshot>> {
        debug!("Retrieving snapshots for default strategy");

        let snapshot = DefaultSnapshot::new(format!(
            "Application '{}' is active (process: {})",
            self.name, self.process_name
        ));

        Ok(vec![ActivitySnapshot::DefaultSnapshot(snapshot)])
    }

    async fn get_metadata(&mut self) -> Option<String> {
        None
    }

    fn gather_state(&self) -> String {
        format!("Default: {} ({})", self.name, self.process_name)
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_icon(&self) -> &str {
        &self.icon
    }

    fn get_process_name(&self) -> &str {
        &self.process_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategies::ActivityStrategyFunctionality;

    #[test]
    fn test_default_strategy_creation() {
        let strategy = DefaultStrategy::new(
            "Test App".to_string(),
            "test-icon".to_string(),
            "test_process".to_string(),
        );

        assert!(strategy.is_ok());
        let strategy = strategy.unwrap();
        assert_eq!(strategy.name, "Test App");
        assert_eq!(strategy.icon, "test-icon");
        assert_eq!(strategy.process_name, "test_process");
    }

    #[tokio::test]
    async fn test_retrieve_assets() {
        let mut strategy = DefaultStrategy::new(
            "Test App".to_string(),
            "test-icon".to_string(),
            "test_process".to_string(),
        )
        .unwrap();

        let assets = strategy.retrieve_assets().await.unwrap();
        assert_eq!(assets.len(), 1);

        match &assets[0] {
            ActivityAsset::DefaultAsset(asset) => {
                assert_eq!(asset.name, "Test App");
                assert_eq!(
                    asset.get_metadata("process_name"),
                    Some(&"test_process".to_string())
                );
                assert_eq!(asset.get_metadata("strategy"), Some(&"default".to_string()));
            }
            _ => panic!("Expected default asset"),
        }
    }

    #[tokio::test]
    async fn test_retrieve_snapshots() {
        let mut strategy = DefaultStrategy::new(
            "Test App".to_string(),
            "test-icon".to_string(),
            "test_process".to_string(),
        )
        .unwrap();

        let snapshots = strategy.retrieve_snapshots().await.unwrap();
        assert_eq!(snapshots.len(), 1);

        match &snapshots[0] {
            ActivitySnapshot::DefaultSnapshot(snapshot) => {
                assert!(snapshot.state.contains("Test App"));
                assert!(snapshot.state.contains("test_process"));
            }
            _ => panic!("Expected default snapshot"),
        }
    }

    #[test]
    fn test_gather_state() {
        let strategy = DefaultStrategy::new(
            "Test App".to_string(),
            "test-icon".to_string(),
            "test_process".to_string(),
        )
        .unwrap();

        let state = strategy.gather_state();
        assert_eq!(state, "Default: Test App (test_process)");
    }

    #[test]
    fn test_supported_processes() {
        let processes = DefaultStrategy::get_supported_processes();
        // Default strategy doesn't explicitly support any processes
        assert!(processes.is_empty());
    }

    #[tokio::test]
    async fn test_strategy_support_creation() {
        let result = DefaultStrategy::create_strategy(
            "test_process".to_string(),
            "Test Application".to_string(),
            "test-icon".to_string(),
        )
        .await;

        assert!(result.is_ok());
        let strategy = result.unwrap();
        assert_eq!(strategy.get_name(), "Test Application");
        assert_eq!(strategy.get_process_name(), "test_process");
    }
}
