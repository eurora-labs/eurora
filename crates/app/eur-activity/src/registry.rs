//! Strategy registry for dynamic activity strategy management

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::{
    error::{ActivityError, ActivityResult},
    strategies::ActivityStrategy,
};

/// Score indicating how well a strategy matches a process
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct MatchScore(pub f32);

impl MatchScore {
    pub const PERFECT: MatchScore = MatchScore(1.0);
    pub const HIGH: MatchScore = MatchScore(0.8);
    pub const MEDIUM: MatchScore = MatchScore(0.6);
    pub const LOW: MatchScore = MatchScore(0.4);
    pub const NO_MATCH: MatchScore = MatchScore(0.0);

    pub fn is_match(&self) -> bool {
        self.0 > 0.0
    }
}

/// Context information about a process for strategy selection
#[derive(Debug, Clone)]
pub struct ProcessContext {
    pub process_name: String,
    pub display_name: String,
    pub window_title: Option<String>,
    pub icon: image::RgbaImage,
    pub executable_path: Option<std::path::PathBuf>,
}

impl ProcessContext {
    pub fn new(process_name: String, display_name: String, icon: image::RgbaImage) -> Self {
        Self {
            process_name,
            display_name,
            window_title: None,
            icon,
            executable_path: None,
        }
    }

    pub fn with_window_title(mut self, title: String) -> Self {
        self.window_title = Some(title);
        self
    }

    pub fn with_executable_path(mut self, path: std::path::PathBuf) -> Self {
        self.executable_path = Some(path);
        self
    }
}

/// Metadata about a strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub supported_processes: Vec<String>,
    pub category: StrategyCategory,
}

/// Categories of activity strategies
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StrategyCategory {
    Browser,
    Development,
    Communication,
    Productivity,
    Media,
    System,
    Default,
}

/// Factory trait for creating activity strategies
#[async_trait]
pub trait StrategyFactory: Send + Sync {
    /// Create a new strategy instance for the given context
    async fn create_strategy(&self, context: &ProcessContext) -> ActivityResult<ActivityStrategy>;

    /// Check if this factory supports the given process
    fn supports_process(&self, process_name: &str, window_title: Option<&str>) -> MatchScore;

    /// Get metadata about this strategy
    fn get_metadata(&self) -> StrategyMetadata;
}

/// Registry for managing activity strategies
pub struct StrategyRegistry {
    factories: HashMap<String, Arc<dyn StrategyFactory>>,
    process_cache: HashMap<String, String>, // process_name -> strategy_id
}

impl StrategyRegistry {
    /// Create a new strategy registry
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
            process_cache: HashMap::new(),
        }
    }

    /// Register a strategy factory
    pub fn register_factory(&mut self, factory: Arc<dyn StrategyFactory>) {
        let metadata = factory.get_metadata();
        debug!(
            "Registering strategy factory: {} ({})",
            metadata.name, metadata.id
        );

        if self
            .factories
            .insert(metadata.id.clone(), factory)
            .is_some()
        {
            warn!(
                "Strategy factory with id '{}' was already registered; overwriting previous factory",
                metadata.id
            );
        }

        // Clear cache when new factories are registered
        self.process_cache.clear();
    }

    /// Select the best strategy for a given process context
    pub async fn select_strategy(
        &mut self,
        context: &ProcessContext,
    ) -> ActivityResult<ActivityStrategy> {
        debug!("Selecting strategy for process: {}", context.process_name);

        // Check cache first
        if let Some(strategy_id) = self.process_cache.get(&context.process_name) {
            if let Some(factory) = self.factories.get(strategy_id) {
                debug!("Using cached strategy: {}", strategy_id);
                return factory.create_strategy(context).await;
            } else {
                self.process_cache.remove(&context.process_name);
            }
        }

        // Find the best matching strategy
        let mut best_match: Option<(String, MatchScore)> = None;

        for (strategy_id, factory) in &self.factories {
            let score =
                factory.supports_process(&context.process_name, context.window_title.as_deref());

            if score.is_match() {
                debug!(
                    "Strategy {} scored {:.2} for process {}",
                    strategy_id, score.0, context.process_name
                );

                match &best_match {
                    None => best_match = Some((strategy_id.clone(), score)),
                    Some((_, best_score)) if score > *best_score => {
                        best_match = Some((strategy_id.clone(), score));
                    }
                    _ => {}
                }
            }
        }

        match best_match {
            Some((strategy_id, score)) => {
                debug!(
                    "Selected strategy {} with score {:.2} for process {}",
                    strategy_id, score.0, context.process_name
                );

                // Cache the result
                self.process_cache
                    .insert(context.process_name.clone(), strategy_id.clone());

                let factory = self.factories.get(&strategy_id).unwrap();
                factory.create_strategy(context).await
            }
            None => {
                warn!("No strategy found for process: {}", context.process_name);
                Err(ActivityError::InvalidData(format!(
                    "No strategy available for process: {}",
                    context.process_name
                )))
            }
        }
    }

    /// Get all registered strategies
    pub fn get_strategies(&self) -> Vec<StrategyMetadata> {
        self.factories
            .values()
            .map(|factory| factory.get_metadata())
            .collect()
    }

    /// Get a specific strategy by ID
    pub fn get_strategy(&self, id: &str) -> Option<StrategyMetadata> {
        self.factories.get(id).map(|factory| factory.get_metadata())
    }

    /// Clear the process cache
    pub fn clear_cache(&mut self) {
        self.process_cache.clear();
    }
}

