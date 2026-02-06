#![cfg_attr(
    all(windows, not(test), not(debug_assertions)),
    windows_subsystem = "windows"
)]

use dotenv::dotenv;
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
        // message_procedures::{MessageApi, MessageApiImpl},
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
use log::{debug, error};
use tauri::{
    Manager, generate_context,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};
use tauri_plugin_log::fern::colors::ColoredLevelConfig;
use taurpc::Router;
use tokio::sync::Mutex;

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

    // Initialize mock keyring for e2e tests and CI environments
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

    let _sentry_logger = sentry::integrations::log::SentryLogger::new()
        .filter(|_md| sentry::integrations::log::LogFilter::Log);

    // Regular application startup
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
                    let started_by_autostart = std::env::args().any(|arg| arg == "--startup-launch");
                    if started_by_autostart {
                        let event = posthog_rs::Event::new_anon("start_app_by_autostart");

                        tauri::async_runtime::spawn(async move {
                            let _ = posthog_rs::capture(event).await.map_err(|e| {
                                error!("Failed to capture posthog event: {}", e);
                            });
                        });
                    }

                    let app_settings = AppSettings::load_from_default_path_creating().unwrap();
                    tauri_app.manage(Mutex::new(app_settings.clone()));

                    tauri::async_runtime::spawn(async move {
                        let _ = initialize_posthog().await.map_err(|e| {
                            error!("Failed to initialize posthog: {}", e);
                        });
                    });

                    // #[cfg(all(desktop, not(debug_assertions)))]
                    if app_settings.general.autostart && !started_by_autostart {
                        use tauri_plugin_autostart::MacosLauncher;
                        use tauri_plugin_autostart::ManagerExt;

                        let _ = tauri_app.handle().plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--startup-launch"]) /* arbitrary number of args to pass to your app */));

                        // Get the autostart manager
                        let autostart_manager = tauri_app.autolaunch();
                        // Enable autostart
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
                        }
                    });


                    #[cfg(debug_assertions)]
                    {
                        // main_window.open_devtools();
                        // launcher_window.open_devtools();
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

                    let conversation_handle = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        let conversation_manager = euro_conversation::ConversationManager::new().await;
                        conversation_handle.manage(SharedConversationManager::new(conversation_manager));
                    });


                    let timeline_handle = app_handle.clone();
                    let db_app_handle = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        // let db_manager = match create_shared_database_manager(&db_app_handle).await {
                        //     Ok(db) => {
                        //         Some(db)
                        //     }
                        //     Err(e) => {
                        //         error!("Failed to initialize personal database manager: {}", e);
                        //         None
                        //     }
                        // };
                        // if let Some(db_manager) = db_manager {
                        //     db_app_handle.manage(db_manager);
                        let timeline = euro_timeline::TimelineManager::builder().build().await.expect("Failed to create timeline");
                        timeline_handle.manage(Mutex::new(timeline));
                            let timeline_mutex = db_app_handle.state::<Mutex<TimelineManager>>();

                            let mut asset_receiver = {
                                let timeline = timeline_mutex.lock().await;
                                timeline.subscribe_to_assets_events()
                            };
                            let assets_timeline_handle = db_app_handle.clone();
                            tauri::async_runtime::spawn(async move {
                                while let Ok(assets_event) = asset_receiver.recv().await {
                                   let _ = TauRpcTimelineApiEventTrigger::new(assets_timeline_handle.clone())
                                    .new_assets_event(assets_event);
                                }
                            });

                            // Subscribe to activity change events before starting timeline
                            let mut activity_receiver = {
                                let timeline = timeline_mutex.lock().await;
                                timeline.subscribe_to_activity_events()
                            };



                            let activity_timeline_handle = db_app_handle.clone();
                            tauri::async_runtime::spawn(async move {
                                // let db_manager = activity_timeline_handle.state::<PersonalDatabaseManager>().inner();
                                while let Ok(activity_event) = activity_receiver.recv().await {
                                    debug!("Activity changed to: {}",
                                        activity_event.name.clone(),
                                    );

                                    let mut primary_icon_color = None;
                                    let mut icon_base64 = None;

                                    if let Some(icon) = activity_event.icon.as_ref() {
                                        primary_icon_color = color_thief::get_palette(icon, color_thief::ColorFormat::Rgba, 10, 10).ok().map(|c| format!("#{r:02X}{g:02X}{b:02X}", r = c[0].r, g = c[0].g, b = c[0].b));
                                        icon_base64 = euro_vision::rgba_to_base64(icon).ok();
                                    }

                                    let _ = TauRpcTimelineApiEventTrigger::new(activity_timeline_handle.clone())
                                        .new_app_event( TimelineAppEvent {
                                            name: activity_event.name.clone(),
                                            color: primary_icon_color,
                                            icon_base64
                                        });


                                    // // Close previous active activity if exists
                                    // if let Ok(Some(last_activity)) = db_manager.get_last_active_activity().await {
                                    //     let _ = db_manager.update_activity_end_time(&last_activity.id, focus_event.timestamp).await;
                                    //     debug!("Closed previous activity: {}", last_activity.name);
                                    // }

                                    // // Create new activity for the focus change
                                    // let activity = Activity {
                                    //     id: Uuid::new_v4().to_string(),
                                    //     name: focus_event.window_title.clone(),
                                    //     icon_path: None,
                                    //     process_name: focus_event.process_name.clone(),
                                    //     started_at: focus_event.timestamp.to_rfc3339(),
                                    //     ended_at: None,
                                    // };

                                    // match db_manager.insert_activity(&activity).await {
                                    //     Ok(_) => {
                                    //         debug!("Inserted activity: {} ({})", activity.name, activity.process_name);
                                    //         debug!("Activity inserted with ID: {}", activity.id);
                                    //     }
                                    //     Err(e) => {
                                    //         error!("Failed to insert activity: {}", e);
                                    //     }
                                    // }
                                }
                            });

                            let mut timeline = timeline_mutex.lock().await;
                            if let Err(e) = timeline.start().await {
                                error!("Failed to start timeline collection: {}", e);
                            } else {
                                debug!("Timeline collection started successfully");
                            }

                            // }
                    });


                    let app_handle_user = app_handle.clone();
                    let path = tauri_app.path().app_data_dir().unwrap();
                    tauri::async_runtime::spawn(async move {
                        let user_controller = euro_user::Controller::from_path(path)
                            .await
                            .map_err(|e| {
                                error!("Failed to create user controller: {}", e);
                                e
                            })
                            .unwrap();
                        app_handle_user.manage(SharedUserController::new(user_controller));
                        // app_handle_user.manage(user_controller);
                    });


                    Ok(())
                })
                .plugin(tauri_plugin_http::init())
                .plugin(tauri_plugin_opener::init())
                .plugin(
                    tauri_plugin_log::Builder::new()
                            .filter(|metadata| {
                                let target = metadata.target();
                                // Allow all logs from euro-* crates (Rust converts hyphens to underscores in module paths)
                                let is_euro_crate = target.starts_with("euro_");
                                // Allow all logs from common folder crates
                                let is_common_crate = target.starts_with("agent_chain")
                                    || target.starts_with("agent_graph")
                                    || target.starts_with("auth_core")
                                    || target.starts_with("focus_tracker")
                                    || target.starts_with("proto_gen");
                                // Allow webview logs
                                let is_webview = target.starts_with("webview");
                                // For third-party crates, only allow warnings and above
                                let is_warning_or_above = metadata.level() <= log::Level::Warn;
                                is_euro_crate || is_common_crate || is_webview || is_warning_or_above
                            })
                            .level(log::LevelFilter::Trace)
                            // .target(Target::new(TargetKind::Stdout))
                            .with_colors(ColoredLevelConfig::default())
                            .build()
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
                .merge(TimelineApiImpl.into_handler())
                .merge(ConversationApiImpl.into_handler())
                .merge(SettingsApiImpl.into_handler())
                .merge(ThirdPartyApiImpl.into_handler())
                .merge(MonitorApiImpl.into_handler())
                // .merge(MessageApiImpl.into_handler())
                .merge(SystemApiImpl.into_handler())
                .merge(ContextChipApiImpl.into_handler())
                .merge(PromptApiImpl.into_handler())
                .merge(OnboardingApiImpl.into_handler())
                .merge(ChatApiImpl.into_handler());
            builder
                // .invoke_handler(tauri::generate_handler![list_conversations,])
                .invoke_handler(router.into_handler())
                .build(tauri_context)
                .expect("Failed to build tauri app")
                .run(|_app_handle, _event| {});
        });
}
