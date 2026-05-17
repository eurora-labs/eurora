//! Settings sync engine.
//!
//! Owns the network side of the local [`crate::SettingsState`]: pulls
//! the latest cloud blob, pushes local edits, and reconciles the two
//! under optimistic concurrency. Engine instances are clone-cheap
//! (interior `Arc`); the same value is registered in Tauri state and
//! handed to any task spawned by [`SyncEngine::start`].
//!
//! ## State boundary
//!
//! The engine does *not* own the in-memory `SettingsState`; the Tauri
//! command surface does, behind an `Arc<Mutex<SettingsState>>`. The
//! engine takes a clone of that `Arc` and locks it whenever it needs
//! to snapshot the cache (for a PUT) or replace it (after a pull or a
//! 409). Locking is brief — never held across network I/O.
//!
//! ## Auth identity
//!
//! Every network operation resolves the current user via the injected
//! [`super::AuthIdentity`] before doing anything else. The engine
//! enforces account-isolation here: a cache whose `last_user_id`
//! differs from the live JWT subject is discarded in memory (and on
//! disk) before any I/O so a shared machine never leaks one user's
//! appearance / consent into another's session. The freshly-stamped
//! cache then proceeds through the normal pull / first-run-upload
//! ladder.
//!
//! ## Status surface
//!
//! Each operation publishes its progress through a `tokio::sync::watch`
//! channel; subscribers see [`SyncStatus`] transitions and the most
//! recent value is always available via [`SyncEngine::current_status`].
//! The engine retains an internal receiver on the channel so the
//! channel value is updated even when no external subscriber is
//! attached — late subscribers always observe the latest state.
//!
//! ## Coalescing
//!
//! Outbound pushes feed [`super::queue::PushQueue`], a single-slot
//! coalescing queue: N rapid [`SyncEngine::request_push`] calls become
//! at most two `PUT`s — one in flight, one queued behind. The worker
//! that drains the queue is spawned by [`SyncEngine::start`]; until
//! `start` is called, `request_push` still bumps the counter, and the
//! first `wait()` after `start` drains the accumulated requests with a
//! single push.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use bon::Builder;
use chrono::Utc;
use euro_auth::AuthEvent;
use rand::RngExt;
use settings_core::{
    CURRENT_SCHEMA_VERSION, CloudSettings, GetSettingsResponse, PutSettingsAcceptedResponse,
    PutSettingsConflictResponse, PutSettingsRequest,
};
use tokio::sync::{Mutex, broadcast, watch};
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::cloud_cache::CloudSettingsCache;
use crate::state::SettingsState;

use super::client::{PullOutcome, PushOutcome, SettingsTransport};
use super::error::{SyncError, SyncResult};
use super::identity::AuthIdentity;
use super::migrate;
use super::queue::PushQueue;
use super::status::SyncStatus;

/// Exponential-backoff parameters for the push worker's retry loop.
/// Defaults: 1s initial, 60s cap, ±20% jitter.
#[derive(Debug, Clone, Copy, Builder)]
pub struct BackoffConfig {
    #[builder(default = Duration::from_secs(1))]
    pub initial: Duration,
    #[builder(default = Duration::from_secs(60))]
    pub max: Duration,
    /// Multiplicative jitter, applied symmetrically. `0.2` means each
    /// computed delay is scaled by a factor in `[0.8, 1.2]`.
    #[builder(default = 0.2)]
    pub jitter: f64,
}

impl Default for BackoffConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl BackoffConfig {
    /// Return the (jittered) delay for the n-th consecutive retry,
    /// where n=0 yields `initial`. Doubles every step, clamped at
    /// `max`, then scaled by `1 ± jitter`.
    fn delay_for(&self, retry: u32) -> Duration {
        let base = self
            .initial
            .saturating_mul(1u32.checked_shl(retry).unwrap_or(u32::MAX));
        let capped = base.min(self.max);
        if self.jitter <= 0.0 {
            return capped;
        }
        let span = self.jitter.clamp(0.0, 1.0);
        let factor: f64 = 1.0 + rand::rng().random_range(-span..=span);
        capped.mul_f64(factor.max(0.0))
    }
}

