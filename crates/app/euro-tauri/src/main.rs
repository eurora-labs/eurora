#![cfg_attr(
    all(windows, not(test), not(debug_assertions)),
    windows_subsystem = "windows"
)]

use dotenv::dotenv;
use euro_endpoint::EndpointManager;
use euro_settings::AppSettings;
use euro_tauri::procedures::timeline_procedures::TimelineAppEvent;
use euro_tauri::shared_types::SharedUserController;
use euro_tauri::{
    WindowState, create_window,
    procedures::{
        auth_procedures::{AuthApi, AuthApiImpl},
        chat_procedures::{ChatApi, ChatApiImpl},
        context_chip_procedures::{ContextChipApi, ContextChipApiImpl},
        monitor_procedures::{MonitorApi, MonitorApiImpl},
        onboarding_procedures::{OnboardingApi, OnboardingApiImpl},
        payment_procedures::{PaymentApi, PaymentApiImpl},
        prompt_procedures::{PromptApi, PromptApiImpl},
        settings_procedures::{SettingsApi, SettingsApiImpl},
        system_procedures::{SystemApi, SystemApiImpl},
        third_party_procedures::{ThirdPartyApi, ThirdPartyApiImpl},
        thread_procedures::{ThreadApi, ThreadApiImpl},
        timeline_procedures::{TauRpcTimelineApiEventTrigger, TimelineApi, TimelineApiImpl},
    },
    shared_types::SharedThreadManager,
};
use euro_timeline::TimelineManager;
use tauri::{
    Manager, generate_context,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};

use taurpc::Router;
use tokio::sync::Mutex;

