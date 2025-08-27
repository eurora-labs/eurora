//! Default strategy factory implementation

use anyhow::Result;
use async_trait::async_trait;
use tracing::info;

use crate::registry::{
    MatchScore, ProcessContext, StrategyCategory, StrategyFactory, StrategyMetadata,
};
use crate::{ActivityStrategy, default_activity::DefaultStrategy};

/// Factory for creating default activity strategies
pub struct DefaultStrategyFactory;

impl DefaultStrategyFactory {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl StrategyFactory for DefaultStrategyFactory {
    async fn create_strategy(&self, context: &ProcessContext) -> Result<Box<dyn ActivityStrategy>> {
        info!(
            "Creating default strategy for process: {}",
            context.process_name
        );

        let strategy = DefaultStrategy::new(
            context.display_name.clone(),
            "".to_string(), // Icon will be handled by the strategy itself
            context.process_name.clone(),
        )?;

        Ok(Box::new(strategy))
    }

    fn supports_process(&self, _process_name: &str, _window_title: Option<&str>) -> MatchScore {
        // Default strategy supports all processes but with the lowest priority
        MatchScore::LOW
    }

    fn get_metadata(&self) -> StrategyMetadata {
        StrategyMetadata {
            id: "default".to_string(),
            name: "Default Activity Strategy".to_string(),
            version: "1.0.0".to_string(),
            description: "Fallback strategy for applications without specific support".to_string(),
            supported_processes: vec!["*".to_string()], // Supports all processes
            category: StrategyCategory::Default,
        }
    }
}

impl Default for DefaultStrategyFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrous_focus::IconData;

    #[test]
    fn test_default_factory_creation() {
        let factory = DefaultStrategyFactory::new();
        let metadata = factory.get_metadata();

        assert_eq!(metadata.id, "default");
        assert_eq!(metadata.name, "Default Activity Strategy");
        assert!(matches!(metadata.category, StrategyCategory::Default));
    }

    #[test]
    fn test_process_matching() {
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

    #[tokio::test]
    async fn test_strategy_creation() {
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