/// Cheap-to-clone handle around the engine's interior state.
#[derive(Clone)]
pub struct SyncEngine {
    inner: Arc<EngineInner>,
}

struct EngineInner {
    settings: Arc<Mutex<SettingsState>>,
    transport: Arc<dyn SettingsTransport>,
    identity: Arc<dyn AuthIdentity>,
    queue: PushQueue,
    status_tx: watch::Sender<SyncStatus>,
    /// Keepalive receiver: tokio's `watch::Sender::send` only updates
    /// the channel value when at least one receiver is alive. Without
    /// this field a status transition published while no subscriber is
    /// attached would be silently dropped, and the next subscriber
    /// would observe a stale value.
    _status_keepalive: watch::Receiver<SyncStatus>,
    /// Directory holding `cloud.json`. Production passes the platform
    /// config dir; tests pass a `tempfile::TempDir` path. The engine
    /// reaches into this through [`SettingsState::save_cache`].
    config_dir: PathBuf,
    /// Worker handles, guarded by a `Mutex` so `start` is idempotent
    /// (calling it twice is a no-op).
    workers: Mutex<Option<EngineWorkers>>,
    backoff: BackoffConfig,
    /// Serialises [`SyncEngine::pull_now`] across concurrent callers
    /// (boot pull + auth-event listener + future manual refresh). Each
    /// caller re-resolves identity *after* acquiring the lock, which
    /// closes the account-switch race: a pull queued behind another
    /// pull observes the current subject, not the one captured before
    /// the switch.
    pull_lock: Mutex<()>,
    /// Optional auth-event subscription. Some until `start()` consumes
    /// it to spawn the listener task, then None. Wrapped in a `std`
    /// `Mutex` (not async) because the only access pattern is
    /// `take()` under the `workers` guard.
    auth_events: StdMutex<Option<broadcast::Receiver<AuthEvent>>>,
    /// Latched `true` once any server response (200 GET / 409 PUT)
    /// reports a schema version newer than [`CURRENT_SCHEMA_VERSION`].
    /// Subsequent pushes short-circuit to [`SyncStatus::ServerAhead`]
    /// instead of writing — this build cannot have parsed unknown
    /// sections (no top-level `extras` on [`CloudSettings`]), so
    /// pushing back risks dropping fields the server already holds.
    /// In-memory only; cleared by restart on an upgraded build.
    schema_ahead: AtomicBool,
}

struct EngineWorkers {
    push: JoinHandle<()>,
    /// `None` when the engine was built without an auth-event
    /// subscription (test paths that don't care about
    /// `AuthStateChanged`). Production always wires one through the
    /// builder.
    auth_listener: Option<JoinHandle<()>>,
}

#[bon::bon]
impl SyncEngine {
    /// Build an engine bound to the shared [`SettingsState`], a settings
    /// transport, and an auth-identity resolver. Production callers
    /// should use [`super::client::ReqwestTransport`] +
    /// [`super::identity::AuthManagerIdentity`]; engine tests substitute
    /// in-memory fakes.
    ///
    /// `backoff` is optional; omit it for the production defaults
    /// (1s initial, 60s cap, ±20% jitter) or pass an explicit
    /// [`BackoffConfig`] to keep test retry loops sub-second.
    ///
    /// `auth_events` is also optional. Production wires
    /// [`euro_auth::AuthManager::subscribe`] in so the engine pulls
    /// on subject changes; tests omit it (or substitute a manually
    /// driven channel) when the case under test doesn't involve the
    /// auth bus.
    #[builder]
    pub fn new(
        settings: Arc<Mutex<SettingsState>>,
        transport: Arc<dyn SettingsTransport>,
        identity: Arc<dyn AuthIdentity>,
        config_dir: PathBuf,
        #[builder(default)] backoff: BackoffConfig,
        auth_events: Option<broadcast::Receiver<AuthEvent>>,
    ) -> Self {
        let (status_tx, status_rx) = watch::channel(SyncStatus::default());
        Self {
            inner: Arc::new(EngineInner {
                settings,
                transport,
                identity,
                queue: PushQueue::new(),
                status_tx,
                _status_keepalive: status_rx,
                config_dir,
                workers: Mutex::new(None),
                backoff,
                pull_lock: Mutex::new(()),
                auth_events: StdMutex::new(auth_events),
                schema_ahead: AtomicBool::new(false),
            }),
        }
    }
}

