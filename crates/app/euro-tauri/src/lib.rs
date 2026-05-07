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
