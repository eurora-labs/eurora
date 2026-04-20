use euro_user::AuthManager;
use tauri::{AppHandle, Manager, Runtime};

use crate::shared_types::SharedUserController;

pub mod auth_procedures;
pub mod chat_procedures;
pub mod context_chip_procedures;
pub mod monitor_procedures;
pub mod onboarding_procedures;
pub mod payment_procedures;
pub mod prompt_procedures;
pub mod settings_procedures;
pub mod system_procedures;
pub mod third_party_procedures;
pub mod thread_procedures;
pub mod timeline_procedures;

pub(crate) fn user_controller<R: Runtime>(
    app_handle: &AppHandle<R>,
) -> Result<tauri::State<'_, SharedUserController>, String> {
    app_handle
        .try_state::<SharedUserController>()
        .ok_or_else(|| "User controller not available".to_string())
}

/// Briefly lock the shared `UserController`, clone out its `AuthManager`,
/// and return it. The clone is a cheap `Arc` bump; the lock is released
/// before the caller `.await`s, so concurrent requests don't serialize on
/// the outer mutex during network I/O.
pub(crate) async fn auth_manager<R: Runtime>(
    app_handle: &AppHandle<R>,
) -> Result<AuthManager, String> {
    let state = user_controller(app_handle)?;
    let controller = state.lock().await;
    Ok(controller.auth_manager.clone())
}