impl SyncEngine {
    /// Subscribe to status updates. Each subscriber sees the current
    /// value immediately and every subsequent transition (the watch
    /// channel coalesces missed intermediate values, which matches the
    /// UI's "always show latest" semantic).
    #[must_use]
    pub fn subscribe(&self) -> watch::Receiver<SyncStatus> {
        self.inner.status_tx.subscribe()
    }

    /// Snapshot of the current status. Useful for IPC handlers that
    /// just want to answer `settings_get_sync_status` without holding
    /// a receiver open.
    #[must_use]
    pub fn current_status(&self) -> SyncStatus {
        self.inner.status_tx.borrow().clone()
    }

    /// Queue an outbound push. Non-blocking; coalesces with any push
    /// already in flight (the worker drains the counter atomically
    /// when it picks up the next request).
    ///
    /// Safe to call before [`SyncEngine::start`]: the counter is bumped
    /// immediately, and the first `wait()` issued by the worker after
    /// `start` drains the accumulated requests with a single push.
    pub fn request_push(&self) {
        self.inner.queue.request();
    }

    /// Spawn the push worker and the auth-event listener. Idempotent:
    /// a second call returns immediately without spawning duplicates.
    ///
    /// `start` does *not* trigger the boot pull — that is a separate
    /// one-shot the caller invokes via [`SyncEngine::pull_now`]. Two
    /// reasons for the split:
    ///
    /// - Production main.rs spawns the boot pull on its own task so
    ///   window creation never blocks on a server round-trip (the
    ///   "brief flip from defaults" trade in `plan.md`).
    /// - Integration tests can exercise the push worker without
    ///   simultaneously triggering a GET they didn't ask for.
    ///
    /// The auth listener is only spawned when an
    /// [`AuthEvent`] receiver was provided to the builder; otherwise
    /// auth-driven pulls are not scheduled, which is the right
    /// behaviour for transport-only unit tests.
    pub async fn start(&self) {
        let mut workers = self.inner.workers.lock().await;
        if workers.is_some() {
            return;
        }
        let push = tokio::spawn(push_worker(self.inner.clone()));
        let auth_rx = self
            .inner
            .auth_events
            .lock()
            .expect("auth_events mutex poisoned")
            .take();
        let auth_listener =
            auth_rx.map(|rx| tokio::spawn(auth_event_listener(self.inner.clone(), rx)));
        *workers = Some(EngineWorkers {
            push,
            auth_listener,
        });
    }

    /// Stop the push worker and any auth listener. Used by tests to
    /// assert the engine shuts down cleanly; production tears down at
    /// process exit.
    pub async fn stop(&self) {
        let mut workers = self.inner.workers.lock().await;
        if let Some(EngineWorkers {
            push,
            auth_listener,
        }) = workers.take()
        {
            push.abort();
            let _ = push.await;
            if let Some(handle) = auth_listener {
                handle.abort();
                let _ = handle.await;
            }
        }
    }

