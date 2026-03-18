use bon::bon;

use crate::{FocusTrackerConfig, FocusTrackerResult, FocusedWindow, platform};
use std::future::Future;
use std::sync::atomic::AtomicBool;

#[derive(Debug, Clone)]
pub struct FocusTracker {
    config: FocusTrackerConfig,
}

impl Default for FocusTracker {
    fn default() -> Self {
        Self::builder().build()
    }
}

#[bon]
impl FocusTracker {
    #[builder]
    #[must_use]
    pub fn new(#[builder(default)] config: FocusTrackerConfig) -> Self {
        Self { config }
    }

    /// Tracks focus changes, calling `on_focus` each time the focused window changes.
    ///
    /// # Errors
    ///
    /// Returns an error if the platform API fails or the callback returns an error.
    #[builder]
    pub async fn track_focus<F, Fut>(
        &self,
        on_focus: F,
        stop_signal: Option<&AtomicBool>,
    ) -> FocusTrackerResult<()>
    where
        F: FnMut(FocusedWindow) -> Fut,
        Fut: Future<Output = FocusTrackerResult<()>>,
    {
        match stop_signal {
            Some(signal) => platform::track_focus_with_stop(on_focus, signal, &self.config).await,
            None => platform::track_focus(on_focus, &self.config).await,
        }
    }
}
