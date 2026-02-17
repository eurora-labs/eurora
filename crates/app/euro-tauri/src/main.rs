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
        conversation_procedures::{ConversationApi, ConversationApiImpl},
        monitor_procedures::{MonitorApi, MonitorApiImpl},
        onboarding_procedures::{OnboardingApi, OnboardingApiImpl},
        prompt_procedures::{PromptApi, PromptApiImpl},
        settings_procedures::{SettingsApi, SettingsApiImpl},
        system_procedures::{SystemApi, SystemApiImpl},
        third_party_procedures::{ThirdPartyApi, ThirdPartyApiImpl},
        timeline_procedures::{TauRpcTimelineApiEventTrigger, TimelineApi, TimelineApiImpl},
    },
    shared_types::SharedConversationManager,
};
use euro_timeline::TimelineManager;
use tauri::{
    Manager, generate_context,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};

use taurpc::Router;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

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
                warn!(
                    "Could not resolve resource dir, skipping native messaging manifest install: {e}"
                );
                return;
            }
        };

        let binary_path = match std::env::current_exe() {
            Ok(exe) => match exe.parent() {
                Some(dir) => dir.join("euro-native-messaging"),
                None => {
                    warn!("Could not resolve parent directory of current exe");
                    return;
                }
            },

            Err(e) => {
                warn!("Could not resolve current exe path: {e}");
                return;
            }
        };
        // On macOS the sidecar lives next to the main executable inside the .app
        // bundle. On Linux we copy it to a stable well-known path so that
        // manifests survive package-manager upgrades that change the install prefix.
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

            // On Linux, copy the sidecar binary to ~/.eurora/native-messaging/
            // so that browser manifests can reference a stable, well-known path.
            let native_messaging_dir = home.join(".eurora/native-messaging");
            if let Err(e) = std::fs::create_dir_all(&native_messaging_dir) {
                warn!(
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
                            warn!(
                                "Could not set executable permission on {}: {e}",
                                dest.display()
                            );
                        }
                        info!("Copied native messaging binary to {}", dest.display());
                    }
                    Err(e) => warn!(
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
                    warn!("Could not read native messaging template {template_name}: {e}");
                    continue;
                }
            };

            let mut manifest: serde_json::Value = match serde_json::from_str(&content) {
                Ok(v) => v,
                Err(e) => {
                    warn!("Could not parse native messaging template {template_name}: {e}");
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
                    warn!("Could not serialize native messaging manifest: {e}");
                    continue;
                }
            };

            for dir in browser_dirs {
                if let Err(e) = std::fs::create_dir_all(dir) {
                    warn!("Could not create directory {}: {e}", dir.display());
                    continue;
                }
                let dest = dir.join("com.eurora.app.json");
                match std::fs::write(&dest, &manifest_json) {
                    Ok(()) => info!("Installed native messaging manifest to {}", dest.display()),
                    Err(e) => warn!(
                        "Failed to write native messaging manifest to {}: {e}",
                        dest.display()
                    ),
                }
            }
        }
    }
}

async fn initialize_posthog() -> Result<(), posthog_rs::Error> {
    let posthog_key = option_env!("POSTHOG_API_KEY");
    if let Some(key) = posthog_key {
        return posthog_rs::init_global(key).await;
    }
    Err(posthog_rs::Error::Connection(
        "Posthog key not found".to_string(),
    ))
}

