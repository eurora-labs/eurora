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
use eur_native_messaging::create_grpc_ipc_client;
use eur_personal_db::{Conversation, DatabaseManager};
use eur_secret::secret;
use eur_tauri::{
    WindowState, create_launcher, create_window,
    procedures::{
        auth_procedures::{AuthApi, AuthApiImpl},
        context_chip_procedures::{ContextChipApi, ContextChipApiImpl},
        monitor_procedures::{MonitorApi, MonitorApiImpl},
        query_procedures::{QueryApi, QueryApiImpl},
        third_party_procedures::{ThirdPartyApi, ThirdPartyApiImpl},
        window_procedures::{WindowApi, WindowApiImpl},
    },
    shared_types::{SharedOpenAIClient, create_shared_timeline},
};
use eur_user::auth::AuthManager;
use eur_vision::{capture_focused_region_rgba, get_all_monitors, image_to_base64};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Emitter, Wry};
use tauri::{Manager, generate_context};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use taurpc::Router;
// Shared state to track if launcher is visible
static LAUNCHER_VISIBLE: AtomicBool = AtomicBool::new(false);

// mod focus_tracker;

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

// fn create_shared_conversation_storage() -> SharedConversationStorage {
//     Arc::new(async_mutex::Mutex::new(None))
// }

// And replace create_shared_client with this function:
fn create_shared_client() -> SharedQuestionsClient {
    Arc::new(Mutex::new(None))
}

fn create_shared_openai_client() -> SharedOpenAIClient {
    Arc::new(async_mutex::Mutex::new(None))
}

// fn create_shared_secret_service() -> SharedSecretService {
//     Arc::new(SecretService::new())
// }