    /// Pull the latest cloud blob and reconcile against the local
    /// cache.
    ///
    /// Resolution order:
    ///
    /// 1. Acquire the pull lock. Concurrent callers (boot pull,
    ///    auth-event listener, manual refresh) are serialised so a
    ///    pull mid-flight never has its post-network writes interleave
    ///    with a second pull's writes — especially across an account
    ///    switch, where the first pull's captured `current_uid` would
    ///    otherwise overwrite the cache stamped for the new user.
    /// 2. Resolve the current authenticated user. `None` → publish
    ///    `LocalOnly` and return without I/O. `Err` → publish `Offline`
    ///    and bubble up.
    /// 3. Enforce account-isolation: if the cache was previously
    ///    stamped for a different user, discard it in memory and on
    ///    disk before any network call.
    /// 4. GET `/settings`, then:
    ///    - `404` → first-run upload via
    ///      [`super::migrate::first_run_request`], stamping the current
    ///      user id onto the cache on success.
    ///    - `200` and server is fresher → replace cache with the
    ///      server's row.
    ///    - `200` and cache is fresher → enqueue a push so the local
    ///      edits propagate.
    pub async fn pull_now(&self) -> SyncResult<SyncStatus> {
        let _guard = self.inner.pull_lock.lock().await;
        let current_uid = match self.resolve_identity().await {
            Ok(Some(uid)) => uid,
            Ok(None) => {
                self.set_status(SyncStatus::LocalOnly);
                return Ok(SyncStatus::LocalOnly);
            }
            Err(err) => return Err(self.publish_error(err)),
        };

        if let Err(err) = self.enforce_account_isolation(current_uid).await {
            return Err(self.publish_error(err));
        }

        self.set_status(SyncStatus::Syncing);

        let outcome = match self.inner.transport.get().await {
            Ok(o) => o,
            Err(e) => return Err(self.publish_error(e)),
        };

        let result = match outcome {
            PullOutcome::NotFound => self.first_run_upload(current_uid).await,
            PullOutcome::Found(response) => self.reconcile_pull(current_uid, response).await,
        };

        match result {
            Ok(status) => {
                self.set_status(status.clone());
                Ok(status)
            }
            Err(e) => Err(self.publish_error(e)),
        }
    }

    /// Reset the cache when its `last_user_id` doesn't match the live
    /// JWT subject. The reset is atomic in memory and persisted to
    /// disk before any I/O so a foreign cache is never PUT under the
    /// new user's credentials and never observed by IPC handlers
    /// reading from `SettingsState`.
    async fn enforce_account_isolation(&self, current_uid: Uuid) -> SyncResult<()> {
        let mut state = self.inner.settings.lock().await;
        let needs_reset = matches!(state.cache.last_user_id, Some(prev) if prev != current_uid);
        if !needs_reset {
            return Ok(());
        }
        tracing::info!(
            previous = %state.cache.last_user_id.expect("checked Some above"),
            current = %current_uid,
            "Cloud settings cache belonged to a different user; resetting to defaults"
        );
        state.cache = CloudSettingsCache {
            last_user_id: Some(current_uid),
            base_updated_at: None,
            settings: CloudSettings::default(),
        };
        state
            .save_cache(&self.inner.config_dir)
            .map_err(SyncError::Internal)?;
        Ok(())
    }

    async fn resolve_identity(&self) -> SyncResult<Option<Uuid>> {
        self.inner.identity.current_user_id().await
    }

    async fn first_run_upload(&self, current_uid: Uuid) -> SyncResult<SyncStatus> {
        let snapshot = self.snapshot_settings().await;
        let request = migrate::first_run_request(&snapshot);

        match self.inner.transport.put(request).await? {
            PushOutcome::Accepted(accepted) => {
                self.apply_accepted(current_uid, accepted).await?;
                Ok(SyncStatus::Synced { at: Utc::now() })
            }
            PushOutcome::Conflict(conflict) => {
                // Another client created the row between our 404 GET
                // and our PUT. We have no basis to claim the local
                // cache is newer, so the server's row wins.
                self.apply_conflict(current_uid, conflict).await?;
                Ok(SyncStatus::Conflict { at: Utc::now() })
            }
        }
    }

