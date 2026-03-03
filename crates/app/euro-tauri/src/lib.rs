#![allow(
    clippy::used_underscore_binding,
    clippy::module_name_repetitions,
    clippy::struct_field_names,
    clippy::too_many_lines
)]

pub mod error;
pub mod procedures;
pub mod shared_types;
pub mod util;
pub mod window;
pub use window::{create as create_window, state::WindowState};
