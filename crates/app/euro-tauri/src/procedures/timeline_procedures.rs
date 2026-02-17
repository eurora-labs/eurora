use euro_activity::ContextChip;
use tauri::Runtime;

#[taurpc::ipc_type]
pub struct TimelineAppEvent {
    pub name: String,
    pub color: Option<String>,
    pub icon_base64: Option<String>,
}

#[taurpc::procedures(path = "timeline")]
pub trait TimelineApi {
    #[taurpc(event)]
    async fn new_app_event(event: TimelineAppEvent);

    #[taurpc(event)]
    async fn new_assets_event(chips: Vec<ContextChip>);

    async fn list<R: Runtime>(app_handle: tauri::AppHandle<R>) -> Result<Vec<String>, String>;
}

#[derive(Clone)]
pub struct TimelineApiImpl;

#[taurpc::resolvers]
impl TimelineApi for TimelineApiImpl {
    async fn list<R: Runtime>(
        self,
        _app_handle: tauri::AppHandle<R>,
    ) -> Result<Vec<String>, String> {
        Ok(vec![])
    }
}