fn get_db_path(app_handle: &tauri::AppHandle) -> String {
    let base_path = app_handle.path().app_data_dir().unwrap();
    std::fs::create_dir_all(&base_path).unwrap();
    let db_path = base_path.join("personal_database.sqlite");
    db_path.to_string_lossy().to_string()
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

    // info!("Starting Tauri application...");

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            // info!("Setting tokio runtime");
            tauri::async_runtime::set(tokio::runtime::Handle::current());

            let builder = tauri::Builder::default()
                .plugin(tauri_plugin_os::init())
                .plugin(tauri_plugin_updater::Builder::new().build())
                .setup(move |tauri_app| {
                    let _main_window =
                        create_window(tauri_app.handle(), "main", "onboarding".into())
                            // create_window(tauri_app.handle(), "main", "index.html".into())
                            .expect("Failed to create main window");

                    // Start the focus tracker

                    // Create launcher window without Arc<Mutex>
                    let launcher_window =
                        create_launcher(tauri_app.handle(), "launcher", "launcher".into())
                            .expect("Failed to create launcher window");

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

                    let app_handle = tauri_app.handle();

                    // --- State Initialization ---
                    let transcript_state = Arc::new(Mutex::new(None::<String>));
                    app_handle.manage(transcript_state);
                    let questions_client = create_shared_client();
                    app_handle.manage(questions_client.clone());
                    let timeline = create_shared_timeline();
                    app_handle.manage(timeline.clone());
                    let openai_client = create_shared_openai_client();
                    app_handle.manage(openai_client.clone());
                    let current_conversation_id = Arc::new(None::<String>);
                    app_handle.manage(current_conversation_id.clone());
                    // let current_conversation = create_shared_current_conversation();
                    // app_handle.manage(current_conversation);

                    let app_handle_auth = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        let auth_manager = AuthManager::new()
                            .await
                            .expect("AuthManager initialization failed");
                        app_handle_auth.manage(auth_manager);
                        info!("Auth manager initialized");
                    });

                    // Initialize OpenAI client if API key exists
                    let app_handle_openai = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        let api_key =
                            secret::retrieve("OPEN_AI_API_KEY", secret::Namespace::Global).unwrap();
                        if api_key.is_some() {
                            let client = eur_openai::OpenAI::with_api_key(&api_key.unwrap().0);
                            let state: tauri::State<SharedOpenAIClient> = app_handle_openai.state();
                            let mut guard = state.lock().await;
                            *guard = Some(client);
                            info!("OpenAI client initialized with API key from keyring");
                        } else {
                            info!("No API key found in keyring, OpenAI client not initialized");
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

                    // If macos, use Control + Space
                    #[cfg(target_os = "macos")]
                    {
                        let control_space_shortcut =
                            Shortcut::new(Some(Modifiers::ALT), Code::Space);
                        let launcher_label = launcher_window.label().to_string();

                        app_handle
                            .plugin(shortcut_plugin(control_space_shortcut, launcher_label))?;

                        app_handle
                            .global_shortcut()
                            .register(control_space_shortcut)?;
                    }

                    #[cfg(target_os = "windows")]
                    {
                        let super_space_shortcut =
                            Shortcut::new(Some(Modifiers::CONTROL), Code::Space);
                        let launcher_label = launcher_window.label().to_string();

                        app_handle.plugin(shortcut_plugin(
                            super_space_shortcut,
                            launcher_label.clone(),
                        ))?;

                        app_handle
                            .global_shortcut()
                            .register(super_space_shortcut)?;
                    }

                    #[cfg(target_os = "linux")]
                    {
                        let super_space_shortcut =
                            Shortcut::new(Some(Modifiers::SUPER), Code::Space);
                        let launcher_label = launcher_window.label().to_string();

                        app_handle.plugin(shortcut_plugin(
                            super_space_shortcut,
                            launcher_label.clone(),
                        ))?;

                        app_handle
                            .global_shortcut()
                            .register(super_space_shortcut)?;

                        // We'll use a different approach for Windows focus handling via on_window_event
                        // Keep the window-specific handler for Linux focus loss
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
                                        LAUNCHER_VISIBLE.store(false, Ordering::SeqCst);
                                        // Ensure state is updated
                                    }
                                }
                            });
                        }
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
                                LAUNCHER_VISIBLE.store(false, Ordering::SeqCst);
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
                .merge(ContextChipApiImpl.into_handler())
                .merge(WindowApiImpl.into_handler())
                .merge(QueryApiImpl.into_handler());
            builder
                .invoke_handler(tauri::generate_handler![list_conversations,])
                .invoke_handler(router.into_handler())
                .build(tauri_context)
                .expect("Failed to build tauri app")
                .run(|_app_handle, event| {
                    let _ = event;
                });
        });
}
fn shortcut_plugin(super_space_shortcut: Shortcut, launcher_label: String) -> TauriPlugin<Wry> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(move |app: &AppHandle, shortcut, event| {
            if shortcut != &super_space_shortcut {
                return;
            }
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
                LAUNCHER_VISIBLE.store(false, Ordering::SeqCst);
            } else {
                // Update the shared state to indicate launcher is visible
                LAUNCHER_VISIBLE.store(true, Ordering::SeqCst);

                // Variables to store launcher position and size
                let mut launcher_x = 0;
                let mut launcher_y = 0;
                let mut launcher_width = 1024; // Default width
                let mut launcher_height = 200; // Default height
                let mut monitor_id = "".to_string();
                let mut monitor_width = 1920u32; // Default monitor width
                let mut monitor_height = 1080u32; // Default monitor height

                // Get cursor position and center launcher on that screen
                if let Ok(cursor_position) = launcher.cursor_position() {
                    if let Ok(monitors) = get_all_monitors() {
                        for monitor in monitors {
                            monitor_id = monitor.id().unwrap().to_string();
                            monitor_width = monitor.width().unwrap();
                            monitor_height = monitor.height().unwrap();
                            let monitor_x = monitor.x().unwrap();
                            let monitor_y = monitor.y().unwrap();
                            let scale_factor = monitor.scale_factor().unwrap() as f64;

                            info!("Monitor width: {:?}", monitor_width);
                            info!("Monitor height: {:?}", monitor_height);
                            info!("Monitor x: {:?}", monitor_x);
                            info!("Monitor y: {:?}", monitor_y);
                            info!("Monitor scale factor: {:?}", scale_factor);

                            // Check if cursor is on this monitor
                            if cursor_position.x >= monitor_x as f64
                                && cursor_position.x <= (monitor_x + monitor_width as i32) as f64
                                && cursor_position.y >= monitor_y as f64
                                && cursor_position.y <= (monitor_y + monitor_height as i32) as f64
                            {
                                // Center the launcher on this monitor
                                let window_size = launcher.inner_size().unwrap();

                                info!("Window size: {:?}", window_size);

                                launcher_x = monitor_x
                                    + (monitor_width as i32 - window_size.width as i32) / 2;
                                launcher_y = monitor_y
                                    + (monitor_height as i32 - window_size.height as i32) / 4;

                                info!("Launcher position: ({}, {})", launcher_x, launcher_y);

                                launcher
                                    .set_position(tauri::Position::Physical(
                                        tauri::PhysicalPosition {
                                            x: launcher_x,
                                            y: launcher_y,
                                            // x: 0,
                                            // y: 0,
                                        },
                                    ))
                                    .expect("Failed to set launcher position");

                                launcher_x = ((monitor_width as i32 as f64) / 2.0) as i32
                                    - (window_size.width as f64 / 2.0) as i32;
                                launcher_y = ((monitor_height as i32 as f64) / 4.0) as i32
                                    - (window_size.height as f64 / 4.0) as i32;
                                launcher_width = window_size.width;
                                launcher_height = window_size.height;
                                break;
                            }
                        }
                    }
                }
                let start_record = std::time::Instant::now();
                // Capture the screen region behind the launcher
                match capture_focused_region_rgba(
                    monitor_id.clone(),
                    launcher_x as u32,
                    launcher_y as u32,
                    launcher_width,
                    launcher_height,
                ) {
                    Ok(img) => {
                        let t0 = std::time::Instant::now();
                        let img = match cfg!(target_os = "linux") {
                            true => pollster::block_on(eur_renderer::blur_image(&img, 0.1, 36.0)),
                            false => image::DynamicImage::ImageRgba8(img.clone()).to_rgb8(),
                        };

                        info!("Captured image size: {:?}", img.dimensions());
                        let duration = t0.elapsed();
                        info!("Capture of background area completed in: {:?}", duration);

                        // Convert the image to base64
                        if let Ok(base64_image) = image_to_base64(img) {
                            // Send the base64 image to the frontend
                            launcher
                                .emit("background_image", base64_image)
                                .expect("Failed to emit background_image event");
                        }
                    }
                    Err(e) => {
                        error!("Failed to capture screen region: {}", e);
                    }
                }
                let duration = start_record.elapsed();
                info!("Capture of background area completed in: {:?}", duration);

                // Only show the launcher if it was previously hidden
                launcher.show().expect("Failed to show launcher window");

                // Emit an event to notify that the launcher has been opened
                // Include positioning information for proper background alignment
                let launcher_info = serde_json::json!({
                    "monitor_id": monitor_id.clone(),
                    "launcher_x": launcher_x,
                    "launcher_y": launcher_y,
                    "launcher_width": launcher_width,
                    "launcher_height": launcher_height,
                    "monitor_width": monitor_width,
                    "monitor_height": monitor_height
                });
                launcher
                    .emit("launcher_opened", launcher_info)
                    .expect("Failed to emit launcher_opened event");

                launcher
                    .set_focus()
                    .expect("Failed to focus launcher window");
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
