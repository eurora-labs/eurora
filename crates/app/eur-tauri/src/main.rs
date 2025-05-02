#![cfg_attr(
    all(windows, not(test), not(debug_assertions)),
    windows_subsystem = "windows"
)]

use anyhow::{Context, Result};
use chrono::Utc;
use eur_client_questions::QuestionsClient;
// use eur_conversation::{ChatMessage, Conversation, ConversationStorage};
use eur_native_messaging::create_grpc_ipc_client;
use eur_personal_db::{ChatMessage, Conversation, DatabaseManager};
use eur_proto::ipc::{ProtoArticleState, ProtoPdfState, ProtoYoutubeState};
use eur_proto::questions_service::ProtoChatMessage;
use eur_tauri::{WindowState, create_launcher};
use eur_timeline::Timeline;
use eur_vision::{capture_region_rgba, image_to_base64};
use futures::{StreamExt, TryFutureExt};
// use secret_service::{ApiKeyStatus, SecretService};
use eur_secret::Sensitive;
use eur_secret::secret;
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::ipc::Channel;
use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Emitter, Wry};
use tauri::{Manager, generate_context};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use std::sync::atomic::{AtomicBool, Ordering};

// Shared state to track if launcher is visible
static LAUNCHER_VISIBLE: AtomicBool = AtomicBool::new(false);

// mod focus_tracker;

use tracing::{error, info};
type SharedTimeline = Arc<Timeline>;
type SharedQuestionsClient = Arc<Mutex<Option<QuestionsClient>>>;
type SharedOpenAIClient = Arc<async_mutex::Mutex<Option<eur_openai::OpenAI>>>;
type SharedPersonalDb = Arc<DatabaseManager>;
type SharedCurrentConversation = Arc<Option<Conversation>>;
type SharedCurrentConversationId = Arc<String>;
// type SharedSecretService = Arc<SecretService>;

async fn create_shared_database_manager(app_handle: &tauri::AppHandle) -> SharedPersonalDb {
    let db_path = get_db_path(app_handle);
    Arc::new(
        DatabaseManager::new(&db_path)
            .await
            .map_err(|e| {
                eprintln!("Failed to create database manager: {}", e);
                e
            })
            .unwrap(),
    )
}

// fn create_shared_conversation_storage() -> SharedConversationStorage {
//     Arc::new(async_mutex::Mutex::new(None))
// }

fn create_shared_current_conversation() -> SharedCurrentConversation {
    Arc::new(None)
}

// And replace create_shared_client with this function:
fn create_shared_client() -> SharedQuestionsClient {
    Arc::new(Mutex::new(None))
}

