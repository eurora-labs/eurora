//! Async request/response correlation.
//!
//! Both `be-thread-service`'s `ChatRemoteBus` and `euro-bridge`'s
//! `BridgeService` allocate per-request IDs, park the caller on a oneshot,
//! and route inbound responses back by ID. This crate factors out that
//! pattern so the cleanup, timeout, and cancellation logic lives in one
//! place — including a drop-guarded entry so any exit path (success,
//! timeout, cancel, panic) frees the slot.
//!
//! Typical usage:
//!
//! ```rust,ignore
//! let correlator = RequestCorrelator::<u32, MyResp, MyErr>::new();
//! let id = self.next_id.fetch_add(1, Ordering::Relaxed);
//! let guard = correlator.register(id);
//! send_request_frame(id, ...).await?;
//! match guard.wait_cancellable(timeout, &cancel).await {
//!     Ok(Ok(value)) => Ok(value),
//!     Ok(Err(err))  => Err(err.into()),
//!     Err(WaitError::Timeout)   => Err(MyTransport::Timeout),
//!     Err(WaitError::Cancelled) => Err(MyTransport::Cancelled),
//!     Err(WaitError::SenderDropped) => Err(MyTransport::DroppedBeforeResponse),
//! }
//! ```
//!
//! From the inbound side:
//!
//! ```rust,ignore
//! correlator.resolve(id, Ok(response));   // or Err(err)
//! ```
//!
//! Late `resolve` calls for keys that are no longer pending are silent
//! no-ops, matching the assumption that the caller has already moved on
//! (timed out, cancelled, or been shut down).

#![warn(missing_docs)]

use std::hash::Hash;
use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

/// In-flight request registry.
///
/// `K` is the correlation key (typically `u32` allocated from an
/// `AtomicU32`); `V` is the success payload; `E` is the structured error
/// the remote side may surface. The full success/error pair returned to
/// the waiter is `Result<V, E>` — exactly what the remote sent.
///
/// Cheap to clone (interior `Arc`); pass clones into spawned tasks that
/// need to call [`Self::resolve`] without holding a `&self` borrow on the
/// owner.
pub struct RequestCorrelator<K, V, E>
where
    K: Eq + Hash + Copy + Send + Sync + 'static,
    V: Send + 'static,
    E: Send + 'static,
{
    pending: Arc<DashMap<K, oneshot::Sender<Result<V, E>>>>,
}

impl<K, V, E> Clone for RequestCorrelator<K, V, E>
where
    K: Eq + Hash + Copy + Send + Sync + 'static,
    V: Send + 'static,
    E: Send + 'static,
{
    fn clone(&self) -> Self {
        Self {
            pending: Arc::clone(&self.pending),
        }
    }
}

impl<K, V, E> Default for RequestCorrelator<K, V, E>
where
    K: Eq + Hash + Copy + Send + Sync + 'static,
    V: Send + 'static,
    E: Send + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V, E> RequestCorrelator<K, V, E>
where
    K: Eq + Hash + Copy + Send + Sync + 'static,
    V: Send + 'static,
    E: Send + 'static,
{
    /// Construct an empty correlator.
    pub fn new() -> Self {
        Self {
            pending: Arc::new(DashMap::new()),
        }
    }

    /// Reserve a pending slot under `key` and return a guard whose [`Drop`]
    /// removes the entry — protecting against panics between `register`
    /// and the eventual [`PendingGuard::wait`] / `wait_cancellable`.
    ///
    /// The caller is responsible for producing unique keys; an overlapping
    /// `register` replaces the previous sender, which drops it and wakes
    /// the previous waiter with [`WaitError::SenderDropped`]. With monotone
    /// counters this is unreachable in practice.
    pub fn register(&self, key: K) -> PendingGuard<K, V, E> {
        let (tx, rx) = oneshot::channel();
        self.pending.insert(key, tx);
        PendingGuard {
            pending: Arc::clone(&self.pending),
            key,
            rx: Some(rx),
        }
    }

    /// Fulfil a pending request. A no-op when `key` has no pending entry
    /// (the caller has already timed out, cancelled, or been shut down).
    pub fn resolve(&self, key: K, value: Result<V, E>) {
        if let Some((_, sender)) = self.pending.remove(&key) {
            // Receiver may have been dropped between the lookup and the
            // send (a race the guard's Drop closes); ignore the error.
            let _ = sender.send(value);
        }
    }

    /// Remove a pending entry without sending a value, dropping the sender
    /// so the waiter wakes with [`WaitError::SenderDropped`].
    ///
    /// Use when the cancellation reason isn't expressible as a `V` or `E`
    /// — e.g. the remote side acknowledged that the request is gone but
    /// won't be sending a structured failure.
    pub fn drop_silently(&self, key: K) {
        self.pending.remove(&key);
    }

    /// Drain every pending entry, waking each waiter with an error produced
    /// by `factory(key)`. Call when the owning context (turn, session) is
    /// tearing down so no waiter is stranded.
    pub fn shutdown_with<F>(&self, factory: F)
    where
        F: Fn(K) -> E,
    {
        // Collect first so we don't mutate the map under iteration.
        let keys: Vec<K> = self.pending.iter().map(|entry| *entry.key()).collect();
        for key in keys {
            if let Some((_, sender)) = self.pending.remove(&key) {
                let _ = sender.send(Err(factory(key)));
            }
        }
    }

    /// Number of in-flight requests. Tests and instrumentation only.
    pub fn len(&self) -> usize {
        self.pending.len()
    }

    /// Returns true if no requests are pending.
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}

