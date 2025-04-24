#![cfg_attr(
    all(windows, not(test), not(debug_assertions)),
    windows_subsystem = "windows"
)]
mod keyring_service;

use eur_client_grpc::client_builder;
use eur_client_questions::QuestionsClient;
use eur_conversation::{Asset, ChatMessage, Conversation, ConversationStorage};
use eur_native_messaging::create_grpc_ipc_client;
use eur_proto::ipc::{ProtoArticleState, ProtoPdfState, ProtoYoutubeState};
use eur_proto::questions_service::ProtoChatMessage;
use eur_tauri::{WindowState, create_launcher};
use eur_timeline::{BrowserState, Timeline};
use futures::StreamExt;
use keyring_service::{ApiKeyStatus, KeyringService};
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::ipc::Channel;
use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Emitter, Wry};
use tauri::{Manager, generate_context};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tauri_plugin_log::{Target, TargetKind};
use tokio::time::{Duration, sleep};

// mod focus_tracker;

use eur_timeline::focus_tracker;

use tracing::{error, info};
type SharedTimeline = Arc<Timeline>;
type SharedQuestionsClient = Arc<Mutex<Option<QuestionsClient>>>;
type SharedOpenAIClient = Arc<async_mutex::Mutex<Option<eur_openai::OpenAI>>>;
type SharedConversationStorage = Arc<async_mutex::Mutex<Option<ConversationStorage>>>;
type SharedCurrentConversation = Arc<Mutex<Option<Conversation>>>;
type SharedKeyringService = Arc<KeyringService>;

fn create_shared_conversation_storage() -> SharedConversationStorage {
    Arc::new(async_mutex::Mutex::new(None))
}

fn create_shared_current_conversation() -> SharedCurrentConversation {
    Arc::new(Mutex::new(None))
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

fn create_shared_keyring_service() -> SharedKeyringService {
    Arc::new(KeyringService::new())
}

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
            width: 800.0,
            height: height as f64,
        }));
    }
    Ok(())
}

#[tauri::command]
async fn check_api_key_exists(
    keyring_service: tauri::State<'_, SharedKeyringService>,
) -> Result<ApiKeyStatus, String> {
    let has_key = keyring_service.has_api_key();
    Ok(ApiKeyStatus { has_key })
}

#[tauri::command]
async fn save_api_key(
    keyring_service: tauri::State<'_, SharedKeyringService>,
    api_key: String,
) -> Result<(), String> {
    keyring_service
        .set_api_key(&api_key)
        .map_err(|e| format!("Failed to save API key: {}", e))
}

