#![cfg_attr(
    all(windows, not(test), not(debug_assertions)),
    windows_subsystem = "windows"
)]

use dotenv::dotenv;
// use eur_conversation::{ChatMessage, Conversation, ConversationStorage};
mod launcher;
mod util;
use eur_encrypt::MainKey;
use eur_native_messaging::create_grpc_ipc_client;
use eur_settings::AppSettings;
use eur_tauri::{
    WindowState, create_hover, create_launcher, create_window,
    procedures::{
        auth_procedures::{AuthApi, AuthApiImpl},
        chat_procedures::{ChatApi, ChatApiImpl},
        context_chip_procedures::{ContextChipApi, ContextChipApiImpl},
        conversation_procedures::{ConversationApi, ConversationApiImpl},
        message_procedures::{MessageApi, MessageApiImpl},
        monitor_procedures::{MonitorApi, MonitorApiImpl},
        prompt_procedures::{PromptApi, PromptApiImpl},
        settings_procedures::{SettingsApi, SettingsApiImpl},
        system_procedures::{SystemApi, SystemApiImpl},
        third_party_procedures::{ThirdPartyApi, ThirdPartyApiImpl},
        user_procedures::{UserApi, UserApiImpl},
        window_procedures::{WindowApi, WindowApiImpl},
    },
    shared_types::{
        SharedCurrentConversation, SharedPromptKitService, create_shared_database_manager,
    },
};
use eur_timeline::TimelineManager;
use launcher::{monitor_cursor_for_hover, toggle_launcher_window};
use tauri::{
    AppHandle, Manager, Wry, generate_context,
    menu::{Menu, MenuItem},
    plugin::TauriPlugin,
    tray::TrayIconBuilder,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_updater::UpdaterExt;
use taurpc::Router;
use tracing::{error, info};

async fn update(app: tauri::AppHandle) -> tauri_plugin_updater::Result<()> {
    if let Some(update) = app.updater()?.check().await? {
        let mut downloaded = 0;

        // alternatively we could also call update.download() and update.install() separately
        update
            .download_and_install(
                |chunk_length, content_length| {
                    downloaded += chunk_length;
                    info!("downloaded {downloaded} from {content_length:?}");
                },
                || {
                    info!("download finished");
                },
            )
            .await?;

        info!("update installed");
        app.restart();
    }

    Ok(())
}

fn main() {
    dotenv().ok();

    #[cfg(not(debug_assertions))]
    {
        let _guard = sentry::init((
            "https://c274bba2ddbc19e4c2c34cedc1779588@o4508907847352320.ingest.de.sentry.io/4509796610605136",
            sentry::ClientOptions {
                release: sentry::release_name!(),
                send_default_pii: false,
                ..Default::default()
            },
        ));
    }

    // Regular application startup
    let tauri_context = generate_context!();

    info!("Starting Tauri application...");

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            info!("Setting tokio runtime");
            tauri::async_runtime::set(tokio::runtime::Handle::current());

            let builder = tauri::Builder::default()
                .plugin(tauri_plugin_os::init())
                .plugin(tauri_plugin_updater::Builder::new().build())
                .setup(move |tauri_app| {
                    let started_by_autostart = std::env::args().any(|arg| arg == "--startup-launch");

                    let app_settings = AppSettings::load_from_default_path_creating().unwrap();
                    tauri_app.manage(async_mutex::Mutex::new(app_settings.clone()));

                    // Ensure state exists immediately
                    tauri_app.manage::<SharedPromptKitService>(async_mutex::Mutex::new(None));

                    // Ensure empty current conversation exists
                    tauri_app.manage::<SharedCurrentConversation>(async_mutex::Mutex::new(None));

                    let handle = tauri_app.handle().clone();
                    tauri::async_runtime::spawn(async move {
                        if let Ok(prompt_kit_service) = app_settings.backend.initialize().await {
                            let service: SharedPromptKitService = async_mutex::Mutex::new(Some(prompt_kit_service));
                            handle.manage(service);
                        } else {
                            let service: SharedPromptKitService = async_mutex::Mutex::new(None);
                            handle.manage(service);
                            info!("No backend available");
                        }
                    });

                    // If no main key is available, generate a new one
                    let main_key = MainKey::new().expect("Failed to generate main key");

                    let handle = tauri_app.handle().clone();
                    tauri::async_runtime::spawn(async move {
                        update(handle).await.unwrap();
                    });


                    #[cfg(desktop)]
                    if app_settings.general.autostart && !started_by_autostart {
                        use tauri_plugin_autostart::MacosLauncher;
                        use tauri_plugin_autostart::ManagerExt;

                        let _ = tauri_app.handle().plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--startup-launch"]) /* arbitrary number of args to pass to your app */));

                        // Get the autostart manager
                        let autostart_manager = tauri_app.autolaunch();
                        // Enable autostart
                        let _ = autostart_manager.enable();
                        // Check enable state
                        info!("Autostart enabled: {}", autostart_manager.is_enabled().unwrap());
                    }

                    let main_window = create_window(tauri_app.handle(), "main", "".into())
                        .expect("Failed to create main window");

                    if started_by_autostart {
                        main_window.hide().expect("Failed to hide main window");
                    }


                    // Create launcher window without Arc<Mutex>
                    let launcher_window =
                        create_launcher(tauri_app.handle(), "launcher", "launcher".into())
                            .expect("Failed to create launcher window");

                        let hover_window = create_hover(tauri_app.handle(), "hover", "hover".into())
                            .expect("Failed to create hover window");

                        // Position hover window initially
                        util::position_hover_window(&hover_window);

                        // Start cursor monitoring for hover window
                        let hover_window_clone = hover_window.clone();
                        tauri::async_runtime::spawn(async move {
                            monitor_cursor_for_hover(hover_window_clone).await;
                        });

                    if !app_settings.hover.enabled {
                        hover_window.hide().expect("Failed to hide hover window");
                    }

                    let app_handle = tauri_app.handle();

                    let main_window_handle = app_handle.clone();
                    main_window.on_window_event(move |event| {
                        info!("Window event: {:?}", event);
                        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                            let main_window = main_window_handle.get_window("main").expect("Failed to get main window");
                            main_window.hide().expect("Failed to hide main window");
                            api.prevent_close();
                        }
                        if let tauri::WindowEvent::Focused(focused) = event {
                            let main_window = main_window_handle.get_window("main").expect("Failed to get main window");
                            let minimized = main_window.is_minimized().expect("Failed to get window state");
                            if !*focused && minimized {
                                main_window.hide().expect("Failed to hide main window");
                            }
                            info!("Window focused: {}", focused);
                        }
                    });


                    #[cfg(debug_assertions)]
                    {
                        // main_window.open_devtools();
                        // launcher_window.open_devtools();
                    }

                    // Ensure launcher is hidden on startup for Windows
                    #[cfg(target_os = "windows")]
                    {
                        launcher_window
                            .hide()
                            .expect("Failed to hide launcher window on startup");
                    }

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
                                let main_window = tray_icon_handle.get_window("main").expect("Failed to get main window");
                                main_window.unminimize().map_err(|e| error!("Failed to set window state: {}", e)).ok();
                                main_window.show().map_err(|e| error!("Failed to show main window: {}", e)).ok();
                            }
                        })
                        .build(tauri_app)
                        .expect("Failed to create tray icon");


                    let timeline = eur_timeline::TimelineManagerBuilder::new()
                    .with_activity_storage_config(
                        eur_activity::ActivityStorageConfig {
                        base_dir: app_handle.path().app_data_dir().unwrap(),
                        use_content_hash: false,
                        max_file_size: None,
                        main_key: main_key.clone()
                    })
                        .build().expect("Failed to create timeline");
                    app_handle.manage(async_mutex::Mutex::new(timeline));

                    // Start timeline collection
                    let timeline_handle = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        let timeline_mutex = timeline_handle.state::<async_mutex::Mutex<TimelineManager>>();
                        let mut timeline = timeline_mutex.lock().await;
                        if let Err(e) = timeline.start().await {
                            error!("Failed to start timeline collection: {}", e);
                        } else {
                            info!("Timeline collection started successfully");
                        }
                    });

                    let launcher_label = launcher_window.label().to_string();
                    app_handle.plugin(shortcut_plugin(launcher_label.clone()))?;

                    let app_handle_user = app_handle.clone();
                    let path = tauri_app.path().app_data_dir().unwrap();
                    tauri::async_runtime::spawn(async move {
                        let user_controller = eur_user::Controller::from_path(path)
                            .await
                            .map_err(|e| {
                                error!("Failed to create user controller: {}", e);
                                e
                            })
                            .unwrap();
                        app_handle_user.manage(user_controller);

                        // Register the initial global shortcut now that user controller is available
                        let launcher_shortcut = crate::util::convert_hotkey_to_shortcut(app_settings.launcher.hotkey.clone());

                        // Register the global shortcut
                        if let Err(e) = app_handle_user.global_shortcut().register(launcher_shortcut) {
                            error!("Failed to register initial launcher shortcut: {}", e);
                        } else {
                            info!("Successfully registered initial launcher shortcut: {:?}", launcher_shortcut);
                        }
                    });



                    // Initialize conversation storage
                    // let db_app_handle = app_handle.clone();
                    // tauri::async_runtime::spawn(async move {
                    //     let db = create_shared_database_manager(&db_app_handle).await;
                    //     db_app_handle.manage(db);
                    // });
                    // Initialize conversation storage
                    let db_app_handle = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        match create_shared_database_manager(&db_app_handle).await {
                            Ok(db) => {
                                db_app_handle.manage(db);
                                info!("Personal database manager initialized");
                            }
                            Err(e) => error!("Failed to initialize personal database manager: {}", e),
                        }
                    });


                    // Initialize IPC client
                    let ipc_handle = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        match create_grpc_ipc_client().await {
                            Ok(ipc_client) => {
                                ipc_handle.manage(ipc_client.clone());
                                info!("gRPC IPC client initialized");
                            }
                            Err(e) => error!("Failed to initialize gRPC IPC client: {}", e),
                        }
                    });

                    Ok(())
                })
                .plugin(tauri_plugin_http::init())
                .plugin(
                    tauri_plugin_log::Builder::new()
                            .filter(|metadata| metadata.target().starts_with("eur_") || metadata.level() == log::Level::Warn)
                            .level(log::LevelFilter::Info)
                            .build()
                )
                .plugin(tauri_plugin_shell::init())
                .plugin(tauri_plugin_single_instance::init(|_, _, _| {}))
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
                    tauri::WindowEvent::Focused(false) => {
                        // Handle launcher window focus loss for non-Linux OS
                        // #[cfg(not(target_os = "linux"))]
                        // {
                        //     // Check if this is the launcher window
                        //     if window.label() == "launcher" {
                        //         window.hide().expect("Failed to hide launcher window");
                        //         // Emit an event to clear the conversation when launcher is hidden
                        //         window
                        //             .emit("launcher_closed", ())
                        //             .expect("Failed to emit launcher_closed event");
                        //         set_launcher_visible(false);
                        //         // Ensure state is updated
                        //     }
                        // }
                    }

                    _ => {}
                });

            #[cfg(not(target_os = "linux"))]
            let builder = builder.plugin(tauri_plugin_window_state::Builder::default().build());
            // let typescript_config = specta_typescript::Typescript::default();
            // typescript_config
            //     .export_to("../../../bindings.ts", &specta::export())
            //     .unwrap();

            let router = Router::new()
                .export_config(
                    specta_typescript::Typescript::default()
                        .bigint(specta_typescript::BigIntExportBehavior::BigInt),
                )
                .merge(AuthApiImpl.into_handler())
                .merge(ConversationApiImpl.into_handler())
                .merge(SettingsApiImpl.into_handler())
                .merge(ThirdPartyApiImpl.into_handler())
                .merge(MonitorApiImpl.into_handler())
                .merge(MessageApiImpl.into_handler())
                .merge(SystemApiImpl.into_handler())
                .merge(ContextChipApiImpl.into_handler())
                .merge(PromptApiImpl.into_handler())
                .merge(WindowApiImpl.into_handler())
                .merge(ChatApiImpl.into_handler())
                .merge(UserApiImpl.into_handler());
            builder
                // .invoke_handler(tauri::generate_handler![list_conversations,])
                .invoke_handler(router.into_handler())
                .build(tauri_context)
                .expect("Failed to build tauri app")
                .run(|_app_handle, _event| {});
        });
}

fn shortcut_plugin(launcher_label: String) -> TauriPlugin<Wry> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(move |app: &AppHandle, _shortcut, event| {
            // Handle any registered shortcut - we'll validate it's a launcher shortcut
            // by checking if it matches the current user's launcher shortcut
            if ShortcutState::Pressed != event.state() {
                return;
            }
            let Some(launcher) = app.get_window(&launcher_label) else {
                return;
            };
            toggle_launcher_window(&launcher);
        })
        .build()
}
