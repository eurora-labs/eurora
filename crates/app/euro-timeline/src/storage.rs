use std::collections::VecDeque;

use chrono::Utc;

use crate::{ActivitySession, config::StorageConfig};

pub struct TimelineStorage {
    sessions: VecDeque<ActivitySession>,
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
            sessions: VecDeque::with_capacity(config.max_activities),
            config,
        }
    }

    pub fn add_session(&mut self, session: ActivitySession) {
        tracing::debug!(
            "Adding session: {} (process: {})",
            session.activity.key,
            session.process_name
        );

        self.sessions.push_back(session);

        while self.sessions.len() > self.config.max_activities {
            if let Some(removed) = self.sessions.pop_front() {
                tracing::debug!(
                    "Removed old session due to capacity limit: {}",
                    removed.activity.key
                );
            }
        }

        if self.config.auto_cleanup {
            self.cleanup_old_sessions();
        }
    }

    pub fn get_current_session(&self) -> Option<&ActivitySession> {
        self.sessions.back()
    }

    pub fn get_all_sessions_mut(&mut self) -> &mut VecDeque<ActivitySession> {
        &mut self.sessions
    }

    fn cleanup_old_sessions(&mut self) {
        let now = Utc::now();
        let cutoff_time = now
            - chrono::Duration::from_std(self.config.max_age)
                .unwrap_or_else(|_| chrono::Duration::seconds(3600));

        let initial_count = self.sessions.len();

        while let Some(session) = self.sessions.front() {
            if session.started_at < cutoff_time {
                if let Some(removed) = self.sessions.pop_front() {
                    tracing::debug!(
                        "Cleaned up old session: {} (age: {:?})",
                        removed.activity.key,
                        now - removed.started_at
                    );
                }
            } else {
                break;
            }
        }

        let removed_count = initial_count - self.sessions.len();
        if removed_count > 0 {
            tracing::debug!("Cleaned up {} old sessions", removed_count);
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
    use euro_activity::ActivitySession;

    fn fresh_storage() -> TimelineStorage {
        TimelineStorage::new(StorageConfig {
            max_activities: 10,
            max_age: std::time::Duration::from_secs(3600),
            auto_cleanup: false,
        })
    }

    fn fresh_session(key: &str) -> ActivitySession {
        ActivitySession::new_process(key.to_string(), 1, None, None)
    }

    /// The collector's `NewActivity` branch must end the previous
    /// session locally before pushing the new one, so chat-context
    /// reads and the closing PATCH agree on the row's lifetime.
    #[test]
    fn ending_previous_then_adding_new_records_end_on_previous_only() {
        let mut storage = fresh_storage();
        let first = fresh_session("first");
        let first_id = first.id;
        storage.add_session(first);

        if let Some(prev) = storage.get_all_sessions_mut().back_mut() {
            prev.end_session();
        }
        let second = fresh_session("second");
        let second_id = second.id;
        storage.add_session(second);

        let sessions = storage.get_all_sessions_mut();
        let first_back = sessions
            .iter()
            .find(|s| s.id == first_id)
            .expect("first session retained");
        let second_back = sessions
            .iter()
            .find(|s| s.id == second_id)
            .expect("second session retained");

        assert!(
            first_back.ended_at.is_some(),
            "previous session must be ended"
        );
        assert!(
            second_back.ended_at.is_none(),
            "newly-added session has no end yet"
        );
        assert_eq!(storage.get_current_session().map(|s| s.id), Some(second_id));
    }

    /// `end_session` is idempotent — a second call leaves the existing
    /// timestamp untouched so a `Stopping` after `NewActivity` can't
    /// accidentally extend the row's `ended_at`.
    #[test]
    fn end_session_is_idempotent() {
        let mut session = fresh_session("once");
        session.end_session();
        let first = session.ended_at;
        std::thread::sleep(std::time::Duration::from_millis(2));
        session.end_session();
        assert_eq!(session.ended_at, first);
    }
}
