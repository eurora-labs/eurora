//! Timeline module for storing system state over time
//!
//! This crate provides functionality to capture system state at regular intervals
//! and store it in memory for later retrieval. The new implementation focuses on
//! simplicity, modularity, and ease of use.
//!
//! # Quick Start
//!
//! ```rust
//! use eur_timeline::TimelineManager;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create with sensible defaults
//!     let mut timeline = TimelineManager::new();
//!     
//!     // Start collection (handles focus tracking automatically)
//!     timeline.start().await?;
//!     
//!     // Get current activity
//!     if let Some(activity) = timeline.get_current_activity().await {
//!         println!("Current: {}", activity.name);
//!     }
//!     
//!     // Get recent activities
//!     let recent = timeline.get_recent_activities(10).await;
//!     for activity in recent {
//!         println!("Recent: {}", activity.name);
//!     }
//!     
//!     // Stop when done
//!     timeline.stop().await?;
//!     Ok(())
//! }
//! ```
//!
//! # Advanced Usage
//!
//! ```rust
//! use eur_timeline::{TimelineManager, TimelineConfig};
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Custom configuration
//!     let config = TimelineConfig::builder()
//!         .max_activities(500)
//!         .collection_interval(Duration::from_secs(5))
//!         .disable_focus_tracking()
//!         .build();
//!
//!     let mut timeline = TimelineManager::with_config(config);
//!
//!     // Start collection
//!     timeline.start().await?;
//!
//!     // Stop when done
//!     timeline.stop().await?;
//!
//!     Ok(())
//! }
//! ```

// Re-export main types for easy access
pub use collector::{CollectorService, CollectorStats};
pub use config::{CollectorConfig, FocusTrackingConfig, StorageConfig, TimelineConfig};
pub use error::{Result, TimelineError};
pub use manager::{TimelineManager, create_default_timeline, create_timeline};
pub use storage::{StorageStats, TimelineStorage};

// Re-export activity types for convenience
pub use eur_activity::{
    Activity, ActivityAsset, ActivitySnapshot, ActivityStrategy, ContextChip, DisplayAsset,
};
pub use ferrous_llm_core::Message;

// Internal modules
mod collector;
mod config;
mod error;
mod manager;
mod storage;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_new_api() {
        let timeline = TimelineManager::new();
        assert!(!timeline.is_running());
        assert!(timeline.is_empty().await);
    }

    #[tokio::test]
    async fn test_config_builder() {
        let config = TimelineConfig::builder()
            .max_activities(100)
            .collection_interval(std::time::Duration::from_secs(5))
            .disable_focus_tracking()
            .build();

        assert!(config.validate().is_ok());

        let timeline = TimelineManager::with_config(config);
        assert_eq!(timeline.get_config().storage.max_activities, 100);
    }

    #[tokio::test]
    async fn test_convenience_functions() {
        let timeline1 = create_default_timeline();
        assert!(!timeline1.is_running());

        let timeline2 = create_timeline(500, 5);
        assert_eq!(timeline2.get_config().storage.max_activities, 500);
    }
}
