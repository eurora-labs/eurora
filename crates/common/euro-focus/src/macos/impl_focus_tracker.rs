use crate::{FerrousFocusResult, FocusTrackerConfig, FocusedWindow};
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::debug;

#[cfg(feature = "async")]
use std::future::Future;

use super::utils;

#[derive(Debug, Clone)]
pub(crate) struct ImplFocusTracker {}

impl ImplFocusTracker {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

/// Tracks the previous focus state for change detection.
/// Uses references to avoid cloning strings on every poll.
#[derive(Default)]
struct FocusState {
    process_name: Option<String>,
    window_title: Option<String>,
}

impl FocusState {
    /// Check if the window has changed compared to the current state.
    /// Returns true if process_name or window_title differs.
    fn has_changed(&self, window: &FocusedWindow) -> bool {
        self.process_name.as_deref() != window.process_name.as_deref()
            || self.window_title.as_deref() != window.window_title.as_deref()
    }

    /// Update the state from the given window.
    /// Only clones when necessary (when focus actually changed).
    fn update_from(&mut self, window: &FocusedWindow) {
        self.process_name = window.process_name.clone();
        self.window_title = window.window_title.clone();
    }
}

/// Check if the stop signal is set.
#[inline]
fn should_stop(stop_signal: Option<&AtomicBool>) -> bool {
    stop_signal.is_some_and(|stop| stop.load(Ordering::Relaxed))
}

impl ImplFocusTracker {
    pub fn track_focus<F>(&self, on_focus: F, config: &FocusTrackerConfig) -> FerrousFocusResult<()>
    where
        F: FnMut(FocusedWindow) -> FerrousFocusResult<()>,
    {
        self.run(on_focus, None, config)
    }

    pub fn track_focus_with_stop<F>(
        &self,
        on_focus: F,
        stop_signal: &AtomicBool,
        config: &FocusTrackerConfig,
    ) -> FerrousFocusResult<()>
    where
        F: FnMut(FocusedWindow) -> FerrousFocusResult<()>,
    {
        self.run(on_focus, Some(stop_signal), config)
    }

    #[cfg(feature = "async")]
    pub async fn track_focus_async<F, Fut>(
        &self,
        on_focus: F,
        config: &FocusTrackerConfig,
    ) -> FerrousFocusResult<()>
    where
        F: FnMut(FocusedWindow) -> Fut,
        Fut: Future<Output = FerrousFocusResult<()>>,
    {
        self.run_async(on_focus, None, config).await
    }

    #[cfg(feature = "async")]
    pub async fn track_focus_async_with_stop<F, Fut>(
        &self,
        on_focus: F,
        stop_signal: &AtomicBool,
        config: &FocusTrackerConfig,
    ) -> FerrousFocusResult<()>
    where
        F: FnMut(FocusedWindow) -> Fut,
        Fut: Future<Output = FerrousFocusResult<()>>,
    {
        self.run_async(on_focus, Some(stop_signal), config).await
    }

    #[cfg(feature = "async")]
    async fn run_async<F, Fut>(
        &self,
        mut on_focus: F,
        stop_signal: Option<&AtomicBool>,
        config: &FocusTrackerConfig,
    ) -> FerrousFocusResult<()>
    where
        F: FnMut(FocusedWindow) -> Fut,
        Fut: Future<Output = FerrousFocusResult<()>>,
    {
        let mut prev_state = FocusState::default();

        loop {
            // Check stop signal before processing
            if should_stop(stop_signal) {
                debug!("Stop signal received, exiting focus tracking loop");
                break;
            }

            // Get basic window info first (without icon - fast)
            match utils::get_frontmost_window_basic_info() {
                Ok(mut window) => {
                    // Only fetch icon and report when focus actually changed
                    if prev_state.has_changed(&window) {
                        // Fetch icon only when focus changed (expensive operation)
                        if let Some(pid) = window.process_id {
                            match utils::fetch_icon_for_pid(pid as i32, &config.icon) {
                                Ok(icon) => window.icon = icon,
                                Err(e) => debug!("Error fetching icon: {}", e),
                            }
                        }
                        prev_state.update_from(&window);
                        on_focus(window).await?;
                    }
                }
                Err(e) => {
                    debug!("Error getting window info: {}", e);
                }
            }

            tokio::time::sleep(config.poll_interval).await;
        }

        Ok(())
    }

    fn run<F>(
        &self,
        mut on_focus: F,
        stop_signal: Option<&AtomicBool>,
        config: &FocusTrackerConfig,
    ) -> FerrousFocusResult<()>
    where
        F: FnMut(FocusedWindow) -> FerrousFocusResult<()>,
    {
        let mut prev_state = FocusState::default();

        loop {
            // Check stop signal before processing
            if should_stop(stop_signal) {
                debug!("Stop signal received, exiting focus tracking loop");
                break;
            }

            // Get basic window info first (without icon - fast)
            match utils::get_frontmost_window_basic_info() {
                Ok(mut window) => {
                    // Only fetch icon and report when focus actually changed
                    if prev_state.has_changed(&window) {
                        // Fetch icon only when focus changed (expensive operation)
                        if let Some(pid) = window.process_id {
                            match utils::fetch_icon_for_pid(pid as i32, &config.icon) {
                                Ok(icon) => window.icon = icon,
                                Err(e) => debug!("Error fetching icon: {}", e),
                            }
                        }
                        prev_state.update_from(&window);
                        on_focus(window)?;
                    }
                }
                Err(e) => {
                    debug!("Error getting window info: {}", e);
                }
            }

            std::thread::sleep(config.poll_interval);
        }

        Ok(())
    }
}