impl Default for StrategyRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategies::ActivityStrategyFunctionality;

    struct MockStrategyFactory {
        metadata: StrategyMetadata,
        supported_processes: Vec<String>,
    }

    impl MockStrategyFactory {
        fn new(id: &str, supported_processes: Vec<String>) -> Self {
            Self {
                metadata: StrategyMetadata {
                    id: id.to_string(),
                    name: format!("{} Strategy", id),
                    version: "1.0.0".to_string(),
                    description: format!("Mock strategy for {}", id),
                    supported_processes: supported_processes.clone(),
                    category: StrategyCategory::Default,
                },
                supported_processes,
            }
        }
    }

    #[async_trait]
    impl StrategyFactory for MockStrategyFactory {
        async fn create_strategy(
            &self,
            context: &ProcessContext,
        ) -> ActivityResult<ActivityStrategy> {
            use crate::strategies::DefaultStrategy;
            let strategy = DefaultStrategy::new(
                context.display_name.clone(),
                "mock-icon".to_string(),
                context.process_name.clone(),
            )?;
            Ok(ActivityStrategy::DefaultStrategy(strategy))
        }

        fn supports_process(&self, process_name: &str, _window_title: Option<&str>) -> MatchScore {
            if self.supported_processes.contains(&process_name.to_string()) {
                MatchScore::PERFECT
            } else {
                MatchScore::NO_MATCH
            }
        }

        fn get_metadata(&self) -> StrategyMetadata {
            self.metadata.clone()
        }
    }

    #[tokio::test]
    async fn test_registry_creation() {
        let registry = StrategyRegistry::new();
        assert_eq!(registry.factories.len(), 0);
        assert_eq!(registry.get_strategies().len(), 0);
    }

    #[tokio::test]
    async fn test_factory_registration() {
        let mut registry = StrategyRegistry::new();
        let factory = Arc::new(MockStrategyFactory::new(
            "test",
            vec!["test_process".to_string()],
        ));

        registry.register_factory(factory);

        assert_eq!(registry.factories.len(), 1);
        assert_eq!(registry.get_strategies().len(), 1);

        let metadata = registry.get_strategy("test").unwrap();
        assert_eq!(metadata.id, "test");
        assert_eq!(metadata.name, "test Strategy");
    }

    #[tokio::test]
    async fn test_strategy_selection() {
        let mut registry = StrategyRegistry::new();
        let factory = Arc::new(MockStrategyFactory::new(
            "browser",
            vec!["firefox".to_string(), "chrome".to_string()],
        ));

        registry.register_factory(factory);

        let context = ProcessContext::new(
            "firefox".to_string(),
            "Firefox Browser".to_string(),
            image::RgbaImage::new(100, 100),
        );

        let strategy = registry.select_strategy(&context).await.unwrap();
        assert_eq!(strategy.get_name(), "Firefox Browser");
    }

    #[tokio::test]
    async fn test_no_matching_strategy() {
        let mut registry = StrategyRegistry::new();
        let factory = Arc::new(MockStrategyFactory::new(
            "browser",
            vec!["firefox".to_string()],
        ));

        registry.register_factory(factory);

        let context = ProcessContext::new(
            "unknown_app".to_string(),
            "Unknown App".to_string(),
            image::RgbaImage::new(100, 100),
        );

        let result = registry.select_strategy(&context).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_strategy_caching() {
        let mut registry = StrategyRegistry::new();
        let factory = Arc::new(MockStrategyFactory::new(
            "browser",
            vec!["firefox".to_string()],
        ));

        registry.register_factory(factory);

        let context = ProcessContext::new(
            "firefox".to_string(),
            "Firefox Browser".to_string(),
            image::RgbaImage::new(100, 100),
        );

        // First call should cache the result
        let _strategy1 = registry.select_strategy(&context).await.unwrap();
        assert_eq!(registry.process_cache.len(), 1);

        // Second call should use cache
        let _strategy2 = registry.select_strategy(&context).await.unwrap();
        assert_eq!(registry.process_cache.len(), 1);
    }
}
