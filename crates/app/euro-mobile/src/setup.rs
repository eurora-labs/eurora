use crate::shared_types::{SharedAppSettings, SharedThreadManager, SharedUserController};
use euro_endpoint::EndpointManager;
use euro_settings::AppSettings;
use tauri::Manager;

pub fn init(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = app.path().app_data_dir()?;
    init_encryption(data_dir)?;

    let config_dir = app.path().app_config_dir()?;
    std::fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("settings.json");
    let app_settings = match AppSettings::load(&config_path) {
        Ok(settings) => settings,
        Err(e) => {
            tracing::warn!("Failed to load settings, resetting to defaults: {e}");
            AppSettings::defaults()
        }
    };
    let endpoint_url = &app_settings.api.endpoint;
    let endpoint_manager = if endpoint_url.is_empty() {
        EndpointManager::from_env()
    } else {
        EndpointManager::new(endpoint_url)
    }?;
    let endpoint_manager = std::sync::Arc::new(endpoint_manager);

    app.manage(endpoint_manager.clone());
    app.manage(SharedAppSettings::new(app_settings));

    init_state(app, &endpoint_manager)?;

    Ok(())
}

fn init_encryption(data_dir: std::path::PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(debug_assertions)]
    let main_key = euro_encrypt::MainKey::from_bytes([
        0xA4, 0x1B, 0x7E, 0x3C, 0x92, 0xF0, 0x55, 0xD8, 0x6A, 0xC3, 0x11, 0xBF, 0x48, 0xE7, 0x2D,
        0x9F, 0x03, 0x86, 0xFA, 0x74, 0xCB, 0x60, 0x1D, 0xA5, 0x39, 0xEE, 0x57, 0x0C, 0xB2, 0x84,
        0x63, 0xD1,
    ]);
    #[cfg(not(debug_assertions))]
    let main_key = euro_encrypt::MainKey::new()?;

    euro_secret::secret::init_file_store(*main_key.as_bytes(), data_dir)?;
    Ok(())
}

fn init_state(
    app: &tauri::App,
    endpoint_manager: &std::sync::Arc<EndpointManager>,
) -> Result<(), Box<dyn std::error::Error>> {
    let app_handle = app.handle();

    // Single shared AuthManager so concurrent refreshes coalesce through one
    // refresh lock, regardless of which consumer initiates the refresh.
    let auth_manager = euro_auth::AuthManager::new(endpoint_manager.subscribe());

    let thread_manager =
        euro_thread::ThreadManager::new(endpoint_manager.subscribe(), auth_manager.clone());
    app_handle.manage(SharedThreadManager::new(thread_manager));

    let path = app.path().app_data_dir()?;
    let user_controller = euro_user::UserController::new(path, auth_manager);
    app_handle.manage(SharedUserController::new(user_controller));

    app_handle.manage(crate::shared_types::ActiveStreamTokens::default());

    Ok(())
}
