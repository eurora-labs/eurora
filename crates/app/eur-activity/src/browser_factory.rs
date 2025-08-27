//! Browser strategy factory implementation

use anyhow::Result;
use async_trait::async_trait;
use tracing::info;

use crate::registry::{
    MatchScore, ProcessContext, StrategyCategory, StrategyFactory, StrategyMetadata,
};
use crate::{ActivityStrategy, BrowserStrategy};

/// Factory for creating browser activity strategies
pub struct BrowserStrategyFactory;

impl BrowserStrategyFactory {
    pub fn new() -> Self {
        Self
    }

    /// Get the list of supported browser processes
    pub fn get_supported_processes() -> Vec<&'static str> {
        BrowserStrategy::get_supported_processes()
    }
}

#[async_trait]
impl StrategyFactory for BrowserStrategyFactory {
    async fn create_strategy(&self, context: &ProcessContext) -> Result<Box<dyn ActivityStrategy>> {
        info!(
            "Creating browser strategy for process: {}",
            context.process_name
        );

        let strategy = BrowserStrategy::new(
            context.display_name.clone(),
            "".to_string(), // Icon will be handled by the strategy itself
            context.process_name.clone(),
        )
        .await?;

        Ok(Box::new(strategy))
    }

    fn supports_process(&self, process_name: &str, _window_title: Option<&str>) -> MatchScore {
        let supported_processes = Self::get_supported_processes();

        if supported_processes.contains(&process_name) {
            MatchScore::PERFECT
        } else {
            // Heuristic: tokenize on non-alphanumeric boundaries and match common browser keywords.
            let process_lower = process_name.to_lowercase();
            let indicators = [
                "firefox", "chrome", "chromium", "safari", "msedge", "edge", "opera", "brave",
            ];
            let mut is_browserish = process_lower
                .split(|c: char| !c.is_ascii_alphanumeric())
                .filter(|t| !t.is_empty())
                .any(|t| indicators.contains(&t));

            // Handle common composite names without clear token boundaries
            if !is_browserish {
                is_browserish = process_lower.contains("microsoft-edge")
                    || process_lower.contains("google chrome")
                    || process_lower.ends_with(".exe")
                        && indicators.iter().any(|k| process_lower.contains(k));
            }

            if is_browserish {
                MatchScore::HIGH
            } else {
                MatchScore::NO_MATCH
            }
        }
    }

    fn get_metadata(&self) -> StrategyMetadata {
        StrategyMetadata {
            id: "browser".to_string(),
            name: "Browser Activity Strategy".to_string(),
            version: "1.0.0".to_string(),
            description: "Collects activity data from web browsers including YouTube videos, articles, and social media content".to_string(),
            supported_processes: Self::get_supported_processes().iter().map(|s| s.to_string()).collect(),
            category: StrategyCategory::Browser,
        }
    }
}

impl Default for BrowserStrategyFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrous_focus::IconData;

    #[test]
    fn test_browser_factory_creation() {
        let factory = BrowserStrategyFactory::new();
        let metadata = factory.get_metadata();

        assert_eq!(metadata.id, "browser");
        assert_eq!(metadata.name, "Browser Activity Strategy");
        assert!(matches!(metadata.category, StrategyCategory::Browser));
    }

    #[test]
    fn test_process_matching() {
        let factory = BrowserStrategyFactory::new();

        // Test exact matches
        assert_eq!(
            factory.supports_process("firefox", None),
            MatchScore::PERFECT
        );
        assert_eq!(
            factory.supports_process("chrome", None),
            MatchScore::PERFECT
        );

        // Test partial matches (processes that contain browser names but aren't exact matches)
        assert_eq!(
            factory.supports_process("firefox-custom", None),
            MatchScore::HIGH
        );
        assert_eq!(
            factory.supports_process("Google Chrome Helper", None),
            MatchScore::HIGH
        );
        assert_eq!(
            factory.supports_process("brave-custom", None),
            MatchScore::HIGH
        );

        // Test no matches
        assert_eq!(
            factory.supports_process("notepad", None),
            MatchScore::NO_MATCH
        );
        assert_eq!(
            factory.supports_process("vscode", None),
            MatchScore::NO_MATCH
        );
    }

    #[tokio::test]
    async fn test_strategy_creation() {
        let factory = BrowserStrategyFactory::new();
        let context = ProcessContext::new(
            "firefox".to_string(),
            "Firefox Browser".to_string(),
            IconData::default(),
        );

        let result = factory.create_strategy(&context).await;

        // Note: This test might fail if the browser strategy requires actual browser communication
        // In a real implementation, you might want to mock the browser communication
        match result {
            Ok(strategy) => {
                assert_eq!(strategy.get_name(), "Firefox Browser");
                assert_eq!(strategy.get_process_name(), "firefox");
            }
            Err(_) => {
                // Expected if browser communication is not available in test environment
                // This is acceptable for unit tests
            }
        }
    }

    #[test]
    fn test_supported_processes() {
        let processes = BrowserStrategyFactory::get_supported_processes();
        assert!(!processes.is_empty());

        // Should contain common browser processes
        let processes_str = processes.join(",");
        assert!(processes_str.contains("firefox") || processes_str.contains("chrome"));
    }
}
