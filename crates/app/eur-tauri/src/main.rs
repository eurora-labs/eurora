#![cfg_attr(
    all(windows, not(test), not(debug_assertions)),
    windows_subsystem = "windows"
)]

use anyhow::Result;
use dotenv::dotenv;
use eur_client_questions::QuestionsClient;
use tracing_subscriber::{
    filter::{EnvFilter, LevelFilter},
    fmt,
};
// use eur_conversation::{ChatMessage, Conversation, ConversationStorage};
mod launcher;
mod util;
use std::sync::{Arc, Mutex};

use eur_native_messaging::create_grpc_ipc_client;
use eur_personal_db::{Conversation, DatabaseManager};
use eur_prompt_kit::PromptKitService;
use eur_secret::secret;
use eur_tauri::{
    WindowState, create_hover, create_launcher, create_window,
    procedures::{
        auth_procedures::{AuthApi, AuthApiImpl},
        chat_procedures::{ChatApi, ChatApiImpl},
        context_chip_procedures::{ContextChipApi, ContextChipApiImpl},
        monitor_procedures::{MonitorApi, MonitorApiImpl},
        prompt_procedures::{PromptApi, PromptApiImpl},
        system_procedures::{SystemApi, SystemApiImpl},
        third_party_procedures::{ThirdPartyApi, ThirdPartyApiImpl},
        user_procedures::{UserApi, UserApiImpl},
        window_procedures::{WindowApi, WindowApiImpl},
    },
    shared_types::{SharedPromptKitService, create_shared_timeline},
};
use launcher::{
    monitor_cursor_for_hover, open_launcher_window, position_hover_window, set_launcher_visible,
};
use tauri::{
    AppHandle, Emitter, Manager, Wry, generate_context,
    menu::{Menu, MenuItem},
    plugin::TauriPlugin,
    tray::TrayIconBuilder,
};
use tauri_plugin_global_shortcut::ShortcutState;
use tauri_plugin_updater::UpdaterExt;
use taurpc::Router;
use tracing::{error, info};

type SharedQuestionsClient = Arc<Mutex<Option<QuestionsClient>>>;
type SharedPersonalDb = Arc<DatabaseManager>;

async fn create_shared_database_manager(app_handle: &tauri::AppHandle) -> SharedPersonalDb {
    let db_path = get_db_path(app_handle);
    Arc::new(
        DatabaseManager::new(&db_path)
            .await
            .map_err(|e| {
                info!("Failed to create database manager: {}", e);
                e
            })
            .unwrap(),
    )
}

fn create_shared_client() -> SharedQuestionsClient {
    Arc::new(Mutex::new(None))
}

fn create_shared_promptkit_client() -> SharedPromptKitService {
    Arc::new(async_mutex::Mutex::new(None))
}