#[tauri::command]
async fn initialize_openai_client(
    app_handle: tauri::AppHandle,
    keyring_service: tauri::State<'_, SharedKeyringService>,
) -> Result<bool, String> {
    // Get the API key from keyring
    let api_key = keyring_service
        .get_api_key()
        .map_err(|e| format!("Failed to get API key: {}", e))?;

    // Initialize the OpenAI client with the API key
    let openai_client = eur_openai::OpenAI::with_api_key(&api_key);

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
                    let transcript_state = Arc::new(Mutex::new(None::<String>));
                    app_handle.manage(transcript_state);

                    // Initialize the gRPC client state
                    let questions_client = create_shared_client();
                    app_handle.manage(questions_client.clone());

                    // Initialize the timeline and start collection
                    let timeline = create_shared_timeline();
                    app_handle.manage(timeline.clone());

                    // focus_tracker::spawn(&timeline.clone());

                    // Create the keyring service
                    let keyring_service = create_shared_keyring_service();
                    app_handle.manage(keyring_service.clone());

                    // Create the OpenAI client (initially None, will be initialized after API key check)
                    let openai_client = create_shared_openai_client();
                    app_handle.manage(openai_client.clone());

                    // Check if API key exists and initialize OpenAI client if it does
                    let keyring_clone = keyring_service.clone();
                    let app_handle_clone = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        if keyring_clone.has_api_key() {
                            if let Ok(api_key) = keyring_clone.get_api_key() {
                                let client = eur_openai::OpenAI::with_api_key(&api_key);
                                let state: tauri::State<SharedOpenAIClient> =
                                    app_handle_clone.state();
                                let mut guard = state.lock().await;
                                *guard = Some(client);
                                info!("OpenAI client initialized with API key from keyring");
                            }
                        } else {
                            info!("No API key found in keyring, OpenAI client not initialized");
                        }
                    });
                    app_handle.manage(openai_client.clone());

                    // Initialize conversation storage
                    let conversation_storage = create_shared_conversation_storage();
                    let current_conversation = create_shared_current_conversation();

                    // Initialize storage with a database in the app's data directory
                    let db_path = get_db_path(&app_handle);

                    app_handle.manage(conversation_storage.clone());

                    tauri::async_runtime::spawn(async move {
                        if let Ok(storage) = ConversationStorage::new(db_path) {
                            conversation_storage.lock().await.replace(storage);
                        }
                    });

                    app_handle.manage(current_conversation);

                    // Start the timeline collection process in the background
                    let timeline_clone = timeline.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(e) = timeline_clone.start_collection().await {
                            error!("Failed to start timeline collection: {}", e);
                        } else {
                            info!("Timeline collection started successfully");
                        }
                    });

                    // Initialize the IPC client asynchronously
                    let ipc_handle = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        let ipc_client = create_grpc_ipc_client().await.unwrap();
                        ipc_handle.manage(ipc_client.clone());
                    });

                    // Connect to the gRPC server
                    let client_handle = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        init_grpc_client(client_handle).await;
                    });

                    #[cfg(desktop)]
                    {
                        // println!("Setting up global shortcut");
                        let super_space_shortcut =
                            Shortcut::new(Some(Modifiers::SUPER), Code::Space);

                        let launcher_label = launcher_window.label().to_string();

                        app_handle.plugin(shortcut_plugin(super_space_shortcut, launcher_label))?;

                        app_handle
                            .global_shortcut()
                            .register(super_space_shortcut)?;
                    }

                    // Listen for window focus events
                    let launcher_label = launcher_window.label().to_string();
                    let app_handle_clone = app_handle.clone();

                    // We'll use a different approach for Windows

                    // Keep the window-specific handler for Linux
                    #[cfg(target_os = "linux")]
                    launcher_window.on_window_event(move |event| {
                        if let tauri::WindowEvent::Focused(false) = event {
                            if let Some(launcher) = app_handle_clone.get_window(&launcher_label) {
                                launcher.hide().expect("Failed to hide launcher window");
                                // Emit an event to clear the conversation when launcher is hidden
                                launcher
                                    .emit("launcher_closed", ())
                                    .expect("Failed to emit launcher_closed event");
                            }
                        }
                    });

                    Ok(())
                })
                .plugin(tauri_plugin_http::init())
                .plugin(tauri_plugin_shell::init())
                .plugin(tauri_plugin_single_instance::init(|_, _, _| {}))
                .plugin(
                    tauri_plugin_log::Builder::default()
                        .targets([
                            Target::new(TargetKind::Stdout),
                            Target::new(TargetKind::LogDir { file_name: None }),
                        ])
                        .build(),
                )
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
                        // Handle launcher window focus loss for Windows
                        #[cfg(not(target_os = "linux"))]
                        {
                            // Check if this is the launcher window
                            if window.label() == "launcher" {
                                window.hide().expect("Failed to hide launcher window");
                                // Emit an event to clear the conversation when launcher is hidden
                                window
                                    .emit("launcher_closed", ())
                                    .expect("Failed to emit launcher_closed event");
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
                    list_conversations,
                    check_api_key_exists,
                    save_api_key,
                    initialize_openai_client,
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
            } else {
                // Get cursor position and center launcher on that screen
                if let Ok(cursor_position) = launcher.cursor_position() {
                    if let Ok(monitors) = launcher.available_monitors() {
                        for monitor in monitors {
                            let size = monitor.size();
                            let position = monitor.position();

                            // Check if cursor is on this monitor
                            if cursor_position.x >= position.x as f64
                                && cursor_position.x <= (position.x + size.width as i32) as f64
                                && cursor_position.y >= position.y as f64
                                && cursor_position.y <= (position.y + size.height as i32) as f64
                            {
                                // Center the launcher on this monitor
                                let window_size = launcher.inner_size().unwrap();
                                let x =
                                    position.x + (size.width as i32 - window_size.width as i32) / 2;
                                let y = position.y
                                    + (size.height as i32 - window_size.height as i32) / 4;
                                // let y = (size.height as i32 - window_size.height as i32);

                                launcher
                                    .set_position(tauri::Position::Physical(
                                        tauri::PhysicalPosition { x, y },
                                    ))
                                    .expect("Failed to set launcher position");
                                break;
                            }
                        }
                    }
                }

                // Only show the launcher if it was previously hidden
                launcher.show().expect("Failed to show launcher window");

                // Add a small delay before setting focus
                let launcher_clone = launcher.clone();
                tauri::async_runtime::spawn(async move {
                    sleep(Duration::from_millis(1000)).await;
                    launcher_clone
                        .set_focus()
                        .expect("Failed to focus launcher window");
                });
            }
        })
        .build()
}

