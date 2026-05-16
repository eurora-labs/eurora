//! Tauri integration for [`AuthManager`].
//!
//! Provides:
//!
//! - [`AuthStateChanged`] — the typed `tauri-specta` event that bridges
//!   backend auth transitions to the frontend. Single declaration site,
//!   so the TypeScript bindings stay in sync with the Rust type by
//!   construction.
//! - [`install`] — registers an [`AuthManager`] as Tauri-managed state
//!   *and* spawns the bus → frontend bridge task in one call, so app
//!   entry points don't need to remember to wire both.
//! - [`auth_manager`] — convenience lookup for IPC procedures.
//!
//! ## Bridge ordering
//!
//! [`install`] subscribes to the bus *before* spawning the bridge task,
//! so transitions that race the spawn aren't lost: the subscriber
//! receiver is created on the calling task and only then moved into the
//! background. Procedures wired up immediately after `install` (e.g.
//! during `setup`) can safely call mutating `AuthManager` methods
//! knowing the bridge will pick up every emitted event.

use ::tauri::{AppHandle, Manager, Runtime};
use auth_core::Claims;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::Event;

use crate::manager::AuthManager;

/// Backend → frontend auth-state transition. Mirrors the shape of
/// [`crate::AuthEvent`] for the JS side; `claims == None` means signed out.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct AuthStateChanged {
    pub claims: Option<Claims>,
}

/// Register `manager` in Tauri state and spawn the bus → frontend
/// bridge.
///
/// After this returns, any future `AuthManager::login/register/refresh/
/// logout` call (regardless of which subsystem makes it) automatically
/// fans out to:
/// 1. The in-process [`crate::AuthEvent`] bus — for sync engines, observability,
///    etc.
/// 2. The Tauri-side [`AuthStateChanged`] event — for the frontend.
///
/// The bridge is the *sole* publisher of `AuthStateChanged`, so IPC
/// procedures must not emit it directly — doing so would cause
/// double-emit races on every transition.
pub fn install<R: Runtime>(app: &AppHandle<R>, manager: AuthManager) {
    let mut rx = manager.subscribe();
    app.manage(manager);

    let app_handle = app.clone();
    ::tauri::async_runtime::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    if let Err(e) = (AuthStateChanged {
                        claims: event.claims,
                    })
                    .emit(&app_handle)
                    {
                        tracing::warn!(error = %e, "failed to emit AuthStateChanged");
                    }
                }
                Err(::tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    // A slow frame on the Tauri runtime dropped events
                    // we never got to forward. Auth state is observed
                    // by `claims.sub`, not by a chain of deltas, so the
                    // next event re-syncs the frontend. Logging the gap
                    // is enough.
                    tracing::warn!(
                        skipped,
                        "AuthStateChanged bridge lagged behind the AuthEvent bus"
                    );
                }
                Err(::tokio::sync::broadcast::error::RecvError::Closed) => {
                    // The manager (and therefore the sender) was
                    // dropped — happens at app shutdown. Exit cleanly.
                    return;
                }
            }
        }
    });
}

/// Look up the [`AuthManager`] registered by [`install`]. Returns `None`
/// only when called before `install` (i.e. during shutdown, after state
/// has been torn down). Procedures map `None` to a typed "state
/// unavailable" error rather than retrying.
#[must_use]
pub fn auth_manager<R: Runtime>(app: &AppHandle<R>) -> Option<AuthManager> {
    app.try_state::<AuthManager>().map(|s| s.inner().clone())
}
