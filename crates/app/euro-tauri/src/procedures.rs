use euro_user::AuthManager;
use tauri::{AppHandle, Manager, Runtime};

use crate::shared_types::SharedUserController;

pub mod auth_procedures;
pub mod payment_procedures;
pub mod settings_procedures;
pub mod system_procedures;
pub mod thread_procedures;
pub mod timeline_procedures;

/// Look up the shared [`SharedUserController`] state. Returns `None` if it
/// was not registered — after the Phase-5 startup reorder this only
/// happens during shutdown, so call sites should map `None` to a typed
/// "state unavailable" error rather than retry.
pub(crate) fn user_controller<R: Runtime>(
    app_handle: &AppHandle<R>,
) -> Option<tauri::State<'_, SharedUserController>> {
    app_handle.try_state::<SharedUserController>()
}

/// Briefly lock the shared `UserController`, clone out its `AuthManager`,
/// and return it. The clone is a cheap `Arc` bump; the lock is released
/// before the caller `.await`s, so concurrent requests don't serialize on
/// the outer mutex during network I/O.
pub(crate) async fn auth_manager<R: Runtime>(app_handle: &AppHandle<R>) -> Option<AuthManager> {
    let state = user_controller(app_handle)?;
    let controller = state.lock().await;
    Some(controller.auth_manager.clone())
}