// Initialize the gRPC client connection
async fn init_grpc_client(app_handle: tauri::AppHandle) {
    let server_url = "http://[::1]:50051".to_string(); // Using IPv6 localhost format

    let builder = client_builder().with_base_url(server_url.clone());

    match builder.create_channel().await {
        Ok(channel) => match QuestionsClient::new(channel) {
            Ok(client) => {
                let state: tauri::State<SharedQuestionsClient> = app_handle.state();
                if let Ok(mut guard) = state.lock() {
                    *guard = Some(client);
                    eprintln!("Connected to gRPC server at {}", server_url);
                }
            }
            Err(e) => {
                eprintln!("Failed to create questions client: {}", e);
            }
        },
        Err(e) => {
            eprintln!("Failed to connect to gRPC server: {}", e);
        }
    }
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

    // Get the conversation storage
    let storage_state: tauri::State<SharedConversationStorage> = app_handle.state();
    let storage_guard = storage_state.lock().await;
    let storage = storage_guard.as_ref().ok_or("Storage not initialized")?;

    // Get the conversation by ID
    let mut conversation = storage
        .get_conversation(&conversation_id)
        .map_err(|e| format!("Failed to get conversation: {}", e))?;

    // Add the user's question to the conversation
    let chat_message = ChatMessage::new(None, "user".to_string(), question.clone(), true);
    conversation
        .add_message(chat_message)
        .map_err(|e| format!("Failed to add message to conversation: {}", e))?;

    // Save the updated conversation
    storage
        .save_conversation(&conversation)
        .map_err(|e| format!("Failed to save conversation: {}", e))?;

    // Get the assets for this conversation
    let assets = storage
        .get_conversation_assets(&conversation_id)
        .map_err(|e| format!("Failed to get conversation assets: {}", e))?;

    if assets.is_empty() {
        return Err("No assets found for this conversation".to_string());
    }

    // For now, we'll just use the first asset
    let asset = &assets[0];

    // Get the OpenAI client
    let state: tauri::State<SharedOpenAIClient> = app_handle.state();
    let mut guard = state.lock().await;
    let client = guard
        .as_mut()
        .ok_or_else(|| "OpenAI client not initialized".to_string())?;

    // Process based on asset type
    if asset.asset_type == "youtube" {
        // Deserialize the YouTube state from the asset content
        let youtube_state: ProtoYoutubeState =
            serde_json::from_value(asset.content.get("Youtube").unwrap().clone())
                .map_err(|e| format!("Failed to deserialize YouTube state: {}", e))?;

        // Get all messages from the conversation
        let messages: Vec<ProtoChatMessage> = conversation
            .messages
            .iter()
            .map(|msg| ProtoChatMessage {
                role: msg.role.clone(),
                content: msg.content.clone(),
            })
            .collect();

        // Collect the complete response
        let mut complete_response = String::new();

        // Send the question to the OpenAI API
        let mut stream = client.video_question_old(messages, youtube_state).await?;

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
            // Create a new ChatMessage with the assistant's response
            let chat_message =
                ChatMessage::new(None, "assistant".to_string(), complete_response, true);

            // Add the message to the conversation and save it
            let mut updated_conversation = conversation.clone();
            updated_conversation
                .add_message(chat_message)
                .map_err(|e| e.to_string())?;
            storage.save_conversation(&updated_conversation).unwrap();

            eprintln!(
                "Added assistant response to conversation {}",
                updated_conversation.id
            );
        }

        Ok(())
    } else if asset.asset_type == "article" {
        // Deserialize the Article state from the asset content
        let article_state: ProtoArticleState =
            serde_json::from_value(asset.content.get("Article").unwrap().clone())
                .map_err(|e| format!("Failed to deserialize Article state: {}", e))?;

        // Get all messages from the conversation
        let messages: Vec<ProtoChatMessage> = conversation
            .messages
            .iter()
            .map(|msg| ProtoChatMessage {
                role: msg.role.clone(),
                content: msg.content.clone(),
            })
            .collect();

        // Collect the complete response
        let mut complete_response = String::new();

        // Send the question to the OpenAI API
        let mut stream = client.article_question(messages, article_state).await?;

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
            // Create a new ChatMessage with the assistant's response
            let chat_message =
                ChatMessage::new(None, "assistant".to_string(), complete_response, true);

            // Add the message to the conversation and save it
            let mut updated_conversation = conversation.clone();
            updated_conversation
                .add_message(chat_message)
                .map_err(|e| e.to_string())?;
            storage.save_conversation(&updated_conversation).unwrap();

            eprintln!(
                "Added assistant response to conversation {}",
                updated_conversation.id
            );
        }

        Ok(())
    } else if asset.asset_type == "pdf" {
        // Deserialize the PDF state from the asset content
        let pdf_state: ProtoPdfState =
            serde_json::from_value(asset.content.get("Pdf").unwrap().clone())
                .map_err(|e| format!("Failed to deserialize PDF state: {}", e))?;

        // Get all messages from the conversation
        let messages: Vec<ProtoChatMessage> = conversation
            .messages
            .iter()
            .map(|msg| ProtoChatMessage {
                role: msg.role.clone(),
                content: msg.content.clone(),
            })
            .collect();

        // Collect the complete response
        let mut complete_response = String::new();

        // Send the question to the OpenAI API
        let mut stream = client.pdf_question(messages, pdf_state).await?;

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
            // Create a new ChatMessage with the assistant's response
            let chat_message =
                ChatMessage::new(None, "assistant".to_string(), complete_response, true);

            // Add the message to the conversation and save it
            let mut updated_conversation = conversation.clone();
            updated_conversation
                .add_message(chat_message)
                .map_err(|e| e.to_string())?;
            storage.save_conversation(&updated_conversation).unwrap();

            eprintln!(
                "Added assistant response to conversation {}",
                updated_conversation.id
            );
        }

        Ok(())
    } else {
        Err(format!(
            "Asset type {} not yet implemented",
            asset.asset_type
        ))
    }
}

