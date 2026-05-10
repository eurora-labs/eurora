use euro_user::AuthManager;
use tauri::{AppHandle, Manager};

use crate::shared_types::SharedUserController;

pub mod auth_procedures;
pub mod settings_procedures;
pub mod system_procedures;

pub(crate) fn user_controller(
    app_handle: &AppHandle,
) -> Result<tauri::State<'_, SharedUserController>, String> {
    app_handle
        .try_state::<SharedUserController>()
        .ok_or_else(|| "User controller not available".to_string())
}

/// Briefly lock the shared `UserController`, clone out its `AuthManager`,
/// and return it. The clone is a cheap `Arc` bump; the lock is released
/// before the caller `.await`s, so concurrent requests don't serialize on
/// the outer mutex during network I/O.
pub(crate) async fn auth_manager(app_handle: &AppHandle) -> Result<AuthManager, String> {
    let state = user_controller(app_handle)?;
    let controller = state.lock().await;
    Ok(controller.auth_manager.clone())
}
