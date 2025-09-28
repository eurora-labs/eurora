//! Default strategy implementation for unsupported applications

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::{
    DefaultAsset, DefaultSnapshot,
    error::ActivityResult,
    registry::{MatchScore, ProcessContext, StrategyCategory, StrategyFactory, StrategyMetadata},
    strategies::ActivityStrategy,
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

    /// Retrieve assets (creates a simple default asset)
    pub async fn retrieve_assets(&mut self) -> ActivityResult<Vec<ActivityAsset>> {
        debug!("Retrieving assets for default strategy");

        let asset = DefaultAsset::simple(self.name.clone())
            .with_metadata("process_name".to_string(), self.process_name.clone())
            .with_metadata("strategy".to_string(), "default".to_string());

        Ok(vec![ActivityAsset::DefaultAsset(asset)])
    }

    /// Retrieve snapshots (creates a simple state snapshot)
    pub async fn retrieve_snapshots(&mut self) -> ActivityResult<Vec<ActivitySnapshot>> {
        debug!("Retrieving snapshots for default strategy");

        let snapshot = DefaultSnapshot::new(format!(
            "Application '{}' is active (process: {})",
            self.name, self.process_name
        ));

        Ok(vec![ActivitySnapshot::DefaultSnapshot(snapshot)])
    }

    /// Gather current state as string
    pub fn gather_state(&self) -> String {
        format!("Default: {} ({})", self.name, self.process_name)
    }
}

/// Default strategy factory for creating default strategy instances
pub struct DefaultStrategyFactory;

impl DefaultStrategyFactory {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl StrategyFactory for DefaultStrategyFactory {
    async fn create_strategy(&self, context: &ProcessContext) -> ActivityResult<ActivityStrategy> {
        let strategy = DefaultStrategy::new(
            context.display_name.clone(),
            "default-icon".to_string(),
            context.process_name.clone(),
        )?;

        Ok(ActivityStrategy::Default(strategy))
    }

    fn supports_process(&self, _process_name: &str, _window_title: Option<&str>) -> MatchScore {
        // Default strategy supports all processes but with the lowest priority
        MatchScore::LOW
    }

    fn get_metadata(&self) -> StrategyMetadata {
        StrategyMetadata {
            id: "default".to_string(),
            name: "Default Strategy".to_string(),
            version: "2.0.0".to_string(),
            description: "Fallback strategy for applications without specific implementations"
                .to_string(),
            supported_processes: vec!["*".to_string()], // Supports all processes
            category: StrategyCategory::Default,
        }
    }
}

#[cfg(test)]
mod tests {
    use ferrous_focus::IconData;

    use super::*;

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
    fn test_factory_process_matching() {
        let factory = DefaultStrategyFactory::new();

        // Default strategy should match any process with low priority
        assert_eq!(
            factory.supports_process("any_process", None),
            MatchScore::LOW
        );
        assert_eq!(
            factory.supports_process("unknown_app", None),
            MatchScore::LOW
        );
        assert_eq!(factory.supports_process("", None), MatchScore::LOW);
    }

    #[test]
    fn test_factory_metadata() {
        let factory = DefaultStrategyFactory::new();
        let metadata = factory.get_metadata();

        assert_eq!(metadata.id, "default");
        assert_eq!(metadata.name, "Default Strategy");
        assert_eq!(metadata.version, "2.0.0");
        assert_eq!(metadata.category, StrategyCategory::Default);
        assert_eq!(metadata.supported_processes, vec!["*".to_string()]);
    }

    #[tokio::test]
    async fn test_factory_strategy_creation() {
        let factory = DefaultStrategyFactory::new();
        let context = ProcessContext::new(
            "unknown_app".to_string(),
            "Unknown Application".to_string(),
            IconData::default(),
        );

        let result = factory.create_strategy(&context).await;
        assert!(result.is_ok());

        let strategy = result.unwrap();
        assert_eq!(strategy.get_name(), "Unknown Application");
        assert_eq!(strategy.get_process_name(), "unknown_app");
    }
}