// Tauri command to ask questions about content (video or article) via gRPC
#[tauri::command]
async fn ask_video_question(
    app_handle: tauri::AppHandle,
    conversation_id: String,
    question: String,
    channel: Channel<DownloadEvent<'_>>,
) -> Result<String, String> {
    eprintln!("Asking question: {}", question);
    eprintln!("Conversation ID: {}", conversation_id);
    // Get the timeline from app state
    let timeline_state: tauri::State<SharedTimeline> = app_handle.state();
    let timeline = timeline_state.inner();

    let mut title: Option<String> = Some("Test".to_string());

    let mut messages = timeline.construct_asset_messages();

    messages.push(eur_prompt_kit::Message {
        role: eur_prompt_kit::Role::User,
        content: eur_prompt_kit::MessageContent::Text(eur_prompt_kit::TextContent {
            text: question.clone(),
        }),
    });

    // Collect a new fragment from the timeline
    // let content_data: BrowserState = {
    //     let fragment = timeline
    //         .collect_new_fragment()
    //         .await
    //         .map_err(|e| format!("Failed to collect new fragment: {}", e))?;

    //     let browser_state = fragment
    //         .browser_state
    //         .ok_or_else(|| "No browser state in the newly collected fragment".to_string())?;

    //     match browser_state {
    //         BrowserState::Youtube(youtube_state) => {
    //             let proto_youtube: ProtoYoutubeState = youtube_state.clone();
    //             eprintln!(
    //                 "YouTube content detected from timeline: {:?}",
    //                 youtube_state.title
    //             );
    //             eprintln!("URL: {}", youtube_state.url);
    //             title = Some(youtube_state.title.clone());
    //             Some(BrowserState::Youtube(proto_youtube))
    //         }
    //         BrowserState::Article(article_state) => {
    //             let proto_article: ProtoArticleState = article_state.clone();
    //             eprintln!(
    //                 "Article content detected from timeline: {:?}",
    //                 article_state.title
    //             );
    //             eprintln!("URL: {}", article_state.url);
    //             title = Some(article_state.title.clone());
    //             Some(BrowserState::Article(proto_article))
    //         }
    //         BrowserState::Pdf(pdf_state) => {
    //             let proto_pdf: ProtoPdfState = pdf_state.clone();
    //             eprintln!("PDF content detected from timeline: {:?}", pdf_state.title);
    //             eprintln!("URL: {}", pdf_state.url);
    //             title = Some(pdf_state.title.clone());
    //             Some(BrowserState::Pdf(proto_pdf))
    //         }
    //     }
    // }
    // .ok_or_else(|| "No content available in timeline".to_string())?;

    let state: tauri::State<SharedOpenAIClient> = app_handle.state();
    let mut guard = state.lock().await;
    let client = guard
        .as_mut()
        .ok_or_else(|| "OpenAI client not initialized".to_string())?;

    if conversation_id == "NEW" {
        // Create new conversation and store it in SQLite
        eprintln!(
            "Creating new conversation with title: {}",
            title.clone().unwrap()
        );
        let mut conversation = Conversation::new(None, title.clone());
        let chat_message = ChatMessage::new(None, "user".to_string(), question.clone(), true);

        conversation.add_message(chat_message).unwrap();

        // Store empty conversation in SQLite
        let storage_state: tauri::State<SharedConversationStorage> = app_handle.state();
        let storage_guard = storage_state.lock().await;
        let storage = storage_guard.as_ref().ok_or("Storage not initialized")?;
        storage.save_conversation(&conversation).unwrap();

        // Create a new asset with the browser state
        // let browser_state_json = serde_json::to_value(&content_data)
        // .map_err(|e| format!("Failed to serialize browser state: {}", e))?;

        // Add the asset to the conversation
        // storage
        //     .save_asset(&Asset::new(
        //         conversation.id.clone(),
        //         content_data.content_type().to_string(),
        //         browser_state_json,
        //     ))
        //     .unwrap();
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
        // Get the conversation storage
        let storage_state: tauri::State<SharedConversationStorage> = app_handle.state();
        let storage_guard = storage_state.lock().await;
        let storage = storage_guard.as_ref().ok_or("Storage not initialized")?;

        // Get the current conversation
        let conversation = if conversation_id == "NEW" {
            // If this is a new conversation, we need to get it by the most recent one
            let conversations = storage.list_conversations().unwrap();
            if conversations.is_empty() {
                return Err("No conversations found".to_string());
            }
            conversations[0].clone()
        } else {
            // Otherwise, get the conversation by ID
            storage.get_conversation(&conversation_id).unwrap()
        };

        // Create a new ChatMessage with the assistant's response
        let chat_message = ChatMessage::new(None, "assistant".to_string(), complete_response, true);

        // Add the message to the conversation and save it
        let mut updated_conversation = conversation.clone();
        updated_conversation
            .add_message(chat_message)
            .map_err(|e| e.to_string())?;
        storage.save_conversation(&updated_conversation).unwrap();

        eprintln!(
            "Added assistant response to conversation {}",
            updated_conversation.id
        );
    }

    Ok("test".into())

    // if content_data.content_type() == "youtube" {
    //     // Collect the complete response
    //     let mut complete_response = String::new();

    //     let mut stream = client
    //         .video_question(
    //             vec![ProtoChatMessage {
    //                 role: "user".to_string(),
    //                 content: question.clone(),
    //             }],
    //             content_data.youtube().unwrap(),
    //         )
    //         .await?;
    //     channel
    //         .send(DownloadEvent::Message { message: "" })
    //         .map_err(|e| format!("Failed to send response: {e}"))?;

    // while let Some(Ok(chunk)) = stream.next().await {
    //     for message in chunk.choices {
    //         let Some(message) = message.delta.content else {
    //             continue;
    //         };
    //         // Append to the complete response
    //         complete_response.push_str(&message);

    //         channel
    //             .send(DownloadEvent::Append { chunk: &message })
    //             .map_err(|e| format!("Failed to send response: {e}"))?;
    //     }
    // }

    //     // After the stream ends, add the complete response as a ChatMessage to the conversation
    //     if !complete_response.is_empty() {
    //         // Get the conversation storage
    //         let storage_state: tauri::State<SharedConversationStorage> = app_handle.state();
    //         let storage_guard = storage_state.lock().await;
    //         let storage = storage_guard.as_ref().ok_or("Storage not initialized")?;

    //         // Get the current conversation
    //         let conversation = if conversation_id == "NEW" {
    //             // If this is a new conversation, we need to get it by the most recent one
    //             let conversations = storage.list_conversations().unwrap();
    //             if conversations.is_empty() {
    //                 return Err("No conversations found".to_string());
    //             }
    //             conversations[0].clone()
    //         } else {
    //             // Otherwise, get the conversation by ID
    //             storage.get_conversation(&conversation_id).unwrap()
    //         };

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

    //     Ok("test".into())
    // } else if content_data.content_type() == "article" {
    //     let mut complete_response = String::new();

    //     let mut stream = client
    //         .article_question(
    //             vec![ProtoChatMessage {
    //                 role: "user".to_string(),
    //                 content: question.clone(),
    //             }],
    //             content_data.article().unwrap(),
    //         )
    //         .await?;
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
    //         // Get the conversation storage
    //         let storage_state: tauri::State<SharedConversationStorage> = app_handle.state();
    //         let storage_guard = storage_state.lock().await;
    //         let storage = storage_guard.as_ref().ok_or("Storage not initialized")?;

    //         // Get the current conversation
    //         let conversation = if conversation_id == "NEW" {
    //             // If this is a new conversation, we need to get it by the most recent one
    //             let conversations = storage.list_conversations().unwrap();
    //             if conversations.is_empty() {
    //                 return Err("No conversations found".to_string());
    //             }
    //             conversations[0].clone()
    //         } else {
    //             // Otherwise, get the conversation by ID
    //             storage.get_conversation(&conversation_id).unwrap()
    //         };

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

    //     Ok("test".into())
    // } else if content_data.content_type() == "pdf" {
    //     let mut complete_response = String::new();

    //     let mut stream = client
    //         .pdf_question(
    //             vec![ProtoChatMessage {
    //                 role: "user".to_string(),
    //                 content: question.clone(),
    //             }],
    //             content_data.pdf().unwrap(),
    //         )
    //         .await?;
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
    //         // Get the conversation storage
    //         let storage_state: tauri::State<SharedConversationStorage> = app_handle.state();
    //         let storage_guard = storage_state.lock().await;
    //         let storage = storage_guard.as_ref().ok_or("Storage not initialized")?;

    //         // Get the current conversation
    //         let conversation = if conversation_id == "NEW" {
    //             // If this is a new conversation, we need to get it by the most recent one
    //             let conversations = storage.list_conversations().unwrap();
    //             if conversations.is_empty() {
    //                 return Err("No conversations found".to_string());
    //             }
    //             conversations[0].clone()
    //         } else {
    //             // Otherwise, get the conversation by ID
    //             storage.get_conversation(&conversation_id).unwrap()
    //         };

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

    //     Ok("test".into())
    // } else {
    //     return Err("No content available in timeline".to_string());
    // }
    // return Err("No content available in timeline".to_string());
}

