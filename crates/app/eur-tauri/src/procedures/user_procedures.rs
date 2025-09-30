use tauri::{Manager, Runtime};
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use tracing::{debug, error};

use crate::util::*;

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
        if key.trim().is_empty() {
            return Err("Key cannot be empty".to_string());
        }

        let valid_modifiers = ["ctrl", "alt", "shift", "meta", "cmd", "super"];
        for modifier in &modifiers {
            if !valid_modifiers.contains(&modifier.to_lowercase().as_str()) {
                return Err(format!("Invalid modifier: {}", modifier));
            }
        }

        // Create the new hotkey to validate it can be converted to a Tauri shortcut
        let new_hotkey = eur_user::Hotkey {
            key: key.clone(),
            modifiers: modifiers.clone(),
            function: eur_user::HotkeyFunction::OpenLauncher,
        };

        let new_shortcut = user_hotkey_to_shortcut(&new_hotkey)
            .ok_or_else(|| "Invalid key or modifier combination".to_string())?;

        if let Some(user_controller) = app_handle.try_state::<eur_user::Controller>() {
            // Get the current user to check for existing hotkey
            let mut user = user_controller
                .get_or_create_user()
                .map_err(|e| e.to_string())?;

            // Always try to unregister any existing shortcuts to avoid conflicts
            // First, try to unregister the current user's custom shortcut if it exists
            if !user.hotkeys.open_launcher.key.is_empty()
                && let Some(old_shortcut) = user_hotkey_to_shortcut(&user.hotkeys.open_launcher)
            {
                match app_handle.global_shortcut().unregister(old_shortcut) {
                    Ok(_) => debug!(
                        "Successfully unregistered old custom shortcut: {:?}",
                        old_shortcut
                    ),
                    Err(e) => error!("Failed to unregister old custom shortcut: {}", e),
                }
            }

            // Also try to unregister the default shortcut in case it's still registered
            let default_shortcut = crate::util::get_default_shortcut();
            match app_handle.global_shortcut().unregister(default_shortcut) {
                Ok(_) => debug!(
                    "Successfully unregistered default shortcut: {:?}",
                    default_shortcut
                ),
                Err(e) => {
                    // This is expected if the default shortcut wasn't registered or was already replaced
                    debug!(
                        "Default shortcut was not registered or already unregistered: {}",
                        e
                    );
                }
            }

            // Register the new shortcut
            app_handle
                .global_shortcut()
                .register(new_shortcut)
                .map_err(|e| format!("Failed to register new shortcut: {}", e))?;

            // Update the user's hotkey
            user.hotkeys.open_launcher = new_hotkey;
            user_controller.set_user(&user).map_err(|e| e.to_string())?;

            debug!(
                "Launcher hotkey updated successfully to: {:?}",
                new_shortcut
            );
            Ok(())
        } else {
            error!("User controller is not available");
            Err("User controller is not available".to_string())
        }
    }
}