    async fn reconcile_pull(
        &self,
        current_uid: Uuid,
        response: GetSettingsResponse,
    ) -> SyncResult<SyncStatus> {
        if response.schema_version > CURRENT_SCHEMA_VERSION {
            // The server's row was written under a schema this build
            // does not understand. Adopting it would mean serialising
            // back from a partial-shape `CloudSettings` — there is no
            // top-level `extras`, so unknown sections would be dropped
            // outright. Refuse to mutate the cache and latch
            // `schema_ahead` so the push worker stops writing too. The
            // user is told to upgrade.
            self.inner.schema_ahead.store(true, Ordering::Relaxed);
            return Err(SyncError::ServerAhead {
                client: CURRENT_SCHEMA_VERSION,
                server: response.schema_version,
            });
        }

        let baseline = {
            let state = self.inner.settings.lock().await;
            state.cache.base_updated_at
        };

        // `baseline.is_none()` means we have never observed a server
        // row before; the server's response is always fresher.
        let server_is_fresher = baseline.is_none_or(|base| response.updated_at > base);

        if server_is_fresher {
            self.replace_cache(current_uid, response.settings, response.updated_at)
                .await?;
            Ok(SyncStatus::Synced { at: Utc::now() })
        } else {
            // Local cache holds edits the server hasn't seen yet.
            // Stamp the current user id (in case this is the first
            // time we've successfully resolved identity for an
            // already-edited cache) and queue a push; the push worker
            // carries it from here.
            self.stamp_user_id(current_uid).await?;
            self.request_push();
            Ok(SyncStatus::Synced { at: Utc::now() })
        }
    }

    /// Snapshot the in-memory cache. The lock is held only across a
    /// `clone`, not across network I/O.
    async fn snapshot_settings(&self) -> CloudSettings {
        self.inner.settings.lock().await.cache.settings.clone()
    }

    /// Build the `PUT /settings` body for an ordinary (non-first-run)
    /// push. Uses the cache's stored OCC baseline (`base_updated_at`)
    /// so the server can detect a race; an unset baseline means we
    /// have never observed a server-side row and the server treats
    /// this as a fresh insert.
    ///
    /// The envelope's `schema_version` is unconditionally
    /// [`CURRENT_SCHEMA_VERSION`]: a client only ever writes blobs
    /// under the schema it was built against. Anything else would be a
    /// lie about which fields the cache actually understands.
    async fn snapshot_for_push(&self) -> PutSettingsRequest {
        let state = self.inner.settings.lock().await;
        let settings = state.cache.settings.clone();
        let base = state.cache.base_updated_at;
        drop(state);

        PutSettingsRequest {
            schema_version: CURRENT_SCHEMA_VERSION,
            settings: serde_json::to_value(&settings)
                .expect("CloudSettings serialises into serde_json::Value"),
            base_updated_at: base,
        }
    }

    /// Apply a 200 response to the local cache: parse the server blob,
    /// stamp the OCC baseline + owner, persist. Field-level invariants
    /// (e.g. UI scale bounds) are enforced at deserialization by the
    /// field types themselves; no separate sanitize pass is needed.
    ///
    /// The envelope's `schema_version` is not threaded through here —
    /// callers must already have rejected ahead-of-schema responses
    /// before invoking this, so anything reaching `replace_cache` is
    /// guaranteed parseable under the current shape.
    async fn replace_cache(
        &self,
        current_uid: Uuid,
        settings_value: serde_json::Value,
        updated_at: chrono::DateTime<Utc>,
    ) -> SyncResult<()> {
        let incoming: CloudSettings = serde_json::from_value(settings_value)?;

        let mut state = self.inner.settings.lock().await;
        state.cache = CloudSettingsCache {
            last_user_id: Some(current_uid),
            settings: incoming,
            base_updated_at: Some(updated_at),
        };
        state
            .save_cache(&self.inner.config_dir)
            .map_err(SyncError::Internal)?;
        Ok(())
    }

    /// Apply a 200 response to a `PUT` — the OCC baseline and the
    /// owner stamp change; the blob itself was what the client just
    /// sent. Persisted so the next `base_updated_at` matches the
    /// server.
    async fn apply_accepted(
        &self,
        current_uid: Uuid,
        accepted: PutSettingsAcceptedResponse,
    ) -> SyncResult<()> {
        let mut state = self.inner.settings.lock().await;
        state.cache.base_updated_at = Some(accepted.updated_at);
        state.cache.last_user_id = Some(current_uid);
        state
            .save_cache(&self.inner.config_dir)
            .map_err(SyncError::Internal)?;
        Ok(())
    }

