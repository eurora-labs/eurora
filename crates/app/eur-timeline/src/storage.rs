//! Timeline storage implementation

use std::{collections::VecDeque, time::Duration};

use chrono::{DateTime, Utc};
use tracing::debug;

use crate::{Activity, config::StorageConfig, error::TimelineResult};

/// Timeline storage that manages activities with configurable retention
pub struct TimelineStorage {
    /// Activities stored in chronological order (oldest first)
    activities: VecDeque<Activity>,
    ///
    /// Storage configuration
    config: StorageConfig,
    /// Last cleanup time
    last_cleanup: DateTime<Utc>,
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
            last_cleanup: Utc::now(),
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

    /// Get the most recent activities (up to count)
    pub fn get_recent_activities(&self, count: usize) -> Vec<&Activity> {
        self.activities
            .iter()
            .rev() // Most recent first
            .take(count)
            .collect()
    }

    /// Get all activities since a specific time
    pub fn get_activities_since(&self, since: DateTime<Utc>) -> Vec<&Activity> {
        self.activities
            .iter()
            .filter(|activity| activity.start >= since)
            .collect()
    }

    /// Get the current (most recent) activity
    pub fn get_current_activity(&self) -> Option<&Activity> {
        self.activities.back()
    }

    /// Get all activities (oldest first)
    pub fn get_all_activities(&self) -> &VecDeque<Activity> {
        &self.activities
    }

    /// Get all activities mutably (oldest first)
    pub fn get_all_activities_mut(&mut self) -> &mut VecDeque<Activity> {
        &mut self.activities
    }

    /// Get the number of activities stored
    pub fn len(&self) -> usize {
        self.activities.len()
    }

    /// Check if the storage is empty
    pub fn is_empty(&self) -> bool {
        self.activities.is_empty()
    }

    /// Cleanup old activities based on max_age configuration
    pub fn cleanup_old_activities(&mut self) {
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

        self.last_cleanup = now;
    }

    /// Force cleanup regardless of auto_cleanup setting
    pub fn force_cleanup(&mut self) {
        self.cleanup_old_activities();
    }

    /// Clear all activities
    pub fn clear(&mut self) {
        let count = self.activities.len();
        self.activities.clear();
        debug!("Cleared {} activities from storage", count);
    }

    /// Get storage statistics
    pub fn get_stats(&self) -> StorageStats {
        let now = Utc::now();
        let oldest_activity_age = self.activities.front().map(|activity| now - activity.start);
        let newest_activity_age = self.activities.back().map(|activity| now - activity.start);

        StorageStats {
            total_activities: self.activities.len(),
            capacity: self.config.max_activities,
            capacity_used_percent: (self.activities.len() as f64
                / self.config.max_activities as f64
                * 100.0) as u8,
            oldest_activity_age,
            newest_activity_age,
            last_cleanup: self.last_cleanup,
        }
    }

    /// Update storage configuration
    pub fn update_config(&mut self, config: StorageConfig) -> TimelineResult<()> {
        debug!("Updating storage configuration");

        // If capacity decreased, remove excess activities
        if config.max_activities < self.config.max_activities {
            while self.activities.len() > config.max_activities {
                if let Some(removed) = self.activities.pop_front() {
                    debug!(
                        "Removed activity due to capacity reduction: {}",
                        removed.name
                    );
                }
            }
        }

        self.config = config;

        // Force cleanup with new configuration
        if self.config.auto_cleanup {
            self.cleanup_old_activities();
        }

        Ok(())
    }

    /// Check if cleanup is needed based on time since last cleanup
    pub fn needs_cleanup(&self) -> bool {
        if !self.config.auto_cleanup {
            return false;
        }

        let cleanup_interval = Duration::from_secs(300); // 5 minutes
        Utc::now() - self.last_cleanup
            > chrono::Duration::from_std(cleanup_interval)
                .unwrap_or_else(|_| chrono::Duration::minutes(5))
    }
}

/// Statistics about the timeline storage
#[derive(Debug, Clone)]
pub struct StorageStats {
    /// Total number of activities stored
    pub total_activities: usize,
    /// Maximum capacity
    pub capacity: usize,
    /// Percentage of capacity used
    pub capacity_used_percent: u8,
    /// Age of the oldest activity
    pub oldest_activity_age: Option<chrono::Duration>,
    /// Age of the newest activity
    pub newest_activity_age: Option<chrono::Duration>,
    /// When cleanup was last performed
    pub last_cleanup: DateTime<Utc>,
}

impl Default for TimelineStorage {
    fn default() -> Self {
        Self::new(StorageConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration as StdDuration;

    use super::*;

    fn create_test_activity(name: &str) -> Activity {
        Activity::new(
            name.to_string(),
            "test_icon".to_string(),
            "test_process".to_string(),
            vec![],
        )
    }

    #[test]
    fn test_storage_creation() {
        let config = StorageConfig {
            max_activities: 100,
            max_age: StdDuration::from_secs(3600),
            auto_cleanup: true,
        };

        let storage = TimelineStorage::new(config);
        assert_eq!(storage.len(), 0);
        assert!(storage.is_empty());
    }

    #[test]
    fn test_add_activity() {
        let mut storage = TimelineStorage::default();
        let activity = create_test_activity("Test Activity");

        storage.add_activity(activity);
        assert_eq!(storage.len(), 1);
        assert!(!storage.is_empty());

        let current = storage.get_current_activity().unwrap();
        assert_eq!(current.name, "Test Activity");
    }

    #[test]
    fn test_capacity_limit() {
        let config = StorageConfig {
            max_activities: 2,
            max_age: StdDuration::from_secs(3600),
            auto_cleanup: false,
        };

        let mut storage = TimelineStorage::new(config);

        storage.add_activity(create_test_activity("Activity 1"));
        storage.add_activity(create_test_activity("Activity 2"));
        storage.add_activity(create_test_activity("Activity 3"));

        assert_eq!(storage.len(), 2);

        // Should have removed the oldest activity
        let activities = storage.get_recent_activities(2);
        assert_eq!(activities[0].name, "Activity 3"); // Most recent first
        assert_eq!(activities[1].name, "Activity 2");
    }

    #[test]
    fn test_get_recent_activities() {
        let mut storage = TimelineStorage::default();

        for i in 1..=5 {
            storage.add_activity(create_test_activity(&format!("Activity {}", i)));
        }

        let recent = storage.get_recent_activities(3);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].name, "Activity 5"); // Most recent first
        assert_eq!(recent[1].name, "Activity 4");
        assert_eq!(recent[2].name, "Activity 3");
    }

    #[test]
    fn test_clear() {
        let mut storage = TimelineStorage::default();

        storage.add_activity(create_test_activity("Activity 1"));
        storage.add_activity(create_test_activity("Activity 2"));

        assert_eq!(storage.len(), 2);

        storage.clear();
        assert_eq!(storage.len(), 0);
        assert!(storage.is_empty());
    }

    #[test]
    fn test_storage_stats() {
        let mut storage = TimelineStorage::default();
        storage.add_activity(create_test_activity("Test Activity"));

        let stats = storage.get_stats();
        assert_eq!(stats.total_activities, 1);
        assert_eq!(stats.capacity, 1000); // Default capacity
        assert!(stats.newest_activity_age.is_some());
    }
}