fn create_shared_timeline() -> SharedTimeline {
    // Create a timeline that collects state every 3 seconds and keeps 1 hour of history
    Arc::new(eur_timeline::create_default_timeline())
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

#[tauri::command]
async fn resize_launcher_window(window: tauri::Window, height: u32) -> Result<(), String> {
    if let Some(launcher) = window.get_window("launcher") {
        let _ = launcher.set_size(tauri::Size::Logical(tauri::LogicalSize {
            width: 1024.0,
            height: height as f64,
        }));
    }
    Ok(())
}

#[tauri::command]
async fn check_api_key_exists() -> Result<String, String> {
    let key = secret::retrieve("OPEN_AI_API_KEY", secret::Namespace::BuildKind)
        .map_err(|e| format!("Failed to retrieve API key: {}", e))?;

    let key = key.map(|s| s.0).unwrap_or_default();
    if key.is_empty() {
        return Err("API key not found".to_string());
    }

    Ok(key)
}

#[tauri::command]
async fn save_api_key(api_key: String) -> Result<(), String> {
    secret::persist(
        "OPEN_AI_API_KEY",
        &Sensitive(api_key),
        secret::Namespace::BuildKind,
    )
    .map_err(|e| format!("Failed to save API key: {}", e))?;
    Ok(())
}

#[tauri::command]
async fn initialize_openai_client(app_handle: tauri::AppHandle) -> Result<bool, String> {
    let api_key = secret::retrieve("OPEN_AI_API_KEY", secret::Namespace::BuildKind)
        .map_err(|e| format!("Failed to retrieve API key: {}", e))?;

    // Initialize the OpenAI client with the API key
    let openai_client = eur_openai::OpenAI::with_api_key(&api_key.unwrap().0);

    // Store the client in the app state
    let state: tauri::State<SharedOpenAIClient> = app_handle.state();
    let mut guard = state.lock().await;
    *guard = Some(openai_client);

    Ok(true)
}

fn main() {
    let _guard = sentry::init((
        "https://5181d08d2bfcb209a768ab99e1e48f1b@o4508907847352320.ingest.de.sentry.io/4508907850694736",
        sentry::ClientOptions {
            release: sentry::release_name!(),
            ..Default::default()
        },
    ));

    // Regular application startup
    let tauri_context = generate_context!();

    // eprintln!("Starting Tauri application...");

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            // println!("Setting tokio runtime");
            tauri::async_runtime::set(tokio::runtime::Handle::current());

            let builder = tauri::Builder::default()
                .plugin(tauri_plugin_updater::Builder::new().build())
                .setup(move |tauri_app| {
                    // let main_window =
                    //     create_window(tauri_app.handle(), "main", "index.html".into())
                    //         .expect("Failed to create main window");

                    // Start the focus tracker

                    // Create launcher window without Arc<Mutex>
                    let launcher_window =
                        create_launcher(tauri_app.handle(), "launcher", "index.html".into())
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

                    // --- Background Tasks ---

                    // Initialize OpenAI client if API key exists
                    let app_handle_openai = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        let api_key =
                            secret::retrieve("OPEN_AI_API_KEY", secret::Namespace::BuildKind)
                                .unwrap();
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
                    let db_path = get_db_path(app_handle);
                    let db_app_handle = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        match DatabaseManager::new(&db_path).await {
                            Ok(db_manager) => {
                                db_app_handle
                                    .manage(Arc::new(db_manager).clone() as SharedPersonalDb);
                                info!("Database manager initialized successfully");
                            }
                            Err(e) => error!("Failed to initialize database manager: {}", e),
                        }
                    });
                    // let storage_init_handle = conversation_storage.clone();
                    // tauri::async_runtime::spawn(async move {
                    //     match ConversationStorage::new(db_path) {
                    //         Ok(storage) => {
                    //             storage_init_handle.lock().await.replace(storage);
                    //             info!("Conversation storage initialized successfully");
                    //         }
                    //         Err(e) => error!("Failed to initialize conversation storage: {}", e),
                    //     }
                    // });

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

                    // println!("Setting up global shortcut");

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

                    #[cfg(any(target_os = "linux", target_os = "windows"))]
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

            builder
                .invoke_handler(tauri::generate_handler![
                    resize_launcher_window,
                    ask_video_question,
                    continue_conversation,
                    check_grpc_server_connection,
                    list_activities,
                    get_current_conversation,
                    switch_conversation,
                    get_conversation,
                    list_conversations,
                    check_api_key_exists,
                    save_api_key,
                    initialize_openai_client,
                    send_key_to_launcher, // Keep for potential testing/manual trigger
                ])
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
                let mut launcher_height = 500; // Default height

                // Get cursor position and center launcher on that screen
                if let Ok(cursor_position) = launcher.cursor_position() {
                    if let Ok(monitors) = launcher.available_monitors() {
                        for monitor in monitors {
                            monitor.name();
                            let size = monitor.size();
                            let position = monitor.position();
                            let scale_factor = monitor.scale_factor();

                            eprintln!("Monitor size: {:?}", size);
                            eprintln!("Monitor position: {:?}", position);
                            eprintln!("Monitor scale factor: {:?}", scale_factor);

                            // Check if cursor is on this monitor
                            if cursor_position.x >= position.x as f64
                                && cursor_position.x <= (position.x + size.width as i32) as f64
                                && cursor_position.y >= position.y as f64
                                && cursor_position.y <= (position.y + size.height as i32) as f64
                            {
                                // Center the launcher on this monitor
                                let window_size = launcher.inner_size().unwrap();

                                eprintln!("Window size: {:?}", window_size);

                                launcher_x =
                                    position.x + (size.width as i32 - window_size.width as i32) / 2;
                                launcher_y = position.y
                                    + (size.height as i32 - window_size.height as i32) / 4;

                                eprintln!("Launcher position: ({}, {})", launcher_x, launcher_y);

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

                                launcher_x = ((size.width as i32 as f64) / 2.0) as i32
                                    - (window_size.width as f64 / 2.0) as i32;
                                launcher_y = ((size.height as i32 as f64) / 4.0) as i32
                                    - (window_size.height as f64 / 4.0) as i32;
                                // - (window_size.height as f64 / 4.0) as i32;
                                launcher_width = window_size.width;
                                launcher_height = window_size.height;
                                break;
                            }
                        }
                    }
                }
                let start_record = std::time::Instant::now();
                // Capture the screen region behind the launcher
                match capture_region_rgba(
                    launcher_x,
                    launcher_y,
                    launcher_width,
                    launcher_height + 200,
                ) {
                    Ok(img) => {
                        let t0 = std::time::Instant::now();
                        // let img = image::DynamicImage::ImageRgba8(img.clone()).to_rgb8();

                        eprintln!("Captured image size: {:?}", img.dimensions());
                        let img = pollster::block_on(eur_renderer::blur_image(&img, 0.2, 36.0));
                        let duration = t0.elapsed();
                        println!("Capture of background area completed in: {:?}", duration);

                        // // Convert the image to base64
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
                println!("Capture of background area completed in: {:?}", duration);

                // Only show the launcher if it was previously hidden
                launcher.show().expect("Failed to show launcher window");

                // Emit an event to notify that the launcher has been opened
                launcher
                    .emit("launcher_opened", ())
                    .expect("Failed to emit launcher_opened event");

                launcher
                    .set_focus()
                    .expect("Failed to focus launcher window");
            }
        })
        .build()
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
enum DownloadEvent<'a> {
    #[serde(rename_all = "camelCase")]
    Message { message: &'a str },
    #[serde(rename_all = "camelCase")]
    Append { chunk: &'a str },
}

#[tauri::command]
async fn continue_conversation(
    app_handle: tauri::AppHandle,
    conversation_id: String,
    question: String,
    channel: Channel<DownloadEvent<'_>>,
) -> Result<(), String> {
    eprintln!("Continuing conversation: {}", conversation_id);
    eprintln!("Asking question: {}", question);

    let db = app_handle.state::<SharedPersonalDb>().clone();

    db.insert_chat_message(
        &conversation_id,
        "user",
        &question,
        true,
        Utc::now(),
        Utc::now(),
    );

    let chat_messages = db
        .get_chat_messages(&conversation_id)
        .await
        .map_err(|e| format!("Failed to get previous messages: {}", e))
        .unwrap();

    // Get the OpenAI client
    let state: tauri::State<SharedOpenAIClient> = app_handle.state();
    let mut guard = state.lock().await;
    let client = guard
        .as_mut()
        .ok_or_else(|| "OpenAI client not initialized".to_string())?;

    let messages = chat_messages
        .iter()
        .map(|msg| {
            let mut role = eur_prompt_kit::Role::System;
            if msg.role == "user" {
                role = eur_prompt_kit::Role::User;
            }

            eur_prompt_kit::LLMMessage {
                role,
                content: eur_prompt_kit::MessageContent::Text(eur_prompt_kit::TextContent {
                    text: msg.content.clone(),
                }),
            }
        })
        .collect();

    let mut stream = client.video_question(messages).await?;
    channel
        .send(DownloadEvent::Message { message: "" })
        .map_err(|e| format!("Failed to send response: {e}"))?;
    // Collect the complete response
    let mut complete_response = String::new();

    while let Some(Ok(chunk)) = stream.next().await {
        for message in chunk.choices {
            let Some(message) = message.delta.content else {
                continue;
            };
            // Append to the complete response
            complete_response.push_str(&message);

            channel
                .send(DownloadEvent::Append { chunk: &message })
                .map_err(|e| format!("Failed to send response: {e}"))?;
        }
    }

    // After the stream ends, add the complete response as a ChatMessage to the conversation
    if !complete_response.is_empty() {
        db.insert_chat_message(
            &conversation_id,
            "SYSTEM",
            &complete_response,
            true,
            Utc::now(),
            Utc::now(),
        );
        eprintln!(
            "Added assistant response to conversation {}",
            conversation_id
        );
    }

    Ok(())

    // // Process based on asset type
    // if asset.asset_type == "youtube" {
    //     // Deserialize the YouTube state from the asset content
    //     let youtube_state: ProtoYoutubeState =
    //         serde_json::from_value(asset.content.get("Youtube").unwrap().clone())
    //             .map_err(|e| format!("Failed to deserialize YouTube state: {}", e))?;

    //     // Get all messages from the conversation
    //     let messages: Vec<ProtoChatMessage> = conversation
    //         .messages
    //         .iter()
    //         .map(|msg| ProtoChatMessage {
    //             role: msg.role.clone(),
    //             content: msg.content.clone(),
    //         })
    //         .collect();

    //     // Collect the complete response
    //     let mut complete_response = String::new();

    //     // Send the question to the OpenAI API
    //     let mut stream = client.video_question_old(messages, youtube_state).await?;

    //     channel
    //         .send(DownloadEvent::Message { message: "" })
    //         .map_err(|e| format!("Failed to send response: {e}"))?;

    //     while let Some(Ok(chunk)) = stream.next().await {
    //         for message in chunk.choices {
    //             let Some(message) = message.delta.content else {
    //                 continue;
    //             };
    //             // Append to the complete response
    //             complete_response.push_str(&message);

    //             channel
    //                 .send(DownloadEvent::Append { chunk: &message })
    //                 .map_err(|e| format!("Failed to send response: {e}"))?;
    //         }
    //     }

    //     // After the stream ends, add the complete response as a ChatMessage to the conversation
    //     if !complete_response.is_empty() {
    //         // Create a new ChatMessage with the assistant's response
    //         let chat_message =
    //             ChatMessage::new(None, "assistant".to_string(), complete_response, true);

    //         // Add the message to the conversation and save it
    //         let mut updated_conversation = conversation.clone();
    //         updated_conversation
    //             .add_message(chat_message)
    //             .map_err(|e| e.to_string())?;
    //         storage.save_conversation(&updated_conversation).unwrap();

    //         eprintln!(
    //             "Added assistant response to conversation {}",
    //             updated_conversation.id
    //         );
    //     }

    //     Ok(())
    // } else if asset.asset_type == "article" {
    //     // Deserialize the Article state from the asset content
    //     let article_state: ProtoArticleState =
    //         serde_json::from_value(asset.content.get("Article").unwrap().clone())
    //             .map_err(|e| format!("Failed to deserialize Article state: {}", e))?;

    //     // Get all messages from the conversation
    //     let messages: Vec<ProtoChatMessage> = conversation
    //         .messages
    //         .iter()
    //         .map(|msg| ProtoChatMessage {
    //             role: msg.role.clone(),
    //             content: msg.content.clone(),
    //         })
    //         .collect();

    //     // Collect the complete response
    //     let mut complete_response = String::new();

    //     // Send the question to the OpenAI API
    //     let mut stream = client.article_question(messages, article_state).await?;

    //     channel
    //         .send(DownloadEvent::Message { message: "" })
    //         .map_err(|e| format!("Failed to send response: {e}"))?;

    //     while let Some(Ok(chunk)) = stream.next().await {
    //         for message in chunk.choices {
    //             let Some(message) = message.delta.content else {
    //                 continue;
    //             };
    //             // Append to the complete response
    //             complete_response.push_str(&message);

    //             channel
    //                 .send(DownloadEvent::Append { chunk: &message })
    //                 .map_err(|e| format!("Failed to send response: {e}"))?;
    //         }
    //     }

    //     // After the stream ends, add the complete response as a ChatMessage to the conversation
    //     if !complete_response.is_empty() {
    //         // Create a new ChatMessage with the assistant's response
    //         let chat_message =
    //             ChatMessage::new(None, "assistant".to_string(), complete_response, true);

    //         // Add the message to the conversation and save it
    //         let mut updated_conversation = conversation.clone();
    //         updated_conversation
    //             .add_message(chat_message)
    //             .map_err(|e| e.to_string())?;
    //         storage.save_conversation(&updated_conversation).unwrap();

    //         eprintln!(
    //             "Added assistant response to conversation {}",
    //             updated_conversation.id
    //         );
    //     }

    //     Ok(())
    // } else if asset.asset_type == "pdf" {
    //     // Deserialize the PDF state from the asset content
    //     let pdf_state: ProtoPdfState =
    //         serde_json::from_value(asset.content.get("Pdf").unwrap().clone())
    //             .map_err(|e| format!("Failed to deserialize PDF state: {}", e))?;

    //     // Get all messages from the conversation
    //     let messages: Vec<ProtoChatMessage> = conversation
    //         .messages
    //         .iter()
    //         .map(|msg| ProtoChatMessage {
    //             role: msg.role.clone(),
    //             content: msg.content.clone(),
    //         })
    //         .collect();

    //     // Collect the complete response
    //     let mut complete_response = String::new();

    //     // Send the question to the OpenAI API
    //     let mut stream = client.pdf_question(messages, pdf_state).await?;

    //     channel
    //         .send(DownloadEvent::Message { message: "" })
    //         .map_err(|e| format!("Failed to send response: {e}"))?;

    //     while let Some(Ok(chunk)) = stream.next().await {
    //         for message in chunk.choices {
    //             let Some(message) = message.delta.content else {
    //                 continue;
    //             };
    //             // Append to the complete response
    //             complete_response.push_str(&message);

    //             channel
    //                 .send(DownloadEvent::Append { chunk: &message })
    //                 .map_err(|e| format!("Failed to send response: {e}"))?;
    //         }
    //     }

    //     // After the stream ends, add the complete response as a ChatMessage to the conversation
    //     if !complete_response.is_empty() {
    //         // Create a new ChatMessage with the assistant's response
    //         let chat_message =
    //             ChatMessage::new(None, "assistant".to_string(), complete_response, true);

    //         // Add the message to the conversation and save it
    //         let mut updated_conversation = conversation.clone();
    //         updated_conversation
    //             .add_message(chat_message)
    //             .map_err(|e| e.to_string())?;
    //         storage.save_conversation(&updated_conversation).unwrap();

    //         eprintln!(
    //             "Added assistant response to conversation {}",
    //             updated_conversation.id
    //         );
    //     }

    //     Ok(())
    // } else {
    //     Err(format!(
    //         "Asset type {} not yet implemented",
    //         asset.asset_type
    //     ))
    // }
}

// Tauri command to ask questions about content (video or article) via gRPC
#[tauri::command]
async fn ask_video_question(
    app_handle: tauri::AppHandle,
    mut conversation_id: String,
    question: String,
    channel: Channel<DownloadEvent<'_>>,
) -> Result<String, String> {
    eprintln!("Asking question: {}", question);
    eprintln!("Conversation ID: {}", conversation_id);

    let db = app_handle.state::<SharedPersonalDb>().clone();

    // Get the timeline from app state
    let timeline_state: tauri::State<SharedTimeline> = app_handle.state();
    let timeline = timeline_state.inner();

    let title: String = "Placeholder Title".to_string();

    let mut messages = timeline.construct_asset_messages();

    messages.push(eur_prompt_kit::LLMMessage {
        role: eur_prompt_kit::Role::User,
        content: eur_prompt_kit::MessageContent::Text(eur_prompt_kit::TextContent {
            text: question.clone(),
        }),
    });

    let state: tauri::State<SharedOpenAIClient> = app_handle.state();
    let mut guard = state.lock().await;
    let client = guard
        .as_mut()
        .ok_or_else(|| "OpenAI client not initialized".to_string())?;

    if conversation_id == "NEW" {
        // Create new conversation and store it in SQLite
        eprintln!("Creating new conversation with title: {}", title);

        let conversation = db
            .insert_conversation(&title, Utc::now(), Utc::now())
            .await
            .map_err(|e| format!("Failed to insert conversation: {}", e))?;

        conversation_id = conversation.id.clone();

        db.insert_chat_message(
            &conversation_id,
            "USER",
            &question,
            true,
            Utc::now(),
            Utc::now(),
        )
        .await
        .map_err(|e| format!("Failed to insert chat message: {}", e))?;
    }

    let mut complete_response = String::new();

    let mut stream = client.video_question(messages).await?;

    channel
        .send(DownloadEvent::Message { message: "" })
        .map_err(|e| format!("Failed to send response: {e}"))?;

    while let Some(Ok(chunk)) = stream.next().await {
        for message in chunk.choices {
            let Some(message) = message.delta.content else {
                continue;
            };
            // Append to the complete response
            complete_response.push_str(&message);

            channel
                .send(DownloadEvent::Append { chunk: &message })
                .map_err(|e| format!("Failed to send response: {e}"))?;
        }
    }

    // After the stream ends, add the complete response as a ChatMessage to the conversation
    if !complete_response.is_empty() {
        db.insert_chat_message(
            &conversation_id,
            "SYSTEM",
            &complete_response,
            true,
            Utc::now(),
            Utc::now(),
        )
        .await
        .map_err(|e| format!("Failed to insert chat message: {}", e))?;
        eprintln!(
            "Added assistant response to conversation {}",
            conversation_id
        );
    }

    Ok("test".into())
}

#[tauri::command]
async fn get_current_conversation(
    app_handle: tauri::AppHandle,
) -> Result<Option<Conversation>, String> {
    let db = app_handle.state::<SharedPersonalDb>().clone();
    let current_conversation_id = app_handle.state::<SharedCurrentConversationId>().clone();
    Ok(db.get_conversation(&current_conversation_id).await.unwrap())
}

// remember to call `.manage(MyState::default())`
#[tauri::command]
async fn get_conversation(
    app_handle: tauri::AppHandle,
    conversation_id: String,
) -> Result<Conversation, String> {
    let db = app_handle.state::<SharedPersonalDb>().clone();
    let conversation = db
        .get_conversation(&conversation_id)
        .await
        .map_err(|e| format!("Failed to get conversation: {}", e))
        .unwrap();

    Ok(conversation.unwrap())
}

#[tauri::command]
async fn switch_conversation(
    app_handle: tauri::AppHandle,
    conversation_id: String,
) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
async fn list_conversations(app_handle: tauri::AppHandle) -> Result<Vec<Conversation>, String> {
    let db = app_handle.state::<SharedPersonalDb>().clone();
    let conversations = db.list_conversations().await.unwrap();
    Ok(conversations)
    // Ok(vec![])
    // let storage_state: tauri::State<SharedConversationStorage> = app_handle.state();
    // let storage_guard = storage_state.lock().await;
    // let storage = storage_guard.as_ref().ok_or("Storage not initialized")?;

    // storage.list_conversations().map_err(|e| e.to_string())
}

// Command to manually send a key event to the launcher
#[tauri::command]
async fn send_key_to_launcher(app_handle: tauri::AppHandle, key: String) -> Result<(), String> {
    if let Some(launcher) = app_handle.get_window("launcher") {
        launcher
            .emit("key_event", key)
            .map_err(|e| format!("Failed to send key event: {}", e))?;
    }
    Ok(())
}

// Add this new command to check server status
#[tauri::command]
async fn check_grpc_server_connection(server_address: Option<String>) -> Result<String, String> {
    let address = server_address.unwrap_or_else(|| "0.0.0.0:50051".to_string());

    eprintln!("Checking connection to gRPC server: {}", address);

    // Try to establish a basic TCP connection to check if the server is listening
    match tokio::net::TcpStream::connect(address.replace("http://", "").replace("https://", ""))
        .await
    {
        Ok(_) => {
            eprintln!("TCP connection successful");
            Ok("Server is reachable".to_string())
        }
        Err(e) => {
            let error_msg = format!("Failed to connect to server: {}", e);
            eprintln!("{}", error_msg);
            Err(error_msg)
        }
    }
}

use eur_activity::DisplayAsset;
#[tauri::command]
async fn list_activities(app_handle: tauri::AppHandle) -> Result<Vec<DisplayAsset>, String> {
    let timeline_state: tauri::State<SharedTimeline> = app_handle.state();
    let timeline = timeline_state.inner();

    // Get all activities from the timeline
    // let mut activities = timeline.get_activities();
    let activities = timeline.get_activities();

    // Sort activities by start time (most recent first)
    // activities.sort_by(|a, b| b.start.cmp(&a.start));

    // Limit to the 5 most recent activities to avoid cluttering the UI
    let limited_activities = activities.into_iter().take(5).collect();

    Ok(limited_activities)
}