fn get_db_path(app_handle: &tauri::AppHandle) -> String {
    let base_path = app_handle.path().app_data_dir().unwrap();
    std::fs::create_dir_all(&base_path).unwrap();
    let db_path = base_path.join("personal_database.sqlite");
    db_path.to_string_lossy().to_string()
}

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
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::WARN.into()) // anything not listed → WARN
        .parse_lossy("eur_=trace,hyper=off,tokio=off"); // keep yours, silence deps

    fmt().with_env_filter(filter).init();
    // let _guard = sentry::init((
    //     "https://5181d08d2bfcb209a768ab99e1e48f1b@o4508907847352320.ingest.de.sentry.io/4508907850694736",
    //     sentry::ClientOptions {
    //         release: sentry::release_name!(),
    //         ..Default::default()
    //     },
    // ));

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
                    let handle = tauri_app.handle().clone();
                    tauri::async_runtime::spawn(async move {
                        update(handle).await.unwrap();
                    });

                    #[cfg(desktop)]
                    {
                        use tauri_plugin_autostart::MacosLauncher;
                        use tauri_plugin_autostart::ManagerExt;

                        let _ = tauri_app.handle().plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--flag1", "--flag2"]) /* arbitrary number of args to pass to your app */));

                        // Get the autostart manager
                        let autostart_manager = tauri_app.autolaunch();
                        // Enable autostart
                        let _ = autostart_manager.enable();
                        // Check enable state
                        info!("Autostart enabled: {}", autostart_manager.is_enabled().unwrap());
                    }
                    let main_window = create_window(tauri_app.handle(), "main", "".into())
                        .expect("Failed to create main window");

                    // Create launcher window without Arc<Mutex>
                    let launcher_window =
                        create_launcher(tauri_app.handle(), "launcher", "launcher".into())
                            .expect("Failed to create launcher window");

                    let hover_window = create_hover(tauri_app.handle(), "hover", "hover".into())
                        .expect("Failed to create hover window");

                    // Position hover window initially
                    position_hover_window(&hover_window);

                    // Start cursor monitoring for hover window
                    let hover_window_clone = hover_window.clone();
                    tauri::async_runtime::spawn(async move {
                        monitor_cursor_for_hover(hover_window_clone).await;
                    });

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
                                main_window.unminimize().expect("Failed to set window state");
                                main_window.show().expect("Failed to show main window");
                            }
                        })
                        .build(tauri_app)
                        .expect("Failed to create tray icon");


                    // --- State Initialization ---
                    let transcript_state = Arc::new(Mutex::new(None::<String>));
                    app_handle.manage(transcript_state);
                    let questions_client = create_shared_client();
                    app_handle.manage(questions_client.clone());
                    let timeline = create_shared_timeline();
                    app_handle.manage(timeline.clone());
                    let promptkit_client = create_shared_promptkit_client();
                    app_handle.manage(promptkit_client.clone());
                    let current_conversation_id = Arc::new(None::<String>);
                    app_handle.manage(current_conversation_id.clone());

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
                    });

                    // Initialize OpenAI client if API key exists
                    let app_handle_openai = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        let api_key =
                            secret::retrieve("OPENAI_API_KEY", secret::Namespace::Global).unwrap();
                        if api_key.is_some() {
                            let prompt_kit_service = PromptKitService::default();

                            let state: tauri::State<SharedPromptKitService> =
                                app_handle_openai.state();
                            let mut guard = state.lock().await;
                            *guard = Some(prompt_kit_service);
                            info!("PromptKitService initialized with API key from keyring");
                        } else {
                            info!("No API key found in keyring, PromptKitService not initialized");
                        }
                    });

                    // Initialize conversation storage
                    let _db_path = get_db_path(app_handle);
                    let db_app_handle = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        let db = create_shared_database_manager(&db_app_handle).await;
                        db_app_handle.manage(db.clone());
                    });

                    // Start timeline collection
                    let timeline_clone = timeline.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(e) = timeline_clone.start_collection().await {
                            error!("Failed to start timeline collection: {}", e);
                        } else {
                            info!("Timeline collection started successfully");
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

                    // info!("Setting up global shortcut");

                    // Get the launcher shortcut from user settings or use default
                    // let launcher_shortcut = get_launcher_shortcut(app_handle);
                    let launcher_label = launcher_window.label().to_string();

                    // Register the shortcut plugin
                    app_handle.plugin(shortcut_plugin(launcher_label.clone()))?;

                    // Register the global shortcut
                    // app_handle.global_shortcut().register(launcher_shortcut)?;

                    // Linux-specific focus handling
                    #[cfg(target_os = "linux")]
                    {
                        let app_handle_focus = app_handle.clone();
                        let launcher_label_linux = launcher_label.clone();
                        launcher_window.on_window_event(move |event| {
                            if let tauri::WindowEvent::Focused(false) = event {
                                if let Some(launcher) =
                                    app_handle_focus.get_window(&launcher_label_linux)
                                {
                                    launcher.hide().expect("Failed to hide launcher window");
                                    // Emit an event to clear the conversation when launcher is hidden
                                    launcher
                                        .emit("launcher_closed", ())
                                        .expect("Failed to emit launcher_closed event");
                                    set_launcher_visible(false);
                                    // Ensure state is updated
                                }
                            }
                        });
                    }

                    Ok(())
                })
                .plugin(tauri_plugin_http::init())
                // .plugin(
                //     tauri_plugin_log::Builder::default()
                //         .level(log::LevelFilter::Error)
                //         .build(),
                // )
                .plugin(tauri_plugin_shell::init())
                .plugin(tauri_plugin_single_instance::init(|_, _, _| {}))
                // .plugin(
                //     tauri_plugin_log::Builder::default()
                //         .targets([
                //             Target::new(TargetKind::Stdout),
                //             Target::new(TargetKind::LogDir { file_name: None }),
                //         ])
                //         .build(),
                // )
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
                        #[cfg(not(target_os = "linux"))]
                        {
                            // Check if this is the launcher window
                            if window.label() == "launcher" {
                                window.hide().expect("Failed to hide launcher window");
                                // Emit an event to clear the conversation when launcher is hidden
                                window
                                    .emit("launcher_closed", ())
                                    .expect("Failed to emit launcher_closed event");
                                set_launcher_visible(false);
                                // Ensure state is updated
                            }
                        }
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
                .merge(ThirdPartyApiImpl.into_handler())
                .merge(MonitorApiImpl.into_handler())
                .merge(SystemApiImpl.into_handler())
                .merge(ContextChipApiImpl.into_handler())
                .merge(PromptApiImpl.into_handler())
                .merge(WindowApiImpl.into_handler())
                .merge(ChatApiImpl.into_handler())
                .merge(UserApiImpl.into_handler());
            builder
                .invoke_handler(tauri::generate_handler![list_conversations,])
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
            let Ok(is_visible) = launcher.is_visible() else {
                return;
            };

            if is_visible {
                // Hide the launcher window and emit the closed event
                launcher.hide().expect("Failed to hide launcher window");
                launcher
                    .emit("launcher_closed", ())
                    .expect("Failed to emit launcher_closed event");

                // Update the shared state to indicate launcher is hidden
                set_launcher_visible(false);
            } else {
                // Use the extracted launcher opening function
                if let Err(e) = open_launcher_window(&launcher) {
                    error!("Failed to open launcher window: {}", e);
                }
            }
        })
        .build()
}

#[tauri::command]
async fn list_conversations(app_handle: tauri::AppHandle) -> Result<Vec<Conversation>, String> {
    let db = app_handle.state::<SharedPersonalDb>().clone();
    let conversations = db.list_conversations().await.unwrap();
    Ok(conversations)
}
