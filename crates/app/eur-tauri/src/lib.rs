#![cfg_attr(
    all(windows, not(test), not(debug_assertions)),
    windows_subsystem = "windows"
)]
// FIXME(qix-): Stuff we want to fix but don't have a lot of time for.
// FIXME(qix-): PRs welcome!
#![allow(
    clippy::used_underscore_binding,
    clippy::module_name_repetitions,
    clippy::struct_field_names,
    clippy::too_many_lines
)]

// mod app;
// pub use app::App;

// pub mod commands;

// pub mod logs;
pub mod launcher;
pub mod procedures;
pub mod shared_types;
pub mod util;
pub mod window;
pub use window::{
    create as create_window, create_hover, create_launcher,
    state::{WindowState, event::ChangeForFrontend},
};
