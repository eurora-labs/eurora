use crate::shared_types::{SharedPromptKitService, SharedTimeline};
use futures::StreamExt;
use tauri::ipc::Channel;
use tauri::{Manager, Runtime};
use tracing::info;

#[taurpc::procedures]
pub trait UserApi {
    async fn get_user<R: Runtime>(app_handle: tauri::AppHandle<R>) -> Result<String, String>;
}

#[derive(Clone)]
pub struct UserApiImpl;

#[taurpc::resolvers]
impl UserApi for UserApiImpl {
    async fn get_user<R: Runtime>(self, app_handle: tauri::AppHandle<R>) -> Result<String, String> {
        Ok("User not implemented".to_string())
    }
}
