#![allow(
    clippy::used_underscore_binding,
    clippy::module_name_repetitions,
    clippy::struct_field_names,
    clippy::too_many_lines
)]

pub mod browser_launcher;
pub mod error;
pub mod native_messaging;
pub mod office_addin;
pub mod procedures;
pub mod shared_types;
pub mod telemetry;
pub mod util;
pub mod window;
pub use window::{
    MAIN_WINDOW_LABEL, create as create_window, show_and_focus_main, state::WindowState,
};

use procedures::auth_procedures::{
    AuthStateChanged, auth_get_access_token_payload, auth_get_login_token, auth_is_authenticated,
    auth_login, auth_logout, auth_poll_for_login, auth_refresh_session, auth_register,
    auth_resend_verification_email,
};
use procedures::chat_procedures::{
    chat_cancel_query, chat_collect_context, chat_regenerate, chat_send_query,
};
use procedures::payment_procedures::{payment_create_checkout_url, payment_is_subscribed};
use procedures::settings_procedures::{
    settings_get_all, settings_get_api, settings_get_appearance, settings_get_general,
    settings_get_telemetry, settings_set_api, settings_set_appearance, settings_set_general,
    settings_set_telemetry,
};
use procedures::system_procedures::{
    BrowserExtensionStatusChanged, system_check_accessibility_permission,
    system_check_backend_connection, system_check_for_update, system_focus_main_window,
    system_get_browser_extension_state, system_get_default_backend_url, system_get_llm_info,
    system_get_telemetry_bootstrap, system_install_update, system_list_activities,
    system_needs_telemetry_consent, system_open_browser_extension_settings,
    system_open_url_in_browser, system_quit, system_reinit_telemetry,
    system_request_accessibility_permission, system_rotate_telemetry_distinct_id,
    system_test_backend_url,
};
use procedures::thread_procedures::{
    thread_create, thread_delete, thread_generate_title, thread_get_messages, thread_list,
    thread_search_messages, thread_search_threads, thread_switch_branch,
};
use procedures::timeline_procedures::{TimelineAppEvent, TimelineAssetsEvent, timeline_list};

/// Assemble the tauri-specta IPC surface. Lives at the crate root (rather
/// than in `main.rs` or a sub-module) because the per-function macros
/// emitted by `#[tauri::command]` and `#[specta::specta]` are
/// `#[macro_export]`'d into this crate's root macro namespace; calling
/// `collect_commands!` from any other module would require either
/// `use crate::__specta__fn__*;` for every function or path-qualifying
/// each entry below. Keeping the call site here avoids both.
///
/// Every procedure module already on `#[tauri::command]` is registered
/// here. The remaining taurpc-shaped modules (`context_chip`,
/// `third_party`) join the lists as they're ported over.
pub fn build_specta() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new()
        .disable_serde_phases()
        .commands(tauri_specta::collect_commands![
            auth_get_login_token,
            auth_poll_for_login,
            auth_register,
            auth_login,
            auth_logout,
            auth_is_authenticated,
            auth_get_access_token_payload,
            auth_refresh_session,
            auth_resend_verification_email,
            chat_collect_context,
            chat_send_query,
            chat_regenerate,
            chat_cancel_query,
            payment_create_checkout_url,
            payment_is_subscribed,
            settings_get_all,
            settings_get_telemetry,
            settings_set_telemetry,
            settings_get_general,
            settings_set_general,
            settings_get_api,
            settings_set_api,
            settings_get_appearance,
            settings_set_appearance,
            system_check_backend_connection,
            system_get_llm_info,
            system_test_backend_url,
            system_get_default_backend_url,
            system_list_activities,
            system_check_for_update,
            system_install_update,
            system_quit,
            system_check_accessibility_permission,
            system_request_accessibility_permission,
            system_get_browser_extension_state,
            system_open_browser_extension_settings,
            system_open_url_in_browser,
            system_focus_main_window,
            system_get_telemetry_bootstrap,
            system_needs_telemetry_consent,
            system_reinit_telemetry,
            system_rotate_telemetry_distinct_id,
            thread_list,
            thread_create,
            thread_delete,
            thread_get_messages,
            thread_switch_branch,
            thread_generate_title,
            thread_search_threads,
            thread_search_messages,
            timeline_list,
        ])
        .events(tauri_specta::collect_events![
            AuthStateChanged,
            TimelineAppEvent,
            TimelineAssetsEvent,
            BrowserExtensionStatusChanged,
        ])
}

/// Inject build-time URL bake-ins into the process environment so the
/// `std::env::var(...)` call sites in `procedures::*` work in packaged
/// release builds where `.env` isn't available on disk. `build.rs`
/// emits these via `cargo:rustc-env`; here we copy them into the
/// runtime env exactly once at startup, leaving any pre-set values
/// (debug runs via `just dev` that already loaded `.env`) alone.
///
/// SAFETY: must be called before any threads spawn that could read
/// the env concurrently. `main`/`run` invoke this as their first
/// action.
pub fn load_env() {
    for (key, value) in [("WEB_URL", option_env!("WEB_URL"))] {
        if std::env::var_os(key).is_some() {
            continue;
        }
        let Some(v) = value else { continue };
        if v.is_empty() {
            continue;
        }
        // SAFETY: see function-level note.
        unsafe { std::env::set_var(key, v) };
    }
}
