use std::sync::Arc;

use crate::shared_types::{SharedSettingsState, SharedThreadManager};
use euro_endpoint::EndpointManager;
use euro_settings::SettingsState;
use euro_telemetry::Controller as TelemetryController;
use euro_thread::commands::{NoopChatContextProvider, SharedChatContextProvider};
use tauri::Manager;

pub fn init(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = app.path().app_data_dir()?;

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

    init_state(app, &endpoint_manager, &data_dir)?;

    Ok(())
}

fn init_state(
    app: &tauri::App,
    endpoint_manager: &std::sync::Arc<EndpointManager>,
    data_dir: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let app_handle = app.handle();

    // Single shared AuthManager so concurrent refreshes coalesce through
    // one refresh lock, regardless of which consumer initiates the
    // refresh. `install` registers it as Tauri-managed state and spawns
    // the bus → frontend bridge that emits `AuthStateChanged` on every
    // transition.
    let auth_manager = euro_auth::AuthManager::new(endpoint_manager.clone(), data_dir)?;
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
