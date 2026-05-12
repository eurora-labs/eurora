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
use std::time::Duration;

use chrono::Utc;
use rand::RngExt;
use settings_core::{
    CURRENT_SCHEMA_VERSION, CloudSettings, GetSettingsResponse, PutSettingsAcceptedResponse,
    PutSettingsConflictResponse, PutSettingsRequest,
};
use tokio::sync::{Mutex, watch};
use tokio::task::JoinHandle;

use crate::cloud_cache::CloudSettingsCache;
use crate::state::SettingsState;

use super::client::{PullOutcome, PushOutcome, SettingsTransport};
use super::error::{SyncError, SyncResult};
use super::migrate;
use super::queue::PushQueue;
use super::status::SyncStatus;

/// Exponential-backoff parameters for the push worker's retry loop.
/// Defaults: 1s initial, 60s cap, ±20% jitter.
#[derive(Debug, Clone, Copy)]
pub struct BackoffConfig {
    pub initial: Duration,
    pub max: Duration,
    /// Multiplicative jitter, applied symmetrically. `0.2` means each
    /// computed delay is scaled by a factor in `[0.8, 1.2]`.
    pub jitter: f64,
}

impl Default for BackoffConfig {
    fn default() -> Self {
        Self {
            initial: Duration::from_secs(1),
            max: Duration::from_secs(60),
            jitter: 0.2,
        }
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
}

struct EngineWorkers {
    push: JoinHandle<()>,
}

impl SyncEngine {
    /// Build an engine bound to the shared [`SettingsState`] and a
    /// settings transport. Production callers should use
    /// [`super::client::ReqwestTransport`]; engine tests substitute an
    /// in-memory fake.
    #[must_use]
    pub fn new(
        settings: Arc<Mutex<SettingsState>>,
        transport: Arc<dyn SettingsTransport>,
        config_dir: PathBuf,
    ) -> Self {
        Self::with_backoff(settings, transport, config_dir, BackoffConfig::default())
    }

    /// Same as [`SyncEngine::new`] but with explicit backoff parameters
    /// — used by integration tests to keep the retry loop sub-second.
    #[must_use]
    pub fn with_backoff(
        settings: Arc<Mutex<SettingsState>>,
        transport: Arc<dyn SettingsTransport>,
        config_dir: PathBuf,
        backoff: BackoffConfig,
    ) -> Self {
        let (status_tx, status_rx) = watch::channel(SyncStatus::default());
        Self {
            inner: Arc::new(EngineInner {
                settings,
                transport,
                queue: PushQueue::new(),
                status_tx,
                _status_keepalive: status_rx,
                config_dir,
                workers: Mutex::new(None),
                backoff,
            }),
        }
    }

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

    /// Spawn the push worker. Idempotent: a second call returns
    /// immediately.
    pub async fn start(&self) {
        let mut workers = self.inner.workers.lock().await;
        if workers.is_some() {
            return;
        }
        let inner = self.inner.clone();
        let push = tokio::spawn(push_worker(inner));
        *workers = Some(EngineWorkers { push });
    }

    /// Stop the push worker. Used by tests to assert the engine
    /// shutdown cleanly; production tears down at process exit.
    pub async fn stop(&self) {
        let mut workers = self.inner.workers.lock().await;
        if let Some(EngineWorkers { push }) = workers.take() {
            push.abort();
            let _ = push.await;
        }
    }

    /// Pull the latest cloud blob and reconcile against the local
    /// cache.
    ///
    /// Branches:
    ///
    /// - `404` → first-run upload via
    ///   [`super::migrate::first_run_request`].
    /// - `200` and server is fresher → replace cache.
    /// - `200` and cache is fresher → enqueue a push so the local
    ///   edits propagate.
    pub async fn pull_now(&self) -> SyncResult<SyncStatus> {
        self.set_status(SyncStatus::Syncing);

        let outcome = match self.inner.transport.get().await {
            Ok(o) => o,
            Err(e) => return Err(self.publish_error(e)),
        };

        let result = match outcome {
            PullOutcome::NotFound => self.first_run_upload().await,
            PullOutcome::Found(response) => self.reconcile_pull(response).await,
        };

        match result {
            Ok(status) => {
                self.set_status(status.clone());
                Ok(status)
            }
            Err(e) => Err(self.publish_error(e)),
        }
    }

    async fn first_run_upload(&self) -> SyncResult<SyncStatus> {
        let snapshot = self.snapshot_settings().await;
        let request = migrate::first_run_request(&snapshot);

        match self.inner.transport.put(request).await? {
            PushOutcome::Accepted(accepted) => {
                self.apply_accepted(accepted).await?;
                Ok(SyncStatus::Synced { at: Utc::now() })
            }
            PushOutcome::Conflict(conflict) => {
                // Another client created the row between our 404 GET
                // and our PUT. We have no basis to claim the local
                // cache is newer, so the server's row wins.
                self.apply_conflict(conflict).await?;
                Ok(SyncStatus::Conflict { at: Utc::now() })
            }
        }
    }