/// Reservation handle for one pending request. Drop guarantees removal of
/// the underlying entry, so any panic between [`RequestCorrelator::register`]
/// and the eventual `wait` call still leaves the map clean.
#[must_use = "drop the guard or call wait*; otherwise the slot is freed immediately"]
pub struct PendingGuard<K, V, E>
where
    K: Eq + Hash + Copy + Send + Sync + 'static,
    V: Send + 'static,
    E: Send + 'static,
{
    pending: Arc<DashMap<K, oneshot::Sender<Result<V, E>>>>,
    key: K,
    rx: Option<oneshot::Receiver<Result<V, E>>>,
}

impl<K, V, E> PendingGuard<K, V, E>
where
    K: Eq + Hash + Copy + Send + Sync + 'static,
    V: Send + 'static,
    E: Send + 'static,
{
    /// The correlation key this guard owns.
    pub fn key(&self) -> K {
        self.key
    }

    /// Wait for the request to resolve, bounded by `timeout`.
    ///
    /// On any exit (response received, timeout, sender dropped) the pending
    /// entry is cleaned up.
    pub async fn wait(mut self, timeout: Duration) -> Result<Result<V, E>, WaitError> {
        let rx = self.rx.take().expect("rx is taken exactly once in wait");
        tokio::select! {
            biased;
            () = tokio::time::sleep(timeout) => Err(WaitError::Timeout),
            res = rx => match res {
                Ok(value) => Ok(value),
                Err(_) => Err(WaitError::SenderDropped),
            },
        }
    }

    /// Wait for the request to resolve, bounded by `timeout` *and* a
    /// cancellation token. Cancellation takes priority over timeout if both
    /// fire in the same tick.
    pub async fn wait_cancellable(
        mut self,
        timeout: Duration,
        cancel: &CancellationToken,
    ) -> Result<Result<V, E>, WaitError> {
        let rx = self.rx.take().expect("rx is taken exactly once in wait");
        tokio::select! {
            biased;
            () = cancel.cancelled() => Err(WaitError::Cancelled),
            () = tokio::time::sleep(timeout) => Err(WaitError::Timeout),
            res = rx => match res {
                Ok(value) => Ok(value),
                Err(_) => Err(WaitError::SenderDropped),
            },
        }
    }
}

impl<K, V, E> Drop for PendingGuard<K, V, E>
where
    K: Eq + Hash + Copy + Send + Sync + 'static,
    V: Send + 'static,
    E: Send + 'static,
{
    fn drop(&mut self) {
        self.pending.remove(&self.key);
    }
}

