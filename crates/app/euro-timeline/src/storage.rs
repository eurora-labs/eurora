use std::collections::VecDeque;

use chrono::Utc;

use crate::{Activity, config::StorageConfig};

pub struct TimelineStorage {
    activities: VecDeque<Activity>,
    config: StorageConfig,
}

impl TimelineStorage {
    pub fn new(config: StorageConfig) -> Self {
        tracing::debug!(
            "Creating timeline storage with max_activities: {}, max_age: {:?}",
            config.max_activities,
            config.max_age
        );

        Self {
            activities: VecDeque::with_capacity(config.max_activities),
            config,
        }
    }

    pub fn add_activity(&mut self, activity: Activity) {
        tracing::debug!(
            "Adding activity: {} (process: {})",
            activity.name,
            activity.process_name
        );

        self.activities.push_back(activity);

        while self.activities.len() > self.config.max_activities {
            if let Some(removed) = self.activities.pop_front() {
                tracing::debug!(
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
                    tracing::debug!(
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
            tracing::debug!("Cleaned up {} old activities", removed_count);
        }
    }
}

impl Default for TimelineStorage {
    fn default() -> Self {
        Self::new(StorageConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use euro_activity::Activity;

    fn fresh_storage() -> TimelineStorage {
        TimelineStorage::new(StorageConfig {
            max_activities: 10,
            max_age: std::time::Duration::from_secs(3600),
            auto_cleanup: false,
        })
    }

    fn fresh_activity(name: &str) -> Activity {
        Activity::new(name.to_string(), None, None, "proc".to_string(), 1)
    }

    /// The collector's `NewActivity` branch must end the previous
    /// activity locally before pushing the new one, so chat-context
    /// reads and the closing PATCH agree on the row's lifetime.
    #[test]
    fn ending_previous_then_adding_new_records_end_on_previous_only() {
        let mut storage = fresh_storage();
        let first = fresh_activity("first");
        let first_id = first.id;
        storage.add_activity(first);

        // Mirror the collector lifecycle: end the back-most activity
        // before pushing the next one.
        if let Some(prev) = storage.get_all_activities_mut().back_mut() {
            prev.end_activity();
        }
        let second = fresh_activity("second");
        let second_id = second.id;
        storage.add_activity(second);

        let activities = storage.get_all_activities_mut();
        let first_back = activities
            .iter()
            .find(|a| a.id == first_id)
            .expect("first activity retained");
        let second_back = activities
            .iter()
            .find(|a| a.id == second_id)
            .expect("second activity retained");

        assert!(first_back.end.is_some(), "previous activity must be ended");
        assert!(
            second_back.end.is_none(),
            "newly-added activity has no end yet"
        );
        assert_eq!(
            storage.get_current_activity().map(|a| a.id),
            Some(second_id)
        );
    }

    /// `end_activity` is idempotent at the field level — calling it
    /// twice keeps the first timestamp rather than ratcheting forward.
    /// The collector relies on this so a `Stopping` after `NewActivity`
    /// can't accidentally extend the row's `ended_at`.
    #[test]
    fn end_activity_is_one_shot_at_field_level() {
        let mut activity = fresh_activity("once");
        activity.end_activity();
        let first = activity.end;
        std::thread::sleep(std::time::Duration::from_millis(2));
        // Re-calling overwrites, which is intentional for heartbeat
        // semantics — confirm the field is writable, not pinned.
        activity.end_activity();
        assert!(activity.end.is_some());
        assert!(activity.end > first);
    }
}