#[tauri::command]
async fn get_current_conversation(
    app_handle: tauri::AppHandle,
) -> Result<Option<Conversation>, String> {
    let current_conversation_state: tauri::State<SharedCurrentConversation> = app_handle.state();
    let current_conversation = current_conversation_state
        .lock()
        .map_err(|e| e.to_string())?;
    Ok(current_conversation.clone())
}

#[tauri::command]
async fn switch_conversation(
    app_handle: tauri::AppHandle,
    conversation_id: String,
) -> Result<(), String> {
    let storage_state: tauri::State<SharedConversationStorage> = app_handle.state();
    let storage_guard = storage_state.lock().await;
    let storage = storage_guard.as_ref().ok_or("Storage not initialized")?;

    let conversation = storage
        .get_conversation(&conversation_id)
        .map_err(|e| e.to_string())?;

    let current_conversation_state: tauri::State<SharedCurrentConversation> = app_handle.state();
    let mut current_conversation = current_conversation_state
        .lock()
        .map_err(|e| e.to_string())?;
    *current_conversation = Some(conversation);

    Ok(())
}

#[tauri::command]
async fn list_conversations(app_handle: tauri::AppHandle) -> Result<Vec<Conversation>, String> {
    let storage_state: tauri::State<SharedConversationStorage> = app_handle.state();
    let storage_guard = storage_state.lock().await;
    let storage = storage_guard.as_ref().ok_or("Storage not initialized")?;

    storage.list_conversations().map_err(|e| e.to_string())
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
    let mut activities = timeline.get_activities_temp();

    // Sort activities by start time (most recent first)
    // activities.sort_by(|a, b| b.start.cmp(&a.start));

    // Limit to the 5 most recent activities to avoid cluttering the UI
    let limited_activities = activities.into_iter().take(5).collect();

    Ok(limited_activities)
}