    async fn reconcile_pull(&self, response: GetSettingsResponse) -> SyncResult<SyncStatus> {
        let baseline = {
            let state = self.inner.settings.lock().await;
            state.cache.base_updated_at
        };

        // `baseline.is_none()` means we have never observed a server
        // row before; the server's response is always fresher.
        let server_is_fresher = baseline.is_none_or(|base| response.updated_at > base);

        if server_is_fresher {
            self.replace_cache(
                response.settings,
                response.schema_version,
                response.updated_at,
            )
            .await?;
            Ok(SyncStatus::Synced { at: Utc::now() })
        } else {
            // Local cache holds edits the server hasn't seen yet.
            // Queue a push and report `Synced` against the pull
            // round-trip; the push worker carries it from here.
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
    async fn snapshot_for_push(&self) -> PutSettingsRequest {
        let state = self.inner.settings.lock().await;
        let settings = state.cache.settings.clone();
        let base = state.cache.base_updated_at;
        drop(state);

        PutSettingsRequest {
            schema_version: settings.schema_version.max(CURRENT_SCHEMA_VERSION),
            settings: serde_json::to_value(&settings)
                .expect("CloudSettings serialises into serde_json::Value"),
            base_updated_at: base,
        }
    }

    /// Apply a 200 response to the local cache: parse the server blob,
    /// sanitize, stamp the OCC baseline, persist.
    async fn replace_cache(
        &self,
        settings_value: serde_json::Value,
        schema_version: u32,
        updated_at: chrono::DateTime<Utc>,
    ) -> SyncResult<()> {
        let mut incoming: CloudSettings = serde_json::from_value(settings_value)?;
        if schema_version > CURRENT_SCHEMA_VERSION {
            tracing::warn!(
                client = CURRENT_SCHEMA_VERSION,
                server = schema_version,
                "Server pushed a newer settings schema than this build understands; \
                 unknown fields will round-trip through `extras` but the schema_version \
                 column is being preserved verbatim."
            );
        }
        // Preserve the server's `schema_version` verbatim — a value
        // newer than the client's `CURRENT_SCHEMA_VERSION` is
        // load-bearing for round-tripping through older clients.
        incoming.schema_version = schema_version;
        incoming.sanitize();

        let mut state = self.inner.settings.lock().await;
        state.cache = CloudSettingsCache {
            last_user_id: state.cache.last_user_id,
            settings: incoming,
            base_updated_at: Some(updated_at),
        };
        state
            .save_cache(&self.inner.config_dir)
            .map_err(SyncError::Internal)?;
        Ok(())
    }

    /// Apply a 200 response to a `PUT` — only `schema_version` and
    /// the OCC baseline change; the blob itself was what the client
    /// just sent. Persisted so the next `base_updated_at` matches the
    /// server.
    async fn apply_accepted(&self, accepted: PutSettingsAcceptedResponse) -> SyncResult<()> {
        let mut state = self.inner.settings.lock().await;
        state.cache.settings.schema_version = accepted.schema_version;
        state.cache.base_updated_at = Some(accepted.updated_at);
        state
            .save_cache(&self.inner.config_dir)
            .map_err(SyncError::Internal)?;
        Ok(())
    }

    async fn apply_conflict(&self, conflict: PutSettingsConflictResponse) -> SyncResult<()> {
        self.replace_cache(
            conflict.current,
            conflict.schema_version,
            conflict.updated_at,
        )
        .await
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

/// Push-worker body: drain the queue, snapshot the cache, PUT,
/// reconcile. On transient failures, sleep for a jittered exponential
/// backoff and retry the same intent; on conflict, replace the local
/// cache and yield to the next intent; on permanent failures, give up
/// on this intent and wait for the next request.
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
        engine.set_status(SyncStatus::Syncing);

        let request = engine.snapshot_for_push().await;
        match inner.transport.put(request).await {
            Ok(PushOutcome::Accepted(accepted)) => {
                if let Err(e) = engine.apply_accepted(accepted).await {
                    let _ = engine.publish_error(e);
                    retry = retry.saturating_add(1);
                    continue;
                }
                engine.set_status(SyncStatus::Synced { at: Utc::now() });
                retry = 0;
            }
            Ok(PushOutcome::Conflict(conflict)) => {
                if let Err(e) = engine.apply_conflict(conflict).await {
                    let _ = engine.publish_error(e);
                    retry = retry.saturating_add(1);
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

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use std::path::PathBuf;

    use super::*;

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

    fn engine_for_test() -> SyncEngine {
        let state = Arc::new(Mutex::new(SettingsState::default()));
        let transport: Arc<dyn SettingsTransport> = Arc::new(NoopTransport);
        SyncEngine::new(state, transport, PathBuf::from("/tmp/euro-settings-test"))
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
        let cfg = BackoffConfig {
            initial: Duration::from_millis(10),
            max: Duration::from_millis(50),
            jitter: 0.0,
        };
        assert_eq!(cfg.delay_for(0), Duration::from_millis(10));
        assert_eq!(cfg.delay_for(1), Duration::from_millis(20));
        assert_eq!(cfg.delay_for(2), Duration::from_millis(40));
        assert_eq!(cfg.delay_for(3), Duration::from_millis(50));
        assert_eq!(cfg.delay_for(20), Duration::from_millis(50));
    }

    #[test]
    fn backoff_jitter_stays_within_band() {
        let cfg = BackoffConfig {
            initial: Duration::from_millis(100),
            max: Duration::from_millis(1000),
            jitter: 0.2,
        };
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
