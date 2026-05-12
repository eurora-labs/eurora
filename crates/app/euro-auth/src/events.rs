//! Backend auth-state event bus.
//!
//! [`AuthManager`](crate::AuthManager) owns a [`tokio::sync::broadcast`]
//! channel that any in-process subscriber can pull from to observe
//! session transitions (login, register, refresh, logout). The cloud
//! settings sync engine consumes this to schedule a pull whenever the
//! signed-in subject changes; future features (e.g. activity / thread
//! observability) can subscribe through the same surface without
//! coupling to the Tauri IPC layer.
//!
//! The Tauri-facing `AuthStateChanged` event in
//! `euro-tauri/src/procedures/auth_procedures.rs` is published from the
//! same place that pumps the bus, so frontend listeners stay
//! authoritative; this type is the backend mirror, not a replacement.
//!
//! ## Why not a `watch` channel
//!
//! A `watch` coalesces missed intermediate values, so a subscriber that
//! arrives mid-transition could observe a "newer" state and skip work
//! tied to the boundary. `broadcast` preserves each transition as a
//! discrete event, which matches the consumer's needs: the sync engine
//! gates on subject *changes*, so dropping intermediate states would
//! break its semantics.

use auth_core::Claims;

/// Snapshot of the auth state at the moment of a transition.
///
/// `claims` is `None` when the transition takes the session into a
/// logged-out state (logout, server-side refresh-token revocation, etc.)
/// and `Some` for every other reported transition (login, register,
/// refresh).
#[derive(Debug, Clone)]
pub struct AuthEvent {
    pub claims: Option<Claims>,
}
