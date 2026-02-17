use euro_activity::ContextChip;
use euro_timeline::TimelineManager;
use tauri::{Manager, Runtime};
use tokio::sync::Mutex;

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
        let timeline_state: tauri::State<Mutex<TimelineManager>> = app_handle.state();
        let timeline = timeline_state.lock().await;

        let activities = timeline.get_context_chips().await;
        let limited_activities = activities.into_iter().take(5).collect();

        Ok(limited_activities)
    }
}
