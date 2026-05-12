#![cfg_attr(
    all(windows, not(test), not(debug_assertions)),
    windows_subsystem = "windows"
)]

use euro_endpoint::EndpointManager;
use euro_settings::{CloudSettingsCache, SettingsState, wants_errors};
use euro_tauri::chat_context::TimelineChatContextProvider;
use euro_tauri::shared_types::SharedUserController;
use euro_tauri::{
    MAIN_WINDOW_LABEL, WindowState, build_specta, create_window,
    procedures::{
        system_procedures::{
            BrowserExtensionStatusChanged, SAFARI_BRIDGE_APP_KIND, resolve_browser_extension_state,
        },
        timeline_procedures::{AccentColor, TimelineAppEvent, TimelineAssetsEvent},
    },
    shared_types::{ActiveStreamTokens, SharedHttpClient, SharedThreadManager},
    show_and_focus_main,
};
use euro_telemetry::{Controller as TelemetryController, sentry_tracing};
use euro_thread::commands::SharedChatContextProvider;
use euro_timeline::TimelineManager;
use tauri::{
    Manager, generate_context,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};
use tauri_specta::Event;
use tokio::sync::Mutex;

/// Returns `true` when the on-disk messenger binary was replaced during
/// this call. Callers use that signal to open the bridge's browser-purge
/// window so stale messengers from the previous session are cleared out
/// when they reconnect.
fn install_native_messaging_manifests(app: &tauri::App) -> bool {
    #[cfg(target_os = "windows")]
    {
        let _ = app;
        false
    }

    #[cfg(not(target_os = "windows"))]
    {
        use std::path::PathBuf;

        let resource_dir = match app.path().resource_dir() {
            Ok(dir) => dir,
            Err(e) => {
                tracing::warn!(
                    "Could not resolve resource dir, skipping native messaging manifest install: {e}"
                );
                return false;
            }
        };

        let binary_path = match std::env::current_exe() {
            Ok(exe) => match exe.parent() {
                Some(dir) => dir.join("euro-native-messaging"),
                None => {
                    tracing::warn!("Could not resolve parent directory of current exe");
                    return false;
                }
            },

            Err(e) => {
                tracing::warn!("Could not resolve current exe path: {e}");
                return false;
            }
        };

        #[cfg(target_os = "macos")]
        let manifest_binary_path = binary_path.to_string_lossy().to_string();
        #[cfg(target_os = "macos")]
        let messenger_replaced = false;

        #[cfg(target_os = "linux")]
        let (manifest_binary_path, messenger_replaced) = {
            use euro_tauri::native_messaging::{InstallOutcome, install_messenger_binary};

            let home = dirs::home_dir().unwrap_or_default();
            let dest = home.join(".eurora/native-messaging/euro-native-messaging");
            let replaced = match install_messenger_binary(&binary_path, &dest) {
                Ok(InstallOutcome::Replaced) => {
                    tracing::info!("Installed native messaging binary at {}", dest.display());
                    true
                }
                Ok(InstallOutcome::Unchanged) => {
                    tracing::debug!(
                        "Native messaging binary at {} is already up to date",
                        dest.display()
                    );
                    false
                }
                Err(e) => {
                    tracing::warn!(
                        "Could not install native messaging binary to {}: {e}",
                        dest.display()
                    );
                    false
                }
            };
            (dest.to_string_lossy().to_string(), replaced)
        };

        #[cfg(target_os = "macos")]
        let manifest_configs: Vec<(&str, Vec<PathBuf>)> = {
            let home = dirs::home_dir().unwrap_or_default();
            vec![
                (
                    "hosts/mac.chromium.native-messaging.json",
                    vec![
                        home.join("Library/Application Support/Google/Chrome/NativeMessagingHosts"),
                        home.join("Library/Application Support/Chromium/NativeMessagingHosts"),
                        home.join("Library/Application Support/BraveSoftware/Brave-Browser/NativeMessagingHosts"),
                    ],
                ),
                (
                    "hosts/mac.edge.native-messaging.json",
                    vec![
                        home.join("Library/Application Support/Microsoft Edge/NativeMessagingHosts"),
                    ],
                ),
                (
                    "hosts/mac.firefox.native-messaging.json",
                    vec![
                        home.join("Library/Application Support/Mozilla/NativeMessagingHosts"),
                    ],
                ),
            ]
        };

        #[cfg(target_os = "linux")]
        let manifest_configs: Vec<(&str, Vec<PathBuf>)> = {
            let home = dirs::home_dir().unwrap_or_default();
            vec![
                (
                    "hosts/linux.chromium.native-messaging.json",
                    vec![
                        home.join(".config/google-chrome/NativeMessagingHosts"),
                        home.join(".config/chromium/NativeMessagingHosts"),
                        home.join(".config/BraveSoftware/Brave-Browser/NativeMessagingHosts"),
                    ],
                ),
                (
                    "hosts/linux.edge.native-messaging.json",
                    vec![home.join(".config/microsoft-edge/NativeMessagingHosts")],
                ),
                (
                    "hosts/linux.firefox.native-messaging.json",
                    vec![home.join(".mozilla/native-messaging-hosts")],
                ),
            ]
        };

        for (template_name, browser_dirs) in &manifest_configs {
            let template_path = resource_dir.join(template_name);
            let content = match std::fs::read_to_string(&template_path) {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!("Could not read native messaging template {template_name}: {e}");
                    continue;
                }
            };

            let mut manifest: serde_json::Value = match serde_json::from_str(&content) {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!(
                        "Could not parse native messaging template {template_name}: {e}"
                    );
                    continue;
                }
            };
            if let Some(obj) = manifest.as_object_mut() {
                obj.insert(
                    "path".to_string(),
                    serde_json::Value::String(manifest_binary_path.clone()),
                );
            }

            let manifest_json = match serde_json::to_string_pretty(&manifest) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("Could not serialize native messaging manifest: {e}");
                    continue;
                }
            };

            for dir in browser_dirs {
                if let Err(e) = std::fs::create_dir_all(dir) {
                    tracing::warn!("Could not create directory {}: {e}", dir.display());
                    continue;
                }
                let dest = dir.join("com.eurora.app.json");
                match std::fs::write(&dest, &manifest_json) {
                    Ok(()) => {
                        tracing::info!("Installed native messaging manifest to {}", dest.display())
                    }
                    Err(e) => tracing::warn!(
                        "Failed to write native messaging manifest to {}: {e}",
                        dest.display()
                    ),
                }
            }
        }

        messenger_replaced
    }
}

