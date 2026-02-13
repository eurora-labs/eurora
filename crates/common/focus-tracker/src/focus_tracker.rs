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
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(FocusTrackerConfig::default())
    }

    #[must_use]
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
    /// Tracks focus changes, calling `on_focus` each time the focused window changes.
    ///
    /// This method blocks the calling thread indefinitely.
    ///
    /// # Errors
    ///
    /// Returns an error if the platform API fails or the callback returns an error.
    pub fn track_focus<F>(&self, on_focus: F) -> FocusTrackerResult<()>
    where
        F: FnMut(FocusedWindow) -> FocusTrackerResult<()>,
    {
        self.impl_focus_tracker.track_focus(on_focus, &self.config)
    }

    /// Tracks focus changes with an external stop signal.
    ///
    /// # Errors
    ///
    /// Returns an error if the platform API fails or the callback returns an error.
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

    /// Async variant of [`track_focus`](Self::track_focus).
    ///
    /// # Errors
    ///
    /// Returns an error if the platform API fails or the callback returns an error.
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

    /// Async variant of [`track_focus_with_stop`](Self::track_focus_with_stop).
    ///
    /// # Errors
    ///
    /// Returns an error if the platform API fails or the callback returns an error.
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

    /// Spawns a background thread that tracks focus changes and sends them
    /// through a channel.
    ///
    /// # Errors
    ///
    /// Returns an error if the background thread cannot be spawned.
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
    #[must_use]
    pub fn receiver(&self) -> &mpsc::Receiver<FocusedWindow> {
        &self.receiver
    }

    /// Consumes the subscription, returning the receiver.
    ///
    /// The stop signal is set so the background thread will exit after its
    #[must_use]
    pub fn into_receiver(mut self) -> mpsc::Receiver<FocusedWindow> {
        self.stop_signal.store(true, Ordering::Release);
        self.handle.take();
        std::mem::replace(&mut self.receiver, mpsc::channel().1)
    }

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
            .finish_non_exhaustive()
    }
}