fn install_native_messaging_manifests(app: &tauri::App) {
    #[cfg(target_os = "windows")]
    {
        let _ = app;
    }

    #[cfg(not(target_os = "windows"))]
    {
        #[cfg(target_os = "linux")]
        use std::os::unix::fs::PermissionsExt;
        use std::path::PathBuf;

        let resource_dir = match app.path().resource_dir() {
            Ok(dir) => dir,
            Err(e) => {
                tracing::warn!(
                    "Could not resolve resource dir, skipping native messaging manifest install: {e}"
                );
                return;
            }
        };

        let binary_path = match std::env::current_exe() {
            Ok(exe) => match exe.parent() {
                Some(dir) => dir.join("euro-native-messaging"),
                None => {
                    tracing::warn!("Could not resolve parent directory of current exe");
                    return;
                }
            },

            Err(e) => {
                tracing::warn!("Could not resolve current exe path: {e}");
                return;
            }
        };

        #[cfg(target_os = "macos")]
        let manifest_binary_path = binary_path.to_string_lossy().to_string();

        #[cfg(target_os = "linux")]
        let manifest_binary_path = {
            let home = dirs::home_dir().unwrap_or_default();
            home.join(".eurora/native-messaging/euro-native-messaging")
                .to_string_lossy()
                .to_string()
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

            let native_messaging_dir = home.join(".eurora/native-messaging");
            if let Err(e) = std::fs::create_dir_all(&native_messaging_dir) {
                tracing::warn!(
                    "Could not create native messaging directory {}: {e}",
                    native_messaging_dir.display()
                );
            } else {
                let dest = native_messaging_dir.join("euro-native-messaging");
                match std::fs::copy(&binary_path, &dest) {
                    Ok(_) => {
                        if let Err(e) = std::fs::set_permissions(
                            &dest,
                            <std::fs::Permissions as PermissionsExt>::from_mode(0o755),
                        ) {
                            tracing::warn!(
                                "Could not set executable permission on {}: {e}",
                                dest.display()
                            );
                        }
                        tracing::info!("Copied native messaging binary to {}", dest.display());
                    }
                    Err(e) => tracing::warn!(
                        "Could not copy native messaging binary to {}: {e}",
                        dest.display()
                    ),
                }
            }

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
    }
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

fn register_autostart(tauri_app: &mut tauri::App, app_settings: &AppSettings) {
    let should_register = !cfg!(debug_assertions) && !cfg!(target_os = "macos");
    let started_by_autostart = std::env::args().any(|arg| arg == "--startup-launch");

    if should_register && app_settings.general.autostart && !started_by_autostart {
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
        .on_menu_event(move |app, event| {
            if event.id == "quit" {
                app.exit(0);
            }
            if event.id == "open" {
                if let Some(main_window) = app_handle.get_webview_window("main") {
                    let _ = main_window.unminimize();
                    let _ = main_window.show();
                } else {
                    tracing::error!("Main window not found");
                }
            }
        })
        .build(tauri_app)?;

    Ok(())
}

fn setup_main_window(
    tauri_app: &mut tauri::App,
    started_by_autostart: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let main_window = create_window(tauri_app.handle(), "main", String::new())?;

    if started_by_autostart {
        let _ = main_window.hide();
    }

    let handle = tauri_app.handle().clone();
    main_window.on_window_event(move |event| match event {
        tauri::WindowEvent::CloseRequested { api, .. } => {
            if let Some(w) = handle.get_webview_window("main") {
                let _ = w.minimize();
            }
            api.prevent_close();
        }
        _ => {}
    });

    Ok(())
}

fn init_state(
    tauri_app: &tauri::App,
    endpoint_manager: &std::sync::Arc<EndpointManager>,
) -> Result<(), Box<dyn std::error::Error>> {
    let app_handle = tauri_app.handle();

    let thread_channel_rx = endpoint_manager.subscribe();
    let thread_manager = euro_thread::ThreadManager::new(thread_channel_rx);
    app_handle.manage(SharedThreadManager::new(thread_manager));

    let timeline_channel_rx = endpoint_manager.subscribe();
    let timeline = euro_timeline::TimelineManager::builder()
        .channel_rx(timeline_channel_rx)
        .build()?;
    app_handle.manage(Mutex::new(timeline));

    let path = tauri_app.path().app_data_dir()?;
    let user_channel_rx = endpoint_manager.subscribe();
    let user_controller = euro_user::Controller::new(path, user_channel_rx)?;
    app_handle.manage(SharedUserController::new(user_controller));

    Ok(())
}

fn spawn_timeline_listeners(app_handle: tauri::AppHandle) {
    let assets_handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        let mut asset_receiver = {
            let tl: tauri::State<'_, Mutex<TimelineManager>> = assets_handle.state();
            let timeline = tl.lock().await;
            timeline.subscribe_to_assets_events()
        };
        while let Ok(assets_event) = asset_receiver.recv().await {
            let _ = TauRpcTimelineApiEventTrigger::new(assets_handle.clone())
                .new_assets_event(assets_event);
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

            let mut primary_icon_color = None;
            let mut icon_bg = None;
            let mut icon_base64 = None;

            if let Some(icon) = activity_event.icon.as_ref() {
                if let Some(c) =
                    color_thief::get_palette(icon, color_thief::ColorFormat::Rgba, 10, 10)
                        .ok()
                        .and_then(|c| c.into_iter().next())
                {
                    let (r, g, b) = (c.r, c.g, c.b);
                    primary_icon_color = Some(format!("#{r:02X}{g:02X}{b:02X}"));
                    let luminance = 0.299 * r as f64 + 0.587 * g as f64 + 0.114 * b as f64;
                    icon_bg = Some(
                        if luminance / 255.0 > 0.5 {
                            "black"
                        } else {
                            "white"
                        }
                        .to_string(),
                    );
                }
                icon_base64 = euro_vision::rgba_to_base64(icon).ok();
            }

            let _ = TauRpcTimelineApiEventTrigger::new(activity_handle.clone()).new_app_event(
                TimelineAppEvent {
                    name: activity_event.name.clone(),
                    color: primary_icon_color,
                    icon_bg,
                    icon_base64,
                },
            );
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

fn build_router() -> Router<tauri::Wry> {
    Router::new()
        .export_config(
            specta_typescript::Typescript::default()
                .bigint(specta_typescript::BigIntExportBehavior::BigInt),
        )
        .merge(AuthApiImpl.into_handler())
        .merge(TimelineApiImpl.into_handler())
        .merge(ThreadApiImpl.into_handler())
        .merge(SettingsApiImpl.into_handler())
        .merge(ThirdPartyApiImpl.into_handler())
        .merge(MonitorApiImpl.into_handler())
        .merge(SystemApiImpl.into_handler())
        .merge(ContextChipApiImpl.into_handler())
        .merge(PromptApiImpl.into_handler())
        .merge(OnboardingApiImpl.into_handler())
        .merge(PaymentApiImpl.into_handler())
        .merge(ChatApiImpl.into_handler())
}

fn main() {
    dotenv().ok();

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

            let builder = tauri::Builder::default()
                .plugin(tauri_plugin_os::init())
                .plugin(tauri_plugin_clipboard_manager::init())
                .plugin(tauri_plugin_updater::Builder::new().build())
                .setup(move |tauri_app| {
                    install_native_messaging_manifests(tauri_app);

                    let data_dir = tauri_app.path().app_data_dir()?;
                    init_encryption(data_dir)?;

                    let started_by_autostart =
                        std::env::args().any(|arg| arg == "--startup-launch");

                    let app_settings = AppSettings::load_from_default_path_creating()?;
                    let endpoint_url = &app_settings.api.endpoint;
                    let endpoint_manager = if endpoint_url.is_empty() {
                        EndpointManager::from_env()
                    } else {
                        EndpointManager::new(endpoint_url)
                    }?;
                    let endpoint_manager = std::sync::Arc::new(endpoint_manager);

                    tauri_app.manage(endpoint_manager.clone());
                    tauri_app.manage(Mutex::new(app_settings.clone()));
                    tauri_app.manage(WindowState::default());

                    register_autostart(tauri_app, &app_settings);
                    setup_main_window(tauri_app, started_by_autostart)?;
                    setup_tray(tauri_app)?;

                    init_state(tauri_app, &endpoint_manager)?;
                    spawn_timeline_listeners(tauri_app.handle().clone());

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
                                || target.starts_with("proto_gen");
                            let is_webview = target.starts_with("webview");
                            let is_warning_or_above = *metadata.level() <= tracing::Level::WARN;
                            is_euro_crate || is_common_crate || is_webview || is_warning_or_above
                        })
                        .with_max_level(tauri_plugin_tracing::LevelFilter::DEBUG)
                        .with_colors()
                        .with_default_subscriber()
                        .build(),
                )
                .plugin(tauri_plugin_shell::init())
                .plugin(tauri_plugin_single_instance::init(|app, _, _| {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.unminimize();
                        let _ = window.set_focus();
                    }
                }))
                .on_window_event(|window, event| match event {
                    #[cfg(target_os = "macos")]
                    tauri::WindowEvent::CloseRequested { .. } => {
                        let app_handle = window.app_handle();
                        if app_handle.webview_windows().len() == 1 {
                            app_handle.exit(0);
                        }
                    }
                    tauri::WindowEvent::Destroyed => {
                        window
                            .app_handle()
                            .state::<WindowState>()
                            .remove(window.label());
                    }
                    tauri::WindowEvent::Focused(false) => {}

                    _ => {}
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

            let router = build_router();
            builder
                .invoke_handler(router.into_handler())
                .build(tauri_context)
                .expect("Failed to build tauri app")
                .run(|_app_handle, _event| {});
        });
}