    async fn apply_conflict(
        &self,
        current_uid: Uuid,
        conflict: PutSettingsConflictResponse,
    ) -> SyncResult<()> {
        if conflict.schema_version > CURRENT_SCHEMA_VERSION {
            // Same reasoning as `reconcile_pull`: the server's current
            // row is on a schema we cannot fully parse. Latch
            // `schema_ahead` and stop writing — the previous push has
            // already been rejected by the server, so the cache's edits
            // are simply lost; the user must upgrade to recover.
            self.inner.schema_ahead.store(true, Ordering::Relaxed);
            return Err(SyncError::ServerAhead {
                client: CURRENT_SCHEMA_VERSION,
                server: conflict.schema_version,
            });
        }
        self.replace_cache(current_uid, conflict.current, conflict.updated_at)
            .await
    }

    /// Stamp `last_user_id` onto the cache without otherwise touching
    /// it. Used when the engine has just confirmed identity but the
    /// cache will keep its existing contents (e.g. the "local fresher
    /// than server" branch of `reconcile_pull`).
    async fn stamp_user_id(&self, current_uid: Uuid) -> SyncResult<()> {
        let mut state = self.inner.settings.lock().await;
        if state.cache.last_user_id == Some(current_uid) {
            return Ok(());
        }
        state.cache.last_user_id = Some(current_uid);
        state
            .save_cache(&self.inner.config_dir)
            .map_err(SyncError::Internal)?;
        Ok(())
    }

    /// Publish `status` to the watch channel, suppressing no-op
    /// transitions so subscribers don't see `changed()` fire on
    /// `Syncing → Syncing` etc.
    fn set_status(&self, status: SyncStatus) {
        self.inner.status_tx.send_if_modified(|current| {
            if *current == status {
                return false;
            }
            *current = status;
            true
        });
    }

    /// Project a [`SyncError`] into a status update and publish it.
    /// Returns the error untouched so the caller can still propagate.
    fn publish_error(&self, err: SyncError) -> SyncError {
        self.set_status(err.into_status());
        err
    }
}

/// Push-worker body: drain the queue, resolve identity, snapshot the
/// cache, PUT, reconcile. On transient failures, sleep for a jittered
/// exponential backoff and retry the same intent; on conflict, replace
/// the local cache and yield to the next intent; on a definitive
/// logout, drop the in-flight intent and park on the queue; on
/// permanent failures, give up on this intent and wait for the next
/// request.
async fn push_worker(inner: Arc<EngineInner>) {
    let mut retry: u32 = 0;
    loop {
        if retry == 0 {
            let _drained = inner.queue.wait().await;
        } else {
            let delay = inner.backoff.delay_for(retry - 1);
            tokio::time::sleep(delay).await;
        }

        let engine = SyncEngine {
            inner: inner.clone(),
        };

        let current_uid = match engine.resolve_identity().await {
            Ok(Some(uid)) => uid,
            Ok(None) => {
                // No authenticated user — drop the intent and park.
                // The next `request_push` (or AuthStateChanged in
                // phase 8) wakes the worker again.
                engine.set_status(SyncStatus::LocalOnly);
                retry = 0;
                continue;
            }
            Err(err) => {
                let retryable = err.is_retryable();
                let _ = engine.publish_error(err);
                retry = if retryable {
                    retry.saturating_add(1)
                } else {
                    0
                };
                continue;
            }
        };

        if let Err(err) = engine.enforce_account_isolation(current_uid).await {
            let retryable = err.is_retryable();
            let _ = engine.publish_error(err);
            retry = if retryable {
                retry.saturating_add(1)
            } else {
                0
            };
            continue;
        }

        // A previous response told us the server is ahead of this
        // build's schema. Drop the intent and park rather than write
        // back from a partial-shape cache; the latch clears only on
        // restart with an upgraded build.
        if inner.schema_ahead.load(Ordering::Relaxed) {
            engine.set_status(SyncStatus::ServerAhead);
            retry = 0;
            continue;
        }

        engine.set_status(SyncStatus::Syncing);

        let request = engine.snapshot_for_push().await;
        match inner.transport.put(request).await {
            Ok(PushOutcome::Accepted(accepted)) => {
                if let Err(e) = engine.apply_accepted(current_uid, accepted).await {
                    let retryable = e.is_retryable();
                    let _ = engine.publish_error(e);
                    retry = if retryable {
                        retry.saturating_add(1)
                    } else {
                        0
                    };
                    continue;
                }
                engine.set_status(SyncStatus::Synced { at: Utc::now() });
                retry = 0;
            }
            Ok(PushOutcome::Conflict(conflict)) => {
                if let Err(e) = engine.apply_conflict(current_uid, conflict).await {
                    let retryable = e.is_retryable();
                    let _ = engine.publish_error(e);
                    retry = if retryable {
                        retry.saturating_add(1)
                    } else {
                        0
                    };
                    continue;
                }
                engine.set_status(SyncStatus::Conflict { at: Utc::now() });
                retry = 0;
            }
            Err(err) => {
                let retryable = err.is_retryable();
                let _ = engine.publish_error(err);
                if retryable {
                    retry = retry.saturating_add(1);
                } else {
                    retry = 0;
                }
            }
        }
    }
}

