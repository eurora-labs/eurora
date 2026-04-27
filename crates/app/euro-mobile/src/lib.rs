#![allow(
    clippy::used_underscore_binding,
    clippy::module_name_repetitions,
    clippy::struct_field_names,
    clippy::too_many_lines
)]

use procedures::auth_procedures::{AuthApi, AuthApiImpl};
use procedures::chat_procedures::{ChatApi, ChatApiImpl};
use procedures::thread_procedures::{ThreadApi, ThreadApiImpl};
use taurpc::Router;

pub mod error;
pub mod procedures;
mod setup;
pub mod shared_types;

pub fn build_router() -> Router<tauri::Wry> {
    Router::new()
        .export_config(
            specta_typescript::Typescript::default()
                .bigint(specta_typescript::BigIntExportBehavior::BigInt),
        )
        .merge(AuthApiImpl.into_handler())
        .merge(ThreadApiImpl.into_handler())
        .merge(ChatApiImpl.into_handler())
}

#[cfg(mobile)]
#[tauri::mobile_entry_point]
fn mobile_entry_point() {
    run();
}

fn load_env() {
    dotenv::dotenv().ok();

    // On mobile the app's working directory is the sandbox, so `dotenv()`
    // above can't find the project's .env. build.rs bakes those values into
    // the binary via cargo:rustc-env; inject them into the process env so
    // existing `std::env::var(...)` call sites (and EndpointManager::from_env)
    // see them.
    for (key, value) in [
        ("AUTH_SERVICE_URL", option_env!("AUTH_SERVICE_URL")),
        ("API_BASE_URL", option_env!("API_BASE_URL")),
    ] {
        if std::env::var_os(key).is_some() {
            continue;
        }
        if let Some(v) = value {
            // SAFETY: called once at startup before any threads are spawned
            // that might read env concurrently.
            unsafe { std::env::set_var(key, v) };
        }
    }
}

pub fn run() {
    load_env();

    let tauri_context = tauri::generate_context!();

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime")
        .block_on(async {
            tauri::async_runtime::set(tokio::runtime::Handle::current());

            let router = build_router();

            tauri::Builder::default()
                .plugin(tauri_plugin_appauth::init())
                .plugin(tauri_plugin_os::init())
                .plugin(tauri_plugin_http::init())
                .plugin(tauri_plugin_opener::init())
                .setup(move |app| {
                    setup::init(app)?;
                    Ok(())
                })
                .invoke_handler(router.into_handler())
                .build(tauri_context)
                .expect("Failed to build tauri app")
                .run(|_app_handle, _event| {});
        });
}
