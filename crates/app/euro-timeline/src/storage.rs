//! Timeline storage implementation

use std::collections::VecDeque;

use chrono::Utc;
use log::debug;

use crate::{Activity, config::StorageConfig};

/// Timeline storage that manages activities with configurable retention
pub struct TimelineStorage {
    /// Activities stored in chronological order (oldest first)
    activities: VecDeque<Activity>,
    /// Storage configuration
    config: StorageConfig,
}

impl TimelineStorage {
    /// Create a new timeline storage with the given configuration
    pub fn new(config: StorageConfig) -> Self {
        debug!(
            "Creating timeline storage with max_activities: {}, max_age: {:?}",
            config.max_activities, config.max_age
        );

        Self {
            activities: VecDeque::with_capacity(config.max_activities),
            config,
        }
    }

    /// Add a new activity to the timeline
    pub fn add_activity(&mut self, activity: Activity) {
        debug!(
            "Adding activity: {} (process: {})",
            activity.name, activity.process_name
        );

        // Add the new activity
        self.activities.push_back(activity);

        // Enforce capacity limit
        while self.activities.len() > self.config.max_activities {
            if let Some(removed) = self.activities.pop_front() {
                debug!(
                    "Removed old activity due to capacity limit: {}",
                    removed.name
                );
            }
        }

        // Auto cleanup if enabled
        if self.config.auto_cleanup {
            self.cleanup_old_activities();
        }
    }

    /// Get the current (most recent) activity
    pub fn get_current_activity(&self) -> Option<&Activity> {
        self.activities.back()
    }

    /// Get all activities mutably (oldest first)
    pub fn get_all_activities_mut(&mut self) -> &mut VecDeque<Activity> {
        &mut self.activities
    }

    /// Cleanup old activities based on max_age configuration
    fn cleanup_old_activities(&mut self) {
        let now = Utc::now();
        let cutoff_time = now
            - chrono::Duration::from_std(self.config.max_age)
                .unwrap_or_else(|_| chrono::Duration::seconds(3600));

        let initial_count = self.activities.len();

        // Remove activities older than cutoff_time
        while let Some(activity) = self.activities.front() {
            if activity.start < cutoff_time {
                if let Some(removed) = self.activities.pop_front() {
                    debug!(
                        "Cleaned up old activity: {} (age: {:?})",
                        removed.name,
                        now - removed.start
                    );
                }
            } else {
                break; // Activities are in chronological order
            }
        }

        let removed_count = initial_count - self.activities.len();
        if removed_count > 0 {
            debug!("Cleaned up {} old activities", removed_count);
        }
    }
}

impl Default for TimelineStorage {
    fn default() -> Self {
        Self::new(StorageConfig::default())
    }
}
