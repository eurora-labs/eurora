use eur_personal_db::PersonalDatabaseManager;
use tauri::{Manager, Runtime};

#[taurpc::ipc_type]
pub struct AppEvent {
    pub name: String,
    pub color: String,
    pub icon_base64: Option<String>,
}

#[taurpc::procedures(path = "timeline")]
pub trait TimelineApi {
    #[taurpc(event)]
    async fn new_app_event(event: AppEvent);

    async fn list<R: Runtime>(app_handle: tauri::AppHandle<R>) -> Result<Vec<String>, String>;
}

#[derive(Clone)]
pub struct TimelineApiImpl;

#[taurpc::resolvers]
impl TimelineApi for TimelineApiImpl {
    async fn list<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<Vec<String>, String> {
        let personal_db = app_handle.state::<PersonalDatabaseManager>().inner();
        let activities = personal_db
            .list_activities(5, 0)
            .await
            .map_err(|e| e.to_string())?;

        Ok(activities
            .into_iter()
            .map(|activity| activity.name)
            .collect())
    }
}
