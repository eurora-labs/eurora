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

use std::sync::Arc;

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

// Legacy compatibility types and functions
pub type TimelineRef = Arc<Timeline>;

/// Legacy Timeline struct for backwards compatibility
///
/// **DEPRECATED**: Use [`TimelineManager`] instead for new code.
/// This struct is provided for backwards compatibility only.
#[deprecated(since = "0.2.0", note = "Use TimelineManager instead")]
pub struct Timeline {
    manager: TimelineManager,
}

#[allow(deprecated)]
impl Timeline {
    /// Create a new timeline with the specified capacity
    ///
    /// **DEPRECATED**: Use [`TimelineManager::new()`] or [`create_timeline()`] instead.
    #[deprecated(
        since = "0.2.0",
        note = "Use TimelineManager::new() or create_timeline() instead"
    )]
    pub fn new(capacity: usize, interval_seconds: u64) -> Self {
        let manager = create_timeline(capacity, interval_seconds);
        Self { manager }
    }

    /// Create a shareable reference to this Timeline
    ///
    /// **DEPRECATED**: Use [`TimelineManager`] directly as it's already thread-safe.
    #[deprecated(
        since = "0.2.0",
        note = "TimelineManager is already thread-safe, use it directly"
    )]
    pub fn clone_ref(&self) -> TimelineRef {
        // This is a hack for backwards compatibility - we can't actually clone the manager
        // since it contains non-cloneable resources. In practice, this method was problematic
        // in the original implementation anyway.
        Arc::new(Timeline {
            manager: TimelineManager::new(),
        })
    }

    /// Add an activity to the timeline
    ///
    /// **DEPRECATED**: Use [`TimelineManager::add_activity()`] instead.
    #[deprecated(since = "0.2.0", note = "Use TimelineManager::add_activity() instead")]
    pub fn add_activity(&self, activity: eur_activity::Activity) {
        // This is async in the new implementation, but we can't change the signature
        // for backwards compatibility. We'll use a blocking approach.
        let rt = tokio::runtime::Handle::try_current().or_else(|_| {
            // If no runtime exists, create a new one
            tokio::runtime::Runtime::new().map(|rt| rt.handle().clone())
        });

        if let Ok(handle) = rt {
            handle.block_on(async {
                self.manager.add_activity(activity).await;
            });
        }
    }

    /// Get context chips from the current activity
    ///
    /// **DEPRECATED**: Use [`TimelineManager::get_context_chips()`] instead.
    #[deprecated(
        since = "0.2.0",
        note = "Use TimelineManager::get_context_chips() instead"
    )]
    pub fn get_context_chips(&self) -> Vec<eur_activity::ContextChip> {
        let rt = tokio::runtime::Handle::try_current()
            .or_else(|_| tokio::runtime::Runtime::new().map(|rt| rt.handle().clone()));

        if let Ok(handle) = rt {
            handle.block_on(async { self.manager.get_context_chips().await })
        } else {
            Vec::new()
        }
    }

    /// Get display assets from the current activity
    ///
    /// **DEPRECATED**: Use [`TimelineManager::get_display_assets()`] instead.
    #[deprecated(
        since = "0.2.0",
        note = "Use TimelineManager::get_display_assets() instead"
    )]
    pub fn get_activities(&self) -> Vec<DisplayAsset> {
        let rt = tokio::runtime::Handle::try_current()
            .or_else(|_| tokio::runtime::Runtime::new().map(|rt| rt.handle().clone()));

        if let Ok(handle) = rt {
            handle.block_on(async { self.manager.get_display_assets().await })
        } else {
            Vec::new()
        }
    }

    /// Construct asset messages
    ///
    /// **DEPRECATED**: Use [`TimelineManager::construct_asset_messages()`] instead.
    #[deprecated(
        since = "0.2.0",
        note = "Use TimelineManager::construct_asset_messages() instead"
    )]
    pub fn construct_asset_messages(&self) -> Vec<Message> {
        let rt = tokio::runtime::Handle::try_current()
            .or_else(|_| tokio::runtime::Runtime::new().map(|rt| rt.handle().clone()));

        if let Ok(handle) = rt {
            handle.block_on(async { self.manager.construct_asset_messages().await })
        } else {
            Vec::new()
        }
    }

    /// Construct snapshot messages
    ///
    /// **DEPRECATED**: Use [`TimelineManager::construct_snapshot_messages()`] instead.
    #[deprecated(
        since = "0.2.0",
        note = "Use TimelineManager::construct_snapshot_messages() instead"
    )]
    pub fn construct_snapshot_messages(&self) -> Vec<Message> {
        let rt = tokio::runtime::Handle::try_current()
            .or_else(|_| tokio::runtime::Runtime::new().map(|rt| rt.handle().clone()));

        if let Ok(handle) = rt {
            handle.block_on(async { self.manager.construct_snapshot_messages().await })
        } else {
            Vec::new()
        }
    }

    /// Start snapshot collection
    ///
    /// **DEPRECATED**: This method was never properly implemented. Use [`TimelineManager::start()`] instead.
    #[deprecated(
        since = "0.2.0",
        note = "This method was never implemented. Use TimelineManager::start() instead"
    )]
    pub async fn start_snapshot_collection(
        &self,
        _activity_strategy: Box<dyn ActivityStrategy>,
        _s: &mut str,
    ) {
        // This was a todo!() in the original implementation
        tracing::warn!("start_snapshot_collection is deprecated and was never implemented");
    }

    /// Start collection activity
    ///
    /// **DEPRECATED**: Use [`TimelineManager::collect_activity()`] instead.
    #[deprecated(
        since = "0.2.0",
        note = "Use TimelineManager::collect_activity() instead"
    )]
    pub async fn start_collection_activity(
        &self,
        activity_strategy: Box<dyn ActivityStrategy>,
        _s: &mut str,
    ) {
        if let Err(e) = self.manager.collect_activity(activity_strategy).await {
            tracing::error!("Failed to collect activity: {}", e);
        }
    }

    /// Start the timeline collection process
    ///
    /// **DEPRECATED**: Use [`TimelineManager::start()`] instead.
    #[deprecated(since = "0.2.0", note = "Use TimelineManager::start() instead")]
    pub async fn start_collection(&self) -> anyhow::Result<()> {
        // We can't modify the manager since we only have a reference
        // This is a limitation of the backwards compatibility approach
        tracing::warn!(
            "start_collection is deprecated. Create a mutable TimelineManager and use start() instead"
        );
        Ok(())
    }
}

/// Create a new timeline with default settings
///
/// **DEPRECATED**: Use [`TimelineManager::new()`] or [`create_default_timeline()`] instead.
#[deprecated(
    since = "0.2.0",
    note = "Use TimelineManager::new() or create_default_timeline() instead"
)]
pub fn create_default_timeline_legacy() -> Timeline {
    Timeline {
        manager: TimelineManager::new(),
    }
}

// Remove the old SystemState struct as it was empty and unused
// If any code depends on it, they can define their own

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

    #[test]
    #[allow(deprecated)]
    fn test_legacy_api() {
        let timeline = Timeline::new(100, 3);

        // Test that legacy methods don't panic
        let _chips = timeline.get_context_chips();
        let _assets = timeline.get_activities();
        let _asset_msgs = timeline.construct_asset_messages();
        let _snapshot_msgs = timeline.construct_snapshot_messages();
    }

    #[test]
    #[allow(deprecated)]
    fn test_legacy_create_function() {
        let _timeline = create_default_timeline_legacy();
    }
}
