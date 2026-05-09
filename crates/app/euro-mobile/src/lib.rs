#![allow(
    clippy::used_underscore_binding,
    clippy::module_name_repetitions,
    clippy::struct_field_names,
    clippy::too_many_lines
)]

use procedures::auth_procedures::{
    AuthStateChanged, auth_get_access_token_payload, auth_is_authenticated, auth_login,
    auth_logout, auth_refresh_session, auth_register, auth_start_login,
};

pub mod error;
pub mod procedures;
mod setup;
pub mod shared_types;

/// Assemble the tauri-specta IPC surface. Lives at the crate root because
/// the per-function macros emitted by `#[tauri::command]` and
/// `#[specta::specta]` are `#[macro_export]`'d into this crate's root macro
/// namespace; calling `collect_commands!` from a sub-module would require
/// path-qualifying every entry.
///
/// Thread/chat commands are sourced from `euro_thread::commands::*` so
/// the desktop and mobile apps share one canonical IPC surface — adding
/// a new thread or chat command requires editing only `euro-thread`.
pub fn build_specta() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new()
        .disable_serde_phases()
        .commands(tauri_specta::collect_commands![
            auth_start_login,
            auth_login,
            auth_register,
            auth_logout,
            auth_is_authenticated,
            auth_get_access_token_payload,
            auth_refresh_session,
            euro_thread::commands::thread::thread_list,
            euro_thread::commands::thread::thread_create,
            euro_thread::commands::thread::thread_delete,
            euro_thread::commands::thread::thread_get_messages,
            euro_thread::commands::thread::thread_switch_branch,
            euro_thread::commands::thread::thread_generate_title,
            euro_thread::commands::thread::thread_search_threads,
            euro_thread::commands::thread::thread_search_messages,
            euro_thread::commands::chat::chat_collect_context,
            euro_thread::commands::chat::chat_send_query,
            euro_thread::commands::chat::chat_regenerate,
            euro_thread::commands::chat::chat_cancel_query,
        ])
        .events(tauri_specta::collect_events![AuthStateChanged])
}

#[cfg(mobile)]
#[tauri::mobile_entry_point]
fn mobile_entry_point() {
    run();
}

/// Mobile apps run in a sandbox with no access to the project's `.env`,
/// so `build.rs` reads it at compile time and forwards the relevant keys
/// into `option_env!` slots. Inject those into the process env at
/// startup so the existing `std::env::var(...)` call sites see them.
fn load_env() {
    for (key, value) in [("WEB_URL", option_env!("WEB_URL"))] {
        if std::env::var_os(key).is_some() {
            continue;
        }
        let Some(v) = value else { continue };
        if v.is_empty() {
            continue;
        }
        // SAFETY: called once at startup before any threads are spawned
        // that might read env concurrently.
        unsafe { std::env::set_var(key, v) };
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
                    .join("../../../apps/mobile/src/lib/bindings/specta.bindings.ts");
                specta
                    .export(specta_typescript::Typescript::default(), &bindings_path)
                    .expect("Failed to export tauri-specta bindings");
            }

            tauri::Builder::default()
                .plugin(tauri_plugin_appauth::init())
                .plugin(tauri_plugin_os::init())
                .plugin(tauri_plugin_http::init())
                .plugin(tauri_plugin_opener::init())
                .invoke_handler(specta.invoke_handler())
                .setup(move |app| {
                    // `mount_events` must run inside `setup` so the typed
                    // event channels are wired before any procedure has a
                    // chance to emit. Move `specta` into the closure so its
                    // event registry stays alive for the app lifetime.
                    specta.mount_events(app);
                    setup::init(app)?;
                    Ok(())
                })
                .build(tauri_context)
                .expect("Failed to build tauri app")
                .run(|_app_handle, _event| {});
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regenerate the TypeScript bindings on every `cargo test` run. This is
    /// the same export the debug-mode `run` path performs at app launch, but
    /// runs on the host without needing the mobile build environment — so
    /// CI and local hosts can keep `specta.bindings.ts` in sync without
    /// having to boot the iOS/Android simulator.
    #[test]
    fn export_specta_bindings() {
        let bindings_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../apps/mobile/src/lib/bindings/specta.bindings.ts");
        build_specta()
            .export(specta_typescript::Typescript::default(), &bindings_path)
            .expect("Failed to export tauri-specta bindings");
    }
}