fn install_office_word_addin(app: &tauri::App) {
    use euro_tauri::office_addin::{Error, InstallOutcome, install_for_app};

    match install_for_app(app.handle()) {
        Ok(InstallOutcome::Installed { manifest_path }) => tracing::info!(
            "Installed Office add-in manifest at {}",
            manifest_path.display()
        ),
        Ok(InstallOutcome::SkippedHostNotPresent) => tracing::info!(
            "Microsoft Word has not been launched on this account; \
             deferring Office add-in install until next desktop launch"
        ),
        Ok(InstallOutcome::SkippedDevSideload) => tracing::info!(
            "Office add-in install skipped: EURORA_OFFICE_ADDIN_DEV_SIDELOAD is set, \
             deferring to the Vite-served add-in"
        ),
        Ok(InstallOutcome::SkippedUnsupportedOs) => {
            tracing::debug!("Office add-in install not applicable on this OS");
        }
        Err(Error::MissingResource(path)) => tracing::warn!(
            "Office add-in resources not bundled at {}; skipping install",
            path.display()
        ),
        Err(e) => tracing::warn!("Failed to install Office add-in: {e}"),
    }
}

/// Bind the bridge listener synchronously inside Tauri's `setup` and
/// spawn the accept loop in the background. The synchronous bind is the
/// load-bearing piece: by the time `setup` returns, the kernel socket
/// is in `LISTEN` state, so the very first add-in or native-messaging
/// connect can no longer race the bind with `ECONNREFUSED`.
///
/// We can't use `tauri::async_runtime::block_on` here: `setup` is
/// already running inside the tokio runtime context, and nested
/// `block_on` panics with "Cannot start a runtime from within a
/// runtime". Instead, spawn the bind+serve task and synchronously wait
/// on a `std::sync::mpsc` channel for the bind result before returning.
fn bind_and_serve_bridge() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, rx) =
        std::sync::mpsc::sync_channel::<Result<std::net::SocketAddr, euro_bridge::BridgeError>>(1);
    tauri::async_runtime::spawn(async move {
        match euro_bridge::bind_bridge_server().await {
            Ok(bound) => {
                let _ = tx.send(Ok(bound.local_addr()));
                if let Err(err) = bound.serve().await {
                    tracing::error!("Bridge accept loop ended with error: {err}");
                }
            }
            Err(err) => {
                let _ = tx.send(Err(err));
            }
        }
    });
    let local_addr = rx.recv()??;
    tracing::info!(
        "Bridge listener bound at ws://{local_addr}{}",
        euro_bridge::BRIDGE_PATH
    );
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

