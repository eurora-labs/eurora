use crate::{
    FocusTrackerConfig, FocusTrackerResult, FocusedWindow,
    platform::impl_focus_tracker::ImplFocusTracker,
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::thread::JoinHandle;

#[cfg(feature = "async")]
use std::future::Future;

#[derive(Debug, Clone)]
pub struct FocusTracker {
    impl_focus_tracker: ImplFocusTracker,
    config: FocusTrackerConfig,
}

impl FocusTracker {
    pub fn new() -> Self {
        Self::with_config(FocusTrackerConfig::default())
    }

    pub fn with_config(config: FocusTrackerConfig) -> Self {
        Self {
            impl_focus_tracker: ImplFocusTracker::new(),
            config,
        }
    }
}

impl Default for FocusTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl FocusTracker {
    pub fn track_focus<F>(&self, on_focus: F) -> FocusTrackerResult<()>
    where
        F: FnMut(FocusedWindow) -> FocusTrackerResult<()>,
    {
        self.impl_focus_tracker.track_focus(on_focus, &self.config)
    }

    pub fn track_focus_with_stop<F>(
        &self,
        on_focus: F,
        stop_signal: &AtomicBool,
    ) -> FocusTrackerResult<()>
    where
        F: FnMut(FocusedWindow) -> FocusTrackerResult<()>,
    {
        self.impl_focus_tracker
            .track_focus_with_stop(on_focus, stop_signal, &self.config)
    }

    #[cfg(feature = "async")]
    pub async fn track_focus_async<F, Fut>(&self, on_focus: F) -> FocusTrackerResult<()>
    where
        F: FnMut(FocusedWindow) -> Fut,
        Fut: Future<Output = FocusTrackerResult<()>>,
    {
        self.impl_focus_tracker
            .track_focus_async(on_focus, &self.config)
            .await
    }

    #[cfg(feature = "async")]
    pub async fn track_focus_async_with_stop<F, Fut>(
        &self,
        on_focus: F,
        stop_signal: &AtomicBool,
    ) -> FocusTrackerResult<()>
    where
        F: FnMut(FocusedWindow) -> Fut,
        Fut: Future<Output = FocusTrackerResult<()>>,
    {
        self.impl_focus_tracker
            .track_focus_async_with_stop(on_focus, stop_signal, &self.config)
            .await
    }

    pub fn subscribe_focus_changes(&self) -> FocusTrackerResult<FocusSubscription> {
        let (sender, receiver) = mpsc::channel();
        let stop_signal = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop_signal);

        let tracker = self.clone();

        let handle = std::thread::Builder::new()
            .name("focus-tracker".into())
            .spawn(move || {
                let _ = tracker.track_focus_with_stop(
                    move |window: FocusedWindow| -> FocusTrackerResult<()> {
                        if sender.send(window).is_err() {
                            return Err(crate::FocusTrackerError::ChannelClosed);
                        }
                        Ok(())
                    },
                    &thread_stop,
                );
            })
            .map_err(|e| {
                crate::FocusTrackerError::platform_with_source(
                    "failed to spawn focus tracking thread",
                    e,
                )
            })?;

        Ok(FocusSubscription {
            receiver,
            stop_signal,
            handle: Some(handle),
        })
    }
}

/// A handle to an active focus-change subscription.
///
/// Provides a [`mpsc::Receiver`] for consuming focus events and manages the
/// lifecycle of the background tracking thread. The thread is signaled to stop
/// and joined when the subscription is dropped.
pub struct FocusSubscription {
    receiver: mpsc::Receiver<FocusedWindow>,
    stop_signal: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl FocusSubscription {
    /// Returns a reference to the receiver for consuming focus events.
    pub fn receiver(&self) -> &mpsc::Receiver<FocusedWindow> {
        &self.receiver
    }

    /// Consumes the subscription, returning the receiver.
    ///
    /// The background thread will continue running until the receiver is dropped
    /// and the channel closes, at which point it exits on its own.
    /// To stop the thread promptly, prefer dropping the entire `FocusSubscription`.
    pub fn into_receiver(mut self) -> mpsc::Receiver<FocusedWindow> {
        // Detach: don't signal stop or join on drop.
        self.handle.take();
        std::mem::replace(&mut self.receiver, mpsc::channel().1)
    }

    /// Signals the background thread to stop and waits for it to exit.
    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        self.stop_signal.store(true, Ordering::Release);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for FocusSubscription {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl std::fmt::Debug for FocusSubscription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FocusSubscription")
            .field("stopped", &self.stop_signal.load(Ordering::Relaxed))
            .finish()
    }
}
