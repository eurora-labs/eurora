use crate::{
    FocusTrackerConfig, FocusTrackerResult, FocusedWindow,
    platform::impl_focus_tracker::ImplFocusTracker,
};
use std::future::Future;
use std::sync::atomic::AtomicBool;

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
    /// # Errors
    ///
    /// Returns an error if the platform API fails or the callback returns an error.
    pub async fn track_focus<F, Fut>(&self, on_focus: F) -> FocusTrackerResult<()>
    where
        F: FnMut(FocusedWindow) -> Fut,
        Fut: Future<Output = FocusTrackerResult<()>>,
    {
        self.impl_focus_tracker
            .track_focus(on_focus, &self.config)
            .await
    }

    /// Tracks focus changes with an external stop signal.
    ///
    /// # Errors
    ///
    /// Returns an error if the platform API fails or the callback returns an error.
    pub async fn track_focus_with_stop<F, Fut>(
        &self,
        on_focus: F,
        stop_signal: &AtomicBool,
    ) -> FocusTrackerResult<()>
    where
        F: FnMut(FocusedWindow) -> Fut,
        Fut: Future<Output = FocusTrackerResult<()>>,
    {
        self.impl_focus_tracker
            .track_focus_with_stop(on_focus, stop_signal, &self.config)
            .await
    }
}