fn register_autostart(tauri_app: &mut tauri::App, settings: &SettingsState) {
    let should_register = !cfg!(debug_assertions) && !cfg!(target_os = "macos");
    let started_by_autostart = std::env::args().any(|arg| arg == "--startup-launch");

    if should_register && settings.local.general.autostart && !started_by_autostart {
        use tauri_plugin_autostart::MacosLauncher;
        use tauri_plugin_autostart::ManagerExt;

        let _ = tauri_app.handle().plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--startup-launch"]),
        ));

        let autostart_manager = tauri_app.autolaunch();
        if !autostart_manager.is_enabled().unwrap_or(false) {
            match autostart_manager.enable() {
                Ok(_) => tracing::debug!("Autostart enabled"),
                Err(e) => tracing::error!("Failed to enable autostart: {e}"),
            }
        }
    }
}

fn setup_tray(tauri_app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let app_handle = tauri_app.handle().clone();
    let open_i = MenuItem::with_id(tauri_app, "open", "Open", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(tauri_app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(tauri_app, &[&open_i, &quit_i])?;

    let icon = tauri_app
        .default_window_icon()
        .ok_or("No default window icon configured")?
        .clone();

    TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "quit" => app.exit(0),
            "open" => {
                if let Err(e) = show_and_focus_main(&app_handle) {
                    tracing::error!("Failed to show main window from tray: {e}");
                }
            }
            other => tracing::warn!("Unhandled tray menu event: {other}"),
        })
        .build(tauri_app)?;

    Ok(())
}

fn setup_main_window(
    tauri_app: &mut tauri::App,
    started_by_autostart: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let main_window = create_window(tauri_app.handle(), MAIN_WINDOW_LABEL, String::new())?;

    if started_by_autostart {
        let _ = main_window.hide();
    }

    let handle = tauri_app.handle().clone();
    main_window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            if let Some(w) = handle.get_webview_window(MAIN_WINDOW_LABEL) {
                let _ = w.hide();
            }
            api.prevent_close();
        }
    });

    Ok(())
}

fn init_state(
    tauri_app: &tauri::App,
    endpoint_manager: &std::sync::Arc<EndpointManager>,
) -> Result<(), Box<dyn std::error::Error>> {
    let app_handle = tauri_app.handle();

    // Single shared AuthManager so concurrent refreshes from any consumer
    // (thread, timeline, user) coalesce through one refresh lock.
    let auth_manager = euro_auth::AuthManager::new(endpoint_manager.clone());

    let thread_manager =
        euro_thread::ThreadManager::new(endpoint_manager.clone(), auth_manager.clone());
    app_handle.manage(SharedThreadManager::new(thread_manager));

    let timeline = euro_timeline::TimelineManager::builder()
        .endpoint_manager(endpoint_manager.clone())
        .auth_manager(auth_manager.clone())
        .build()?;
    app_handle.manage(Mutex::new(timeline));

    let context_provider: SharedChatContextProvider =
        std::sync::Arc::new(TimelineChatContextProvider::new(app_handle.clone()));
    app_handle.manage(context_provider);

    let path = tauri_app.path().app_data_dir()?;
    let user_controller = euro_user::UserController::new(path, auth_manager);
    app_handle.manage(SharedUserController::new(user_controller));
    app_handle.manage(ActiveStreamTokens::default());

    Ok(())
}

