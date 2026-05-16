use std::sync::Arc;

use crate::shared_types::{SharedSettingsState, SharedThreadManager};
use euro_endpoint::EndpointManager;
use euro_settings::SettingsState;
use euro_telemetry::Controller as TelemetryController;
use euro_thread::commands::{NoopChatContextProvider, SharedChatContextProvider};
use tauri::Manager;

pub fn init(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = app.path().app_data_dir()?;
    init_encryption(data_dir)?;

    let config_dir = app.path().app_config_dir()?;
    std::fs::create_dir_all(&config_dir)?;
    let settings = SettingsState::load_or_migrate(&config_dir)?;

    // Initialize Sentry from the loaded settings. Mobile can't peek
    // before the Tauri builder runs (the config dir resolves through
    // `app.path()`, which only exists after `setup`), so unlike the
    // desktop the panic-capture window only opens here. Held as
    // `Arc<TelemetryController>` so the `system_reinit_telemetry`
    // procedure can swap the underlying client when consent changes.
    let telemetry_controller = Arc::new(TelemetryController::init(
        settings
            .cache
            .settings
            .desktop
            .telemetry
            .allows_errors_on_desktop(),
        settings.local.telemetry.distinct_id.as_deref(),
    ));

    // The persisted ConnectionMode always resolves to a non-empty URL, so
    // we never need the env-fallback path.
    let endpoint_url = settings.local.api.endpoint();
    let endpoint_manager = std::sync::Arc::new(EndpointManager::new(endpoint_url)?);

    app.manage(endpoint_manager.clone());
    app.manage(SharedSettingsState::new(settings));
    app.manage(telemetry_controller);

    init_state(app, &endpoint_manager)?;

    Ok(())
}

fn init_encryption(data_dir: std::path::PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(debug_assertions)]
    let main_key = euro_secret::MainKey::from_bytes([
        0xA4, 0x1B, 0x7E, 0x3C, 0x92, 0xF0, 0x55, 0xD8, 0x6A, 0xC3, 0x11, 0xBF, 0x48, 0xE7, 0x2D,
        0x9F, 0x03, 0x86, 0xFA, 0x74, 0xCB, 0x60, 0x1D, 0xA5, 0x39, 0xEE, 0x57, 0x0C, 0xB2, 0x84,
        0x63, 0xD1,
    ]);
    #[cfg(not(debug_assertions))]
    let main_key = euro_secret::MainKey::new()?;

    euro_secret::secret::init_file_store(*main_key.as_bytes(), data_dir)?;
    Ok(())
}

fn init_state(
    app: &tauri::App,
    endpoint_manager: &std::sync::Arc<EndpointManager>,
) -> Result<(), Box<dyn std::error::Error>> {
    let app_handle = app.handle();

    // Single shared AuthManager so concurrent refreshes coalesce through
    // one refresh lock, regardless of which consumer initiates the
    // refresh. `install` registers it as Tauri-managed state and spawns
    // the bus → frontend bridge that emits `AuthStateChanged` on every
    // transition.
    let auth_manager = euro_auth::AuthManager::new(endpoint_manager.clone());
    euro_auth::tauri::install(app_handle, auth_manager.clone());

    let thread_manager = euro_thread::ThreadManager::new(endpoint_manager.clone(), auth_manager);
    app_handle.manage(SharedThreadManager::new(thread_manager));

    app_handle.manage(crate::shared_types::ActiveStreamTokens::default());

    // Mobile has no timeline — `chat_collect_context` returns an empty
    // [`ChatContext`] until per-thread native-picker state replaces this
    // with a real provider.
    let context_provider: SharedChatContextProvider = std::sync::Arc::new(NoopChatContextProvider);
    app_handle.manage(context_provider);

    Ok(())
}
