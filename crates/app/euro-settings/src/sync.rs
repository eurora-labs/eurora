//! Cloud-settings sync engine.
//!
//! The sync engine is the network side of the local
//! [`crate::SettingsState`]: it pulls the latest server blob into the
//! local cache, pushes local edits back, and reconciles the two under
//! optimistic concurrency. Each operation publishes a
//! [`SyncStatus`] update through a `tokio::sync::watch` channel.
//!
//! The engine is constructed in `crates/app/euro-tauri/src/main.rs`
//! and registered with Tauri state; calling [`SyncEngine::start`]
//! spawns the push worker, and IPC handlers fan out into
//! [`SyncEngine::request_push`] / [`SyncEngine::pull_now`] from there.
//!
//! ## Layering
//!
//! | Module     | Role                                                              |
//! |------------|-------------------------------------------------------------------|
//! | `status`   | [`SyncStatus`] enum + watch types.                                |
//! | `error`    | Typed [`SyncError`]; classifies into a [`SyncStatus`].            |
//! | `queue`    | Single-slot coalescing push queue.                                |
//! | `client`   | HTTP transport trait + [`client::ReqwestTransport`] production.   |
//! | `identity` | Auth-identity trait + [`identity::AuthManagerIdentity`] prod impl.|
//! | `migrate`  | Helper that shapes a first-run upload PUT body.                   |
//! | `engine`   | The reconciliation logic; owns the worker and the watch channel. |

pub mod client;
mod engine;
mod error;
pub mod identity;
mod migrate;
mod queue;
mod status;

pub use client::{PullOutcome, PushOutcome, ReqwestTransport, SettingsTransport};
pub use engine::{BackoffConfig, SyncEngine};
pub use error::{SyncError, SyncResult};
pub use identity::{AuthIdentity, AuthManagerIdentity};
pub use status::SyncStatus;
