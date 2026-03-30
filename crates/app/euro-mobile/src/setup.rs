use euro_endpoint::EndpointManager;
use euro_settings::AppSettings;
use tauri::Manager;
use tokio::sync::Mutex;

pub fn init(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = app.path().app_data_dir()?;
    init_encryption(data_dir)?;

    let app_settings = AppSettings::load_from_default_path_creating()?;
    let endpoint_url = &app_settings.api.endpoint;
    let endpoint_manager = if endpoint_url.is_empty() {
        EndpointManager::from_env()
    } else {
        EndpointManager::new(endpoint_url)
    }?;
    let endpoint_manager = std::sync::Arc::new(endpoint_manager);

    app.manage(endpoint_manager.clone());
    app.manage(Mutex::new(app_settings));

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

    let thread_channel_rx = endpoint_manager.subscribe();
    let thread_manager = euro_thread::ThreadManager::new(thread_channel_rx);
    app_handle.manage(std::sync::Arc::new(Mutex::new(thread_manager)));

    let path = app.path().app_data_dir()?;
    let user_channel_rx = endpoint_manager.subscribe();
    let user_controller = euro_user::Controller::new(path, user_channel_rx)?;
    app_handle.manage(std::sync::Arc::new(Mutex::new(user_controller)));

    Ok(())
}
