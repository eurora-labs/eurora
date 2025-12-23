use euro_personal_db::Activity as DbActivity;

use crate::{TimelineError, TimelineManager, TimelineResult};

impl TimelineManager {
    pub async fn get_db_activity(&self) -> TimelineResult<DbActivity> {
        let storage = self.storage.lock().await;
        let activity = storage.get_current_activity();
        match activity {
            Some(activity) => Ok(DbActivity {
                id: activity.id.clone(),
                name: activity.name.clone(),
                icon_path: None,
                process_name: activity.process_name.clone(),
                started_at: activity.start,
                ended_at: activity.end,
            }),
            None => Err(TimelineError::Storage("No activity found".to_string())),
        }
    }
}