/// Bridge `TimelineManager`'s internal broadcast channels to the
/// frontend by emitting tauri-specta typed events. Each task takes a
/// fresh receiver from the manager (so a slow consumer can't starve
/// the producer or another listener) and runs for the app's lifetime.
fn spawn_timeline_listeners(app_handle: tauri::AppHandle) {
    let assets_handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        let mut asset_receiver = {
            let tl: tauri::State<'_, Mutex<TimelineManager>> = assets_handle.state();
            let timeline = tl.lock().await;
            timeline.subscribe_to_assets_events()
        };
        while let Ok(assets_event) = asset_receiver.recv().await {
            let _ = TimelineAssetsEvent(assets_event).emit(&assets_handle);
        }
    });

    let activity_handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        let mut activity_receiver = {
            let tl: tauri::State<'_, Mutex<TimelineManager>> = activity_handle.state();
            let timeline = tl.lock().await;
            timeline.subscribe_to_activity_events()
        };
        while let Ok(activity_event) = activity_receiver.recv().await {
            tracing::debug!("Activity changed to: {}", activity_event.name);

            let mut accent = None;
            let mut icon_base64 = None;

            if let Some(icon) = activity_event.icon.as_ref() {
                accent = color_thief::get_palette(icon, color_thief::ColorFormat::Rgba, 1, 2)
                    .ok()
                    .and_then(|c| c.into_iter().next())
                    .map(|c| AccentColor::from_rgb(c.r, c.g, c.b));
                icon_base64 = euro_vision::rgba_to_base64(icon).ok();
            }

            let _ = TimelineAppEvent {
                name: activity_event.name.clone(),
                accent,
                icon_base64,
                process_name: activity_event.process_name.clone(),
                process_id: activity_event.process_id,
            }
            .emit(&activity_handle);
        }
    });

    tauri::async_runtime::spawn(async move {
        let tl: tauri::State<'_, Mutex<TimelineManager>> = app_handle.state();
        let mut timeline = tl.lock().await;
        if let Err(e) = timeline.start().await {
            tracing::error!("Failed to start timeline collection: {e}");
        } else {
            tracing::debug!("Timeline collection started successfully");
        }
    });
}