/// Listener body for the auth-event bus.
///
/// Tracks the subject of the most recently observed `Some(claims)`
/// event so a token refresh (same `sub`, new `exp`) does not trigger a
/// pull. A subject change — including the None → Some transition after
/// a logout, and a switch from user A to user B — calls
/// [`SyncEngine::pull_now`]; the pull's internal account-isolation
/// step is what actually wipes any stale cache.
///
/// `last_seen_subject` is seeded from `cache.last_user_id` so the very
/// first event after boot is a no-op when it matches the persisted
/// owner (the typical "boot pull already ran, AuthStateChanged for the
/// same user arrives a moment later" case).
async fn auth_event_listener(inner: Arc<EngineInner>, mut rx: broadcast::Receiver<AuthEvent>) {
    let mut last_seen_subject: Option<Uuid> = inner.settings.lock().await.cache.last_user_id;

    loop {
        match rx.recv().await {
            Ok(AuthEvent {
                claims: Some(claims),
            }) => {
                let new_subject = match Uuid::parse_str(&claims.sub) {
                    Ok(uid) => Some(uid),
                    Err(err) => {
                        // A non-UUID `sub` is a server-side invariant
                        // violation. Surface loudly but keep the
                        // listener alive: the next valid event still
                        // needs to drive a pull.
                        tracing::error!(
                            error = %err,
                            "AuthEvent carried a non-UUID subject; ignoring"
                        );
                        continue;
                    }
                };

                if new_subject == last_seen_subject {
                    // Same subject (token refresh, redundant emit) —
                    // nothing for the sync engine to do.
                    continue;
                }

                let engine = SyncEngine {
                    inner: inner.clone(),
                };
                match engine.pull_now().await {
                    Ok(_) => {
                        last_seen_subject = new_subject;
                    }
                    Err(err) => {
                        // Pull failed (transient transport, decode,
                        // etc.). Don't update `last_seen_subject` —
                        // the next event for the same user should
                        // retry rather than be deduped away.
                        tracing::warn!(
                            error = %err,
                            "Settings pull triggered by AuthEvent failed"
                        );
                    }
                }
            }
            Ok(AuthEvent { claims: None }) => {
                // Logout. Per plan.md: flip status to LocalOnly, keep
                // the cache intact so offline-after-logout reads still
                // work, and clear `last_seen_subject` so a subsequent
                // login (even as the same user) re-triggers a pull.
                let engine = SyncEngine {
                    inner: inner.clone(),
                };
                engine.set_status(SyncStatus::LocalOnly);
                last_seen_subject = None;
            }
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                tracing::warn!(
                    skipped,
                    "AuthEvent listener lagged behind the bus; resuming"
                );
            }
            Err(broadcast::error::RecvError::Closed) => {
                // The manager (and therefore the channel) was dropped.
                // Nothing meaningful to do but exit.
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use std::path::PathBuf;

    use super::*;
    use crate::sync::identity::AuthIdentity;

    /// Stub transport used by the dedup test: the engine's worker
    /// surface is exercised through the public API in `tests/sync.rs`,
    /// so this only needs to be enough to construct an engine.
    struct NoopTransport;

    #[async_trait]
    impl SettingsTransport for NoopTransport {
        async fn get(&self) -> SyncResult<PullOutcome> {
            Ok(PullOutcome::NotFound)
        }
        async fn put(&self, _: PutSettingsRequest) -> SyncResult<PushOutcome> {
            unreachable!("not used by dedup test")
        }
        async fn delete(&self) -> SyncResult<()> {
            Ok(())
        }
    }

    /// Identity stub that always reports the same user (or no user).
    struct StaticIdentity(Option<Uuid>);

    #[async_trait]
    impl AuthIdentity for StaticIdentity {
        async fn current_user_id(&self) -> SyncResult<Option<Uuid>> {
            Ok(self.0)
        }
    }

    fn engine_for_test() -> SyncEngine {
        let state = Arc::new(Mutex::new(SettingsState::default()));
        let transport: Arc<dyn SettingsTransport> = Arc::new(NoopTransport);
        let identity: Arc<dyn AuthIdentity> = Arc::new(StaticIdentity(None));
        SyncEngine::builder()
            .settings(state)
            .transport(transport)
            .identity(identity)
            .config_dir(PathBuf::from("/tmp/euro-settings-test"))
            .build()
    }

    #[tokio::test]
    async fn set_status_dedupes_identical_writes() {
        let engine = engine_for_test();
        let mut rx = engine.subscribe();
        rx.mark_unchanged();

        // First write is a real transition (LocalOnly → Syncing).
        engine.set_status(SyncStatus::Syncing);
        rx.changed().await.expect("first transition fires");
        assert!(matches!(*rx.borrow_and_update(), SyncStatus::Syncing));

        // Two redundant writes back-to-back. With `send_if_modified`
        // neither should fire `changed()`.
        engine.set_status(SyncStatus::Syncing);
        engine.set_status(SyncStatus::Syncing);

        let result = tokio::time::timeout(Duration::from_millis(50), rx.changed()).await;
        assert!(
            result.is_err(),
            "redundant Syncing transitions must not fire changed()"
        );
    }

    #[tokio::test]
    async fn set_status_publishes_even_with_no_external_subscriber() {
        // Regression test for the keepalive receiver: without it,
        // `send_if_modified` fails when no subscriber is attached and
        // the channel value stays at the default. A subscriber that
        // arrives later must still see the latest published status.
        let engine = engine_for_test();
        engine.set_status(SyncStatus::Syncing);

        let rx = engine.subscribe();
        assert!(matches!(*rx.borrow(), SyncStatus::Syncing));
        assert!(matches!(engine.current_status(), SyncStatus::Syncing));
    }

    #[test]
    fn backoff_caps_at_max() {
        let cfg = BackoffConfig::builder()
            .initial(Duration::from_millis(10))
            .max(Duration::from_millis(50))
            .jitter(0.0)
            .build();
        assert_eq!(cfg.delay_for(0), Duration::from_millis(10));
        assert_eq!(cfg.delay_for(1), Duration::from_millis(20));
        assert_eq!(cfg.delay_for(2), Duration::from_millis(40));
        assert_eq!(cfg.delay_for(3), Duration::from_millis(50));
        assert_eq!(cfg.delay_for(20), Duration::from_millis(50));
    }

    #[test]
    fn backoff_jitter_stays_within_band() {
        // `jitter` is omitted — the builder default (0.2) is exactly
        // what this test exercises.
        let cfg = BackoffConfig::builder()
            .initial(Duration::from_millis(100))
            .max(Duration::from_millis(1000))
            .build();
        for _ in 0..200 {
            let d = cfg.delay_for(0);
            // ±20% of 100ms = [80ms, 120ms]; ±1ms for rounding.
            assert!(
                d >= Duration::from_millis(80) && d <= Duration::from_millis(121),
                "jittered delay {:?} outside [80ms, 120ms]",
                d
            );
        }
    }
}