/// Reason a [`PendingGuard::wait`] returned without a remote-side value.
///
/// All variants imply the pending entry has been removed; the caller does
/// not need to clean up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitError {
    /// The configured timeout elapsed before a response arrived.
    Timeout,
    /// The supplied cancellation token fired.
    Cancelled,
    /// The matching sender was dropped without being used — typically a
    /// signal that the owning service is shutting down without invoking
    /// [`RequestCorrelator::shutdown_with`].
    SenderDropped,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    type Correlator = RequestCorrelator<u32, &'static str, &'static str>;

    #[tokio::test]
    async fn happy_path_returns_value() {
        let correlator = Correlator::new();
        let guard = correlator.register(1);

        let handle = tokio::spawn(async move { guard.wait(Duration::from_secs(5)).await });

        // Resolve on the main task.
        correlator.resolve(1, Ok("hello"));

        let outcome = handle.await.expect("task didn't panic");
        assert_eq!(outcome, Ok(Ok("hello")));
        assert!(correlator.is_empty(), "entry must be cleaned up");
    }

    #[tokio::test]
    async fn resolve_with_err_surfaces_inner_error() {
        let correlator = Correlator::new();
        let guard = correlator.register(2);

        let handle = tokio::spawn(async move { guard.wait(Duration::from_secs(5)).await });
        correlator.resolve(2, Err("nope"));

        assert_eq!(handle.await.unwrap(), Ok(Err("nope")));
    }

    #[tokio::test(start_paused = true)]
    async fn timeout_yields_wait_error_and_cleans_up() {
        let correlator = Correlator::new();
        let guard = correlator.register(3);

        let handle = tokio::spawn(async move { guard.wait(Duration::from_millis(50)).await });
        tokio::time::advance(Duration::from_millis(100)).await;

        let outcome = handle.await.unwrap();
        assert_eq!(outcome, Err(WaitError::Timeout));
        assert!(correlator.is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn cancellable_wait_fires_cancel_arm() {
        let correlator = Correlator::new();
        let cancel = CancellationToken::new();
        let guard = correlator.register(4);

        let cancel_clone = cancel.clone();
        let handle = tokio::spawn(async move {
            guard
                .wait_cancellable(Duration::from_secs(60), &cancel_clone)
                .await
        });

        cancel.cancel();
        let outcome = handle.await.unwrap();
        assert_eq!(outcome, Err(WaitError::Cancelled));
        assert!(correlator.is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn late_resolve_after_timeout_is_silent_noop() {
        let correlator = Correlator::new();
        let guard = correlator.register(5);

        let handle = tokio::spawn(async move { guard.wait(Duration::from_millis(10)).await });
        tokio::time::advance(Duration::from_millis(100)).await;
        assert_eq!(handle.await.unwrap(), Err(WaitError::Timeout));

        // No panic and no observable side effect when the slot is gone.
        correlator.resolve(5, Ok("late"));
        assert!(correlator.is_empty());
    }

    #[tokio::test]
    async fn shutdown_wakes_pending_callers_with_factory_error() {
        let correlator = Correlator::new();
        let g1 = correlator.register(10);
        let g2 = correlator.register(11);
        let h1 = tokio::spawn(async move { g1.wait(Duration::from_secs(60)).await });
        let h2 = tokio::spawn(async move { g2.wait(Duration::from_secs(60)).await });

        correlator.shutdown_with(|_| "shutdown");

        assert_eq!(h1.await.unwrap(), Ok(Err("shutdown")));
        assert_eq!(h2.await.unwrap(), Ok(Err("shutdown")));
        assert!(correlator.is_empty());
    }

    #[tokio::test]
    async fn drop_guard_cleans_up_when_caller_panics() {
        let correlator = Correlator::new();
        let id = 99u32;
        let cloned = correlator.clone();

        let handle = tokio::spawn(async move {
            let _guard = cloned.register(id);
            // Panic before awaiting.
            panic!("intentional");
        });

        let _ = handle.await; // joins the panic — explicit ignore
        // The entry was inserted, then the panic unwound the task, and the
        // guard's Drop should have cleaned the slot.
        assert!(correlator.is_empty(), "guard must clean up on panic");
    }

    #[tokio::test]
    async fn drop_guard_cleans_up_when_caller_returns_early() {
        let correlator = Correlator::new();
        {
            let _guard = correlator.register(42);
            assert_eq!(correlator.len(), 1);
        }
        assert!(correlator.is_empty());
    }

    #[tokio::test]
    async fn distinct_ids_coexist_and_resolve_independently() {
        let correlator = Correlator::new();
        let counter = AtomicU32::new(0);
        let id_a = counter.fetch_add(1, Ordering::Relaxed);
        let id_b = counter.fetch_add(1, Ordering::Relaxed);

        let ga = correlator.register(id_a);
        let gb = correlator.register(id_b);
        let ha = tokio::spawn(async move { ga.wait(Duration::from_secs(5)).await });
        let hb = tokio::spawn(async move { gb.wait(Duration::from_secs(5)).await });

        correlator.resolve(id_b, Ok("b"));
        correlator.resolve(id_a, Ok("a"));

        assert_eq!(ha.await.unwrap(), Ok(Ok("a")));
        assert_eq!(hb.await.unwrap(), Ok(Ok("b")));
    }

    #[tokio::test]
    async fn clone_shares_pending_state() {
        let a = Correlator::new();
        let b = a.clone();
        let guard = a.register(7);
        assert_eq!(b.len(), 1);

        let handle = tokio::spawn(async move { guard.wait(Duration::from_secs(5)).await });
        b.resolve(7, Ok("via clone"));
        assert_eq!(handle.await.unwrap(), Ok(Ok("via clone")));
    }

    #[tokio::test]
    async fn drop_silently_wakes_waiter_with_sender_dropped() {
        let correlator = Correlator::new();
        let guard = correlator.register(50);
        let handle = tokio::spawn(async move { guard.wait(Duration::from_secs(5)).await });

        correlator.drop_silently(50);
        assert_eq!(handle.await.unwrap(), Err(WaitError::SenderDropped));
        assert!(correlator.is_empty());
    }

    /// Overlapping `register` calls overwrite the prior sender. With
    /// monotone counters this can't happen; document the behaviour.
    #[tokio::test]
    async fn overlapping_register_wakes_previous_waiter_with_sender_dropped() {
        let correlator = Correlator::new();
        let g1 = correlator.register(123);
        let h1 = tokio::spawn(async move { g1.wait(Duration::from_secs(5)).await });

        // Replace before h1 awaits a value.
        let _g2 = correlator.register(123);

        let outcome = h1.await.unwrap();
        assert_eq!(outcome, Err(WaitError::SenderDropped));
    }
}
