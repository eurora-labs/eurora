//! tauri-specta IPC surface assembly.
//!
//! `tauri_specta::collect_commands!` expands each entry into a reference
//! to two `#[tauri::command]`-emitted helper macros (`__cmd__$name` and
//! `__tauri_command_name_$name`). Those helpers are defined in the
//! procedure modules where each command lives — for desktop-only
//! procedures that's under `crate::procedures::*`; for shared
//! thread/chat procedures that's under `euro_thread::commands::*`. We
//! pass fully qualified paths to `collect_commands!` and let
//! module-relative macro resolution find them.

use crate::procedures::activity::{SavedActivityCreated, SavedActivityEnded};
use crate::procedures::system::{BrowserExtensionStatusChanged, ConsentGate};
use crate::procedures::timeline::{TimelineAppEvent, TimelineAssetsEvent};
use euro_auth::tauri::AuthStateChanged;

/// Assemble the tauri-specta IPC surface — every typed command and event
/// the desktop frontend talks to. Returned as a [`tauri_specta::Builder`]
/// so callers can both wire it into `tauri::Builder` (`invoke_handler`,
/// `setup` -> `mount_events`) and export the matching TypeScript
/// bindings.
pub fn build_specta() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new()
        .disable_serde_phases()
        .commands(tauri_specta::collect_commands![
            crate::procedures::auth::auth_get_login_token,
            crate::procedures::auth::auth_poll_for_login,
            crate::procedures::auth::auth_register,
            crate::procedures::auth::auth_login,
            crate::procedures::auth::auth_logout,
            crate::procedures::auth::auth_is_authenticated,
            crate::procedures::auth::auth_get_access_token_payload,
            crate::procedures::auth::auth_refresh_session,
            crate::procedures::auth::auth_resend_verification_email,
            crate::procedures::activity::activity_list,
            euro_thread::commands::chat::chat_collect_context,
            euro_thread::commands::chat::chat_send_query,
            euro_thread::commands::chat::chat_regenerate,
            euro_thread::commands::chat::chat_cancel_query,
            crate::procedures::payment::payment_create_checkout_url,
            crate::procedures::payment::payment_is_subscribed,
            crate::procedures::settings::settings_get_general,
            crate::procedures::settings::settings_set_general,
            crate::procedures::settings::settings_get_api,
            crate::procedures::settings::settings_set_api,
            crate::procedures::settings::settings_get_shared,
            crate::procedures::settings::settings_set_shared,
            crate::procedures::settings::settings_get_desktop,
            crate::procedures::settings::settings_set_desktop,
            crate::procedures::settings::settings_record_telemetry_consent,
            crate::procedures::settings::settings_get_local_telemetry,
            crate::procedures::settings::settings_get_telemetry_consent,
            crate::procedures::system::system_check_backend_connection,
            crate::procedures::system::system_get_llm_info,
            crate::procedures::system::system_test_backend_url,
            crate::procedures::system::system_get_default_backend_url,
            crate::procedures::system::system_list_activities,
            crate::procedures::system::system_check_for_update,
            crate::procedures::system::system_install_update,
            crate::procedures::system::system_quit,
            crate::procedures::system::system_check_accessibility_permission,
            crate::procedures::system::system_request_accessibility_permission,
            crate::procedures::system::system_get_browser_extension_state,
            crate::procedures::system::system_open_browser_extension_settings,
            crate::procedures::system::system_open_url_in_browser,
            crate::procedures::system::system_focus_main_window,
            crate::procedures::system::system_get_telemetry_bootstrap,
            crate::procedures::system::frontend_ready,
            crate::procedures::system::system_reinit_telemetry,
            crate::procedures::system::system_rotate_telemetry_distinct_id,
            euro_thread::commands::thread::thread_list,
            euro_thread::commands::thread::thread_create,
            euro_thread::commands::thread::thread_delete,
            euro_thread::commands::thread::thread_get_messages,
            euro_thread::commands::thread::thread_switch_branch,
            euro_thread::commands::thread::thread_generate_title,
            euro_thread::commands::thread::thread_search_threads,
            euro_thread::commands::thread::thread_search_messages,
        ])
        .events(tauri_specta::collect_events![
            AuthStateChanged,
            TimelineAppEvent,
            TimelineAssetsEvent,
            SavedActivityCreated,
            SavedActivityEnded,
            BrowserExtensionStatusChanged,
            ConsentGate,
        ])
}
