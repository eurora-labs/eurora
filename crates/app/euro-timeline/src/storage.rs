use std::collections::VecDeque;

use chrono::Utc;
use tracing::debug;

use crate::{Activity, config::StorageConfig};

pub struct TimelineStorage {
    activities: VecDeque<Activity>,
    config: StorageConfig,
}

impl TimelineStorage {
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

    pub fn add_activity(&mut self, activity: Activity) {
        debug!(
            "Adding activity: {} (process: {})",
            activity.name, activity.process_name
        );

        self.activities.push_back(activity);

        while self.activities.len() > self.config.max_activities {
            if let Some(removed) = self.activities.pop_front() {
                debug!(
                    "Removed old activity due to capacity limit: {}",
                    removed.name
                );
            }
        }

        if self.config.auto_cleanup {
            self.cleanup_old_activities();
        }
    }

    pub fn get_current_activity(&self) -> Option<&Activity> {
        self.activities.back()
    }

    pub fn get_all_activities_mut(&mut self) -> &mut VecDeque<Activity> {
        &mut self.activities
    }

    fn cleanup_old_activities(&mut self) {
        let now = Utc::now();
        let cutoff_time = now
            - chrono::Duration::from_std(self.config.max_age)
                .unwrap_or_else(|_| chrono::Duration::seconds(3600));

        let initial_count = self.activities.len();

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
                break;
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
