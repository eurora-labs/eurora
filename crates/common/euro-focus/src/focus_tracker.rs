use crate::{
    FerrousFocusResult, FocusTrackerConfig, FocusedWindow,
    platform::impl_focus_tracker::ImplFocusTracker,
};
use std::sync::{atomic::AtomicBool, mpsc};

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
    pub fn track_focus<F>(&self, on_focus: F) -> FerrousFocusResult<()>
    where
        F: FnMut(FocusedWindow) -> FerrousFocusResult<()>,
    {
        self.impl_focus_tracker.track_focus(on_focus, &self.config)
    }

    pub fn track_focus_with_stop<F>(
        &self,
        on_focus: F,
        stop_signal: &AtomicBool,
    ) -> FerrousFocusResult<()>
    where
        F: FnMut(FocusedWindow) -> FerrousFocusResult<()>,
    {
        self.impl_focus_tracker
            .track_focus_with_stop(on_focus, stop_signal, &self.config)
    }

    /// Async version of track_focus - requires the "async" feature
    #[cfg(feature = "async")]
    pub async fn track_focus_async<F, Fut>(&self, on_focus: F) -> FerrousFocusResult<()>
    where
        F: FnMut(FocusedWindow) -> Fut,
        Fut: Future<Output = FerrousFocusResult<()>>,
    {
        self.impl_focus_tracker
            .track_focus_async(on_focus, &self.config)
            .await
    }

    /// Async version of track_focus_with_stop - requires the "async" feature
    #[cfg(feature = "async")]
    pub async fn track_focus_async_with_stop<F, Fut>(
        &self,
        on_focus: F,
        stop_signal: &AtomicBool,
    ) -> FerrousFocusResult<()>
    where
        F: FnMut(FocusedWindow) -> Fut,
        Fut: Future<Output = FerrousFocusResult<()>>,
    {
        self.impl_focus_tracker
            .track_focus_async_with_stop(on_focus, stop_signal, &self.config)
            .await
    }

    /// Subscribe to focus changes and receive them via a channel
    pub fn subscribe_focus_changes(&self) -> FerrousFocusResult<mpsc::Receiver<FocusedWindow>> {
        let (sender, receiver) = mpsc::channel();
        let stop_signal = AtomicBool::new(false);

        // Clone the tracker for the background thread
        let tracker = self.clone();

        // Spawn a background thread to track focus changes
        std::thread::spawn(move || {
            let _ = tracker.track_focus_with_stop(
                move |window: FocusedWindow| -> FerrousFocusResult<()> {
                    if sender.send(window).is_err() {
                        // Receiver has been dropped, stop tracking
                        return Err(crate::FerrousFocusError::Error(
                            "Receiver dropped".to_string(),
                        ));
                    }
                    Ok(())
                },
                &stop_signal,
            );
        });

        Ok(receiver)
    }
}
