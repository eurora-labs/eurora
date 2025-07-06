use tauri::{Manager, Runtime};
use tracing::{error, info};

#[taurpc::procedures(path = "user")]
pub trait UserApi {
    async fn set_launcher_hotkey<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        key: String,
        modifiers: Vec<String>,
    ) -> Result<(), String>;
}

#[derive(Clone)]
pub struct UserApiImpl;

#[taurpc::resolvers]
impl UserApi for UserApiImpl {
    async fn set_launcher_hotkey<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        key: String,
        modifiers: Vec<String>,
    ) -> Result<(), String> {
        if let Some(user_controller) = app_handle.try_state::<eur_user::Controller>() {
            if let Some(mut user) = user_controller.get_user().map_err(|e| e.to_string())? {
                user.hotkeys.open_launcher = eur_user::Hotkey {
                    key,
                    modifiers,
                    function: eur_user::HotkeyFunction::OpenLauncher,
                };
                user_controller.set_user(&user).map_err(|e| e.to_string())?;
            }
            info!("Launcher hotkey set successfully");
            Ok(())
        } else {
            error!("User controller is not available");
            Err("User controller is not available".to_string())
        }
    }
}
