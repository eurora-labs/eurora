use crate::{TimelineError, TimelineManager, TimelineResult};
use eur_personal_db::Activity as DbActivity;

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
                started_at: activity.start.to_string(),
                ended_at: None,
            }),
            None => Err(TimelineError::Storage("No activity found".to_string())),
        }
    }
}
