//! Single-slot coalescing queue for outbound `PUT /settings` requests.
//!
//! Many cloud-backed `settings_set_*` IPC calls can land in quick
//! succession (e.g. the user drags the interface-scale slider). Each
//! call invokes [`crate::SyncEngine::request_push`], which feeds into
//! this queue; the queue collapses the burst into at most one push in
//! flight and at most one push behind it. The contract is "≤ 2 PUTs
//! for N rapid requests, regardless of N."
//!
//! Mechanism: a `Notify` paired with an `AtomicU64` counter. Calling
//! [`PushQueue::request`] bumps the counter and signals the worker.
//! The worker calls [`PushQueue::wait`] which:
//!
//! 1. Atomically swaps the counter back to 0 — if any requests landed
//!    since the last drain, return the count immediately.
//! 2. Otherwise `await`s on the `Notify`, waking on the next `request`.
//!
//! The swap-to-zero step is what gives the coalescing property: while
//! the worker is performing a push, an unbounded number of `request`
//! calls all add to the same counter; the next `wait` drains all of
//! them with a single PUT.

use std::sync::atomic::{AtomicU64, Ordering};

use tokio::sync::Notify;

/// Single-slot coalescing queue. Cheap to allocate; no buffering, no
/// per-request state.
#[derive(Debug, Default)]
pub(crate) struct PushQueue {
    pending: AtomicU64,
    notify: Notify,
}

impl PushQueue {
    pub(crate) fn new() -> Self {
        Self {
            pending: AtomicU64::new(0),
            notify: Notify::new(),
        }
    }

    /// Queue a push. Idempotent in the coalescing sense: ten calls in a
    /// row become a single pending flag observable by the worker.
    pub(crate) fn request(&self) {
        // SeqCst with notify_one gives a "happens-before" between the
        // counter bump and the worker waking; Relaxed would let the
        // worker observe zero after being woken on rare CPUs.
        self.pending.fetch_add(1, Ordering::SeqCst);
        self.notify.notify_one();
    }

    /// Wait until at least one request has been queued, then atomically
    /// drain the pending count and return it. The worker performs one
    /// push regardless of the returned value — the count is exposed
    /// for tracing / test assertions, not for fan-out.
    pub(crate) async fn wait(&self) -> u64 {
        loop {
            let drained = self.pending.swap(0, Ordering::SeqCst);
            if drained > 0 {
                return drained;
            }
            // `notified()` registers the waker *before* awaiting, so a
            // concurrent `notify_one` racing with the swap-to-zero
            // above is delivered to this future rather than dropped.
            self.notify.notified().await;
        }
    }

    /// Snapshot of pending requests. Test-only.
    #[cfg(test)]
    pub(crate) fn pending(&self) -> u64 {
        self.pending.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use super::*;

    #[tokio::test]
    async fn ten_rapid_requests_drain_into_one_wait() {
        let q = PushQueue::new();
        for _ in 0..10 {
            q.request();
        }
        assert_eq!(q.pending(), 10);
        let drained = q.wait().await;
        assert_eq!(drained, 10);
        assert_eq!(q.pending(), 0);
    }

    #[tokio::test]
    async fn wait_parks_until_request() {
        let q = Arc::new(PushQueue::new());
        let q2 = q.clone();
        let handle = tokio::spawn(async move { q2.wait().await });
        // Give the worker a chance to actually park on `notified()`.
        tokio::time::sleep(Duration::from_millis(20)).await;
        q.request();
        let drained = handle.await.unwrap();
        assert_eq!(drained, 1);
    }

    #[tokio::test]
    async fn requests_during_drain_coalesce_into_next_wait() {
        // Simulates the production pattern: worker drains, performs a
        // push (modeled by a yield here), then waits again. Any
        // requests that arrived during the push must surface in the
        // second wait — never get lost.
        let q = Arc::new(PushQueue::new());
        q.request();
        let first = q.wait().await;
        assert_eq!(first, 1);

        // "During the push": three more requests land.
        q.request();
        q.request();
        q.request();

        let second = q.wait().await;
        assert_eq!(second, 3);
    }
}