/// Forward bridge registry changes — native-messenger registrations,
/// disconnects, and bundled-extension state reports — to the frontend as
/// `BrowserExtensionStatusChanged` events.
///
/// All three signals are routed through the same resolver
/// (`resolve_browser_extension_state`) so the frontend sees a single
/// consistent `BrowserExtensionState` per browser, regardless of which
/// signal triggered the recompute. The UI uses these to update the
/// extension affordance without polling.
fn spawn_browser_status_bridge(app_handle: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let service = euro_bridge::BridgeService::get_or_init();
        let mut registrations_rx = service.subscribe_to_registrations();
        let mut disconnects_rx = service.subscribe_to_disconnects();
        let mut extension_states_rx = service.subscribe_to_extension_states();

        let emit = |process_name: String| {
            let state = resolve_browser_extension_state(&process_name);
            let _ = BrowserExtensionStatusChanged {
                process_name,
                state,
            }
            .emit(&app_handle);
        };

        loop {
            tokio::select! {
                event = registrations_rx.recv() => {
                    match event {
                        Ok(reg) => emit(reg.app_name),
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!(
                                "Browser registration subscription lagged by {n} events"
                            );
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
                event = disconnects_rx.recv() => {
                    match event {
                        Ok(reg) => emit(reg.app_name),
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!(
                                "Browser disconnect subscription lagged by {n} events"
                            );
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
                event = extension_states_rx.recv() => {
                    match event {
                        Ok(update) => {
                            // The bundled-state channel is keyed by `app_kind`;
                            // map it back to the focused-window process name
                            // the frontend filters on.
                            if update.app_kind == SAFARI_BRIDGE_APP_KIND {
                                emit(euro_process::Browser::Safari.process_name().to_owned());
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!(
                                "Bundled extension state subscription lagged by {n} events"
                            );
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
            }
        }
    });
}

/// Install rustls' default crypto provider before any rustls consumer
/// runs. The desktop binary pulls rustls in transitively through
/// `reqwest`, `tauri-plugin-http`, `tauri-plugin-updater`, and
/// `tokio-tungstenite` (in the native-messaging dep tree); rustls
/// panics at first use if both the `ring` and `aws-lc-rs` features are
/// enabled in the dependency graph and nothing has installed a
/// provider explicitly. We pick `aws-lc-rs` to match the workspace
/// `rustls` default (`features = ["aws_lc_rs"]`).
///
/// Idempotent: subsequent calls (or a provider already installed by
/// another crate during this process's lifetime) are tolerated.
fn install_default_crypto_provider() {
    if rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .is_err()
    {
        tracing::debug!(
            "rustls default crypto provider was already installed; leaving existing provider in place"
        );
    }
}

fn main() {
    // Inject build-time URL bake-ins into the process env. Whatever
    // is in the host shell already (vars exported by `just dev` via
    // `set dotenv-load`, or one-off `WEB_URL=foo cargo run` overrides)
    // wins; the bake-time values only fill the gaps so packaged
    // release builds — which have no `.env` on disk — still know
    // where to point.
    euro_tauri::load_env();

    install_default_crypto_provider();

    // Initialize Sentry as early as possible so panics during the rest
    // of startup are still captured. The controller is then handed to
    // the Tauri app state so the `system.reinit_telemetry` procedure
    // can swap the underlying client when consent changes at runtime.
    //
    // The early peek reads only the cloud cache (which carries the
    // consent toggles); the `distinct_id` lives in `local.json` and is
    // applied later, after `SettingsState::load_or_migrate` runs inside
    // `setup`. The intervening Sentry events are tagged with no user
    // scope — acceptable for the boot window.
    let early_cache = CloudSettingsCache::peek_from_default_path();
    let telemetry_controller = std::sync::Arc::new(TelemetryController::init(
        wants_errors(&early_cache.settings.desktop.telemetry),
        None,
    ));

    #[cfg(debug_assertions)]
    {
        use keyring::{mock, set_default_credential_builder};
        set_default_credential_builder(mock::default_credential_builder());
    }

    let tauri_context = generate_context!();

    tracing::debug!("Starting Tauri application...");

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime")
        .block_on(async {
            tauri::async_runtime::set(tokio::runtime::Handle::current());

            let telemetry_controller = telemetry_controller.clone();
            let specta = build_specta();

            // Regenerate the TypeScript bindings on every dev launch.
            // `specta-typescript` 0.0.12 fails the export by default if any
            // `i64`/`u64` field crosses the wire without an explicit
            // `#[specta(type = ...)]` override, which is the strictness we
            // want — silently bridging through `bigint` masks lossy round
            // trips on the JS side.
            #[cfg(debug_assertions)]
            {
                let bindings_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../../apps/desktop/src/lib/bindings/specta.bindings.ts");
                specta
                    .export(specta_typescript::Typescript::default(), &bindings_path)
                    .expect("Failed to export tauri-specta bindings");
            }

            let builder = tauri::Builder::default()
                .plugin(tauri_plugin_os::init())
                .plugin(tauri_plugin_clipboard_manager::init())
                .plugin(tauri_plugin_updater::Builder::new().build())
                .invoke_handler(specta.invoke_handler())
                .setup(move |tauri_app| {
                    // `mount_events` must run inside `setup` so the typed
                    // event channels are wired before any procedure has a
                    // chance to emit. Move `specta` into the closure so its
                    // event registry stays alive for the app lifetime.
                    specta.mount_events(tauri_app);

                    let messenger_replaced = install_native_messaging_manifests(tauri_app);
                    install_office_word_addin(tauri_app);
                    bind_and_serve_bridge()?;

                    // We just dropped a fresh messenger binary on disk. Any
                    // browser messengers from the previous desktop session
                    // are sitting in their reconnect-backoff loop and will
                    // hit the new bridge in the next couple of seconds.
                    // Open a short window during which they're sent
                    // Shutdown so the browser respawns them from the new
                    // binary. Three seconds is well above the messenger's
                    // 2-second reconnect interval and well below the
                    // browser extension's 5-second respawn delay, so a
                    // messenger that lands inside the window won't be
                    // followed by another that also lands inside it.
                    if messenger_replaced {
                        const MESSENGER_PURGE_WINDOW: std::time::Duration =
                            std::time::Duration::from_secs(3);
                        euro_bridge::BridgeService::get_or_init()
                            .open_browser_purge_window(MESSENGER_PURGE_WINDOW);
                    }

                    let data_dir = tauri_app.path().app_data_dir()?;
                    init_encryption(data_dir)?;

                    let started_by_autostart =
                        std::env::args().any(|arg| arg == "--startup-launch");

                    let settings = SettingsState::load_or_migrate_from_default_path()?;
                    // The persisted ConnectionMode always resolves to a
                    // non-empty URL, so we never need an env-fallback path.
                    let endpoint_url = settings.local.api.endpoint();
                    tracing::info!(
                        mode = ?settings.local.api.mode,
                        endpoint_url = %endpoint_url,
                        baked_default = euro_settings::DEFAULT_API_URL,
                        "Resolved API endpoint at startup"
                    );
                    let endpoint_manager = std::sync::Arc::new(EndpointManager::new(endpoint_url)?);

                    // Reconcile the early-Sentry guard against the
                    // settings we just authoritatively loaded from disk.
                    // The `peek` may have observed a missing or unreadable
                    // file; this call ensures the runtime telemetry state
                    // matches what's persisted before the app comes up.
                    telemetry_controller.reapply(
                        wants_errors(&settings.cache.settings.desktop.telemetry),
                        settings.local.telemetry.distinct_id.as_deref(),
                    );

                    let http_client: SharedHttpClient = reqwest::Client::builder()
                        .timeout(std::time::Duration::from_secs(5))
                        .build()
                        .expect("failed to build shared HTTP client");

                    tauri_app.manage(endpoint_manager.clone());
                    tauri_app.manage(telemetry_controller.clone());
                    tauri_app.manage(WindowState::default());
                    tauri_app.manage(http_client);

                    // All command-visible state must be in place before the
                    // WebView is created — once the window exists the
                    // frontend starts firing IPC calls, and any procedure
                    // that does `try_state::<...>()` will see `None` if its
                    // backing manager hasn't been registered yet.
                    init_state(tauri_app, &endpoint_manager)?;

                    register_autostart(tauri_app, &settings);
                    tauri_app.manage(Mutex::new(settings));
                    setup_main_window(tauri_app, started_by_autostart)?;
                    setup_tray(tauri_app)?;

                    spawn_timeline_listeners(tauri_app.handle().clone());
                    spawn_browser_status_bridge(tauri_app.handle().clone());

                    Ok(())
                })
                .plugin(tauri_plugin_http::init())
                .plugin(tauri_plugin_opener::init())
                .plugin(
                    tauri_plugin_tracing::Builder::new()
                        .filter(|metadata| {
                            let target = metadata.target();
                            let is_euro_crate = target.starts_with("euro_");
                            let is_common_crate = target.starts_with("agent_chain")
                                || target.starts_with("agent_graph")
                                || target.starts_with("auth_core")
                                || target.starts_with("focus_tracker")
                                || target.starts_with("thread_core");
                            let is_webview = target.starts_with("webview");
                            let is_warning_or_above = *metadata.level() <= tracing::Level::WARN;
                            is_euro_crate || is_common_crate || is_webview || is_warning_or_above
                        })
                        .with_max_level(tauri_plugin_tracing::LevelFilter::DEBUG)
                        .with_colors()
                        // Forward `tracing` events to Sentry alongside the
                        // webview/console output. ERROR events become Sentry
                        // `Event`s (with backtraces); INFO/WARN ride along as
                        // breadcrumbs on the next Event. The layer is benign
                        // when Sentry isn't initialized — events flowing
                        // through it are simply dropped — so installing it
                        // unconditionally lets us avoid coordinating layer
                        // composition with the runtime consent toggle.
                        .with_layer(Box::new(
                            sentry_tracing::layer::<tracing_subscriber::Registry>().event_filter(
                                |metadata| match *metadata.level() {
                                    tracing::Level::ERROR => sentry_tracing::EventFilter::Event,
                                    tracing::Level::WARN | tracing::Level::INFO => {
                                        sentry_tracing::EventFilter::Breadcrumb
                                    }
                                    _ => sentry_tracing::EventFilter::Ignore,
                                },
                            ),
                        ))
                        .with_default_subscriber()
                        .build(),
                )
                .plugin(tauri_plugin_shell::init())
                .plugin(tauri_plugin_single_instance::init(|app, _, _| {
                    if let Err(e) = show_and_focus_main(app) {
                        tracing::error!("Failed to show main window for second instance: {e}");
                    }
                }))
                .on_window_event(|window, event| {
                    if let tauri::WindowEvent::Destroyed = event {
                        window
                            .app_handle()
                            .state::<WindowState>()
                            .remove(window.label());
                    }
                });

            #[cfg(not(target_os = "linux"))]
            let builder = builder.plugin(
                tauri_plugin_window_state::Builder::default()
                    .with_state_flags(
                        tauri_plugin_window_state::StateFlags::all()
                            & !tauri_plugin_window_state::StateFlags::DECORATIONS,
                    )
                    .build(),
            );

            builder
                .build(tauri_context)
                .expect("Failed to build tauri app")
                .run(|_app_handle, event| {
                    if matches!(event, tauri::RunEvent::Exit) {
                        let (tx, rx) = std::sync::mpsc::sync_channel::<()>(1);
                        tauri::async_runtime::spawn(async move {
                            euro_bridge::stop_bridge_server().await;
                            let _ = tx.send(());
                        });
                        let _ = rx.recv();
                    }
                });
        });
}
