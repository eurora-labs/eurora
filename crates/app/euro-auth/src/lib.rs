mod client;
mod error;
mod events;
mod manager;
mod tokens;

#[cfg(feature = "tauri")]
pub mod tauri;

pub use auth_core::*;
pub use client::*;
pub use error::{AuthError, AuthResult};
pub use events::AuthEvent;
pub use manager::*;
