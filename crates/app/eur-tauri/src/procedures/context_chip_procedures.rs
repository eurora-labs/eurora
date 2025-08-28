use eur_activity::ContextChip;
use eur_timeline::TimelineManager;
use tauri::{Manager, Runtime};

#[taurpc::procedures(path = "context_chip")]
pub trait ContextChipApi {
    async fn get<R: Runtime>(app_handle: tauri::AppHandle<R>) -> Result<Vec<ContextChip>, String>;
}

#[derive(Clone)]
pub struct ContextChipApiImpl;

#[taurpc::resolvers]
impl ContextChipApi for ContextChipApiImpl {
    async fn get<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<Vec<ContextChip>, String> {
        let timeline_state: tauri::State<async_mutex::Mutex<TimelineManager>> = app_handle.state();
        let timeline = timeline_state.lock().await;

        // Get all activities from the timeline
        let activities = timeline.get_context_chips().await;

        // Sort activities by start time (most recent first)
        // activities.sort_by(|a, b| b.start.cmp(&a.start));

        // Limit to the 5 most recent activities to avoid cluttering the UI
        let limited_activities = activities.into_iter().take(5).collect();

        Ok(limited_activities)
    }
}