fn main() {
    dotenv().ok();

    #[cfg(feature = "mock-keyring")]
    {
        use keyring::{mock, set_default_credential_builder};
        set_default_credential_builder(mock::default_credential_builder());
    }

    // TODO: Check if this still works on Nightly
    if cfg!(not(debug_assertions)) {
        let _guard = sentry::init((
            // TODO: Replace with Sentry DSN from env
            "https://a0c23c10925999f104c7fd07fd8e3871@o4508907847352320.ingest.de.sentry.io/4510097240424528",
            sentry::ClientOptions {
                release: sentry::release_name!(),
                traces_sample_rate: 0.0,
                enable_logs: true,
                send_default_pii: true, // during closed beta all metrics are non-anonymous
                debug: true,
                ..Default::default()
            },
        ));
    }

    let tauri_context = generate_context!();

    debug!("Starting Tauri application...");

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            debug!("Setting tokio runtime");
            tauri::async_runtime::set(tokio::runtime::Handle::current());

            let builder = tauri::Builder::default()
                .plugin(tauri_plugin_os::init())
                .plugin(tauri_plugin_updater::Builder::new().build())
                .setup(move |tauri_app| {
                    install_native_messaging_manifests(tauri_app);

                    let started_by_autostart =
                        std::env::args().any(|arg| arg == "--startup-launch");
                    if started_by_autostart {
                        let event = posthog_rs::Event::new_anon("start_app_by_autostart");

                        tauri::async_runtime::spawn(async move {
                            let _ = posthog_rs::capture(event).await.map_err(|e| {
                                error!("Failed to capture posthog event: {}", e);
                            });
                        });
                    }

                    let app_settings = AppSettings::load_from_default_path_creating().unwrap();
                    let endpoint_url = &app_settings.api.endpoint;
                    let endpoint_manager = if endpoint_url.is_empty() {
                        EndpointManager::from_env()
                    } else {
                        EndpointManager::new(endpoint_url)
                    }
                    .expect("Failed to initialize API endpoint");
                    let endpoint_manager = std::sync::Arc::new(endpoint_manager);
                    tauri_app.manage(endpoint_manager.clone());
                    tauri_app.manage(Mutex::new(app_settings.clone()));

                    tauri::async_runtime::spawn(async move {
                        let _ = initialize_posthog().await.map_err(|e| {
                            error!("Failed to initialize posthog: {}", e);
                        });
                    });

                    // Autostart is never registered in debug builds — during
                    // development you launch from your IDE or terminal and
                    // don't want a launch agent or registry entry lingering.
                    //
                    // In release builds:
                    //   • macOS — the Swift launcher (Eurora.app) registers
                    //     itself as a login item via SMAppService.  The
                    //     embedded Eurora.app must not create its own
                    //     launch agent (unstable path, bypasses Safari bridge).
                    //   • Windows / Linux — the Tauri app is the top-level
                    //     binary so it registers itself directly.
                    let should_register_autostart =
                        !cfg!(debug_assertions) && !cfg!(target_os = "macos");

                    if should_register_autostart
                        && app_settings.general.autostart
                        && !started_by_autostart
                    {
                        use tauri_plugin_autostart::MacosLauncher;
                        use tauri_plugin_autostart::ManagerExt;

                        let _ = tauri_app.handle().plugin(tauri_plugin_autostart::init(
                            MacosLauncher::LaunchAgent,
                            Some(vec!["--startup-launch"]),
                        ));

                        let autostart_manager = tauri_app.autolaunch();
                        if !autostart_manager.is_enabled().unwrap_or(false) {
                            match autostart_manager.enable() {
                                Ok(_) => debug!("Autostart enabled"),
                                Err(e) => error!("Failed to enable autostart: {}", e),
                            }
                        }
                    }

                    let main_window = create_window(tauri_app.handle(), "main", "".into())
                        .expect("Failed to create main window");

                    if started_by_autostart {
                        main_window.hide().expect("Failed to hide main window");
                    }

                    let app_handle = tauri_app.handle();

                    let main_window_handle = app_handle.clone();
                    main_window.on_window_event(move |event| {
                        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                            let main_window = main_window_handle
                                .get_window("main")
                                .expect("Failed to get main window");
                            main_window.hide().expect("Failed to hide main window");
                            api.prevent_close();
                        }
                        if let tauri::WindowEvent::Focused(focused) = event {
                            let main_window = main_window_handle
                                .get_window("main")
                                .expect("Failed to get main window");
                            let minimized = main_window
                                .is_minimized()
                                .expect("Failed to get window state");
                            if !*focused && minimized {
                                main_window.hide().expect("Failed to hide main window");
                            }
                        }
                    });

                    let open_i = MenuItem::with_id(tauri_app, "open", "Open", true, None::<&str>)?;
                    let quit_i = MenuItem::with_id(tauri_app, "quit", "Quit", true, None::<&str>)?;
                    let menu = Menu::with_items(tauri_app, &[&open_i, &quit_i])?;
                    let tray_icon_handle = app_handle.clone();
                    TrayIconBuilder::new()
                        .icon(tauri_app.default_window_icon().unwrap().clone())
                        .menu(&menu)
                        .show_menu_on_left_click(true)
                        .on_menu_event(move |app, event| {
                            if event.id == "quit" {
                                app.exit(0);
                            }
                            if event.id == "open" {
                                let main_window = tray_icon_handle
                                    .get_window("main")
                                    .expect("Failed to get main window");
                                main_window
                                    .unminimize()
                                    .map_err(|e| error!("Failed to set window state: {}", e))
                                    .ok();
                                main_window
                                    .show()
                                    .map_err(|e| error!("Failed to show main window: {}", e))
                                    .ok();
                            }
                        })
                        .build(tauri_app)
                        .expect("Failed to create tray icon");

                    let conversation_handle = app_handle.clone();
                    let conversation_channel_rx = endpoint_manager.subscribe();
                    tauri::async_runtime::spawn(async move {
                        let conversation_manager =
                            euro_conversation::ConversationManager::new(conversation_channel_rx);
                        conversation_handle
                            .manage(SharedConversationManager::new(conversation_manager));
                    });

                    let timeline_handle = app_handle.clone();
                    let db_app_handle = app_handle.clone();
                    let timeline_channel_rx = endpoint_manager.subscribe();
                    tauri::async_runtime::spawn(async move {
                        let timeline = euro_timeline::TimelineManager::builder()
                            .channel_rx(timeline_channel_rx)
                            .build()
                            .expect("Failed to create timeline");
                        timeline_handle.manage(Mutex::new(timeline));
                        let timeline_mutex = db_app_handle.state::<Mutex<TimelineManager>>();

                        let mut asset_receiver = {
                            let timeline = timeline_mutex.lock().await;
                            timeline.subscribe_to_assets_events()
                        };
                        let assets_timeline_handle = db_app_handle.clone();
                        tauri::async_runtime::spawn(async move {
                            while let Ok(assets_event) = asset_receiver.recv().await {
                                let _ = TauRpcTimelineApiEventTrigger::new(
                                    assets_timeline_handle.clone(),
                                )
                                .new_assets_event(assets_event);
                            }
                        });

                        let mut activity_receiver = {
                            let timeline = timeline_mutex.lock().await;
                            timeline.subscribe_to_activity_events()
                        };

                        let activity_timeline_handle = db_app_handle.clone();
                        tauri::async_runtime::spawn(async move {
                            while let Ok(activity_event) = activity_receiver.recv().await {
                                debug!("Activity changed to: {}", activity_event.name.clone(),);

                                let mut primary_icon_color = None;
                                let mut icon_base64 = None;

                                if let Some(icon) = activity_event.icon.as_ref() {
                                    primary_icon_color = color_thief::get_palette(
                                        icon,
                                        color_thief::ColorFormat::Rgba,
                                        10,
                                        10,
                                    )
                                    .ok()
                                    .map(|c| {
                                        format!(
                                            "#{r:02X}{g:02X}{b:02X}",
                                            r = c[0].r,
                                            g = c[0].g,
                                            b = c[0].b
                                        )
                                    });
                                    icon_base64 = euro_vision::rgba_to_base64(icon).ok();
                                }

                                let _ = TauRpcTimelineApiEventTrigger::new(
                                    activity_timeline_handle.clone(),
                                )
                                .new_app_event(TimelineAppEvent {
                                    name: activity_event.name.clone(),
                                    color: primary_icon_color,
                                    icon_base64,
                                });
                            }
                        });

                        let mut timeline = timeline_mutex.lock().await;
                        if let Err(e) = timeline.start().await {
                            error!("Failed to start timeline collection: {}", e);
                        } else {
                            debug!("Timeline collection started successfully");
                        }
                    });

                    let app_handle_user = app_handle.clone();
                    let path = tauri_app.path().app_data_dir().unwrap();
                    let user_channel_rx = endpoint_manager.subscribe();
                    tauri::async_runtime::spawn(async move {
                        let user_controller = euro_user::Controller::new(path, user_channel_rx)
                            .map_err(|e| {
                                error!("Failed to create user controller: {}", e);
                                e
                            })
                            .unwrap();
                        app_handle_user.manage(SharedUserController::new(user_controller));
                    });

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
                    if let Some(window) = app.get_window("main") {
                        let _ = window.show();
                        let _ = window.unminimize();
                        let _ = window.set_focus();
                    }
                }))
                .on_window_event(|window, event| match event {
                    #[cfg(target_os = "macos")]
                    tauri::WindowEvent::CloseRequested { .. } => {
                        let app_handle = window.app_handle();
                        if app_handle.windows().len() == 1 {
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
            let builder = builder.plugin(tauri_plugin_window_state::Builder::default().build());

            let router = Router::new()
                .export_config(
                    specta_typescript::Typescript::default()
                        .bigint(specta_typescript::BigIntExportBehavior::BigInt),
                )
                .merge(AuthApiImpl.into_handler())
                .merge(TimelineApiImpl.into_handler())
                .merge(ConversationApiImpl.into_handler())
                .merge(SettingsApiImpl.into_handler())
                .merge(ThirdPartyApiImpl.into_handler())
                .merge(MonitorApiImpl.into_handler())
                .merge(SystemApiImpl.into_handler())
                .merge(ContextChipApiImpl.into_handler())
                .merge(PromptApiImpl.into_handler())
                .merge(OnboardingApiImpl.into_handler())
                .merge(ChatApiImpl.into_handler());
            builder
                .invoke_handler(router.into_handler())
                .build(tauri_context)
                .expect("Failed to build tauri app")
                .run(|_app_handle, _event| {});
        });
}
