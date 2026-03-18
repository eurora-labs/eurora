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
    /// A focus event is only emitted when the `(process_id, window_title)` pair
    /// differs from the previously reported one.
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
        platform::track_focus(on_focus, stop_signal, &self.config).await
    }
}
