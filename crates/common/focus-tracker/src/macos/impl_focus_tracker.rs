use crate::{FocusTrackerConfig, FocusTrackerResult, FocusedWindow};
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

#[derive(Default)]
struct FocusState {
    process_name: String,
    window_title: Option<String>,
}

impl FocusState {
    fn has_changed(&self, window: &FocusedWindow) -> bool {
        self.process_name != window.process_name
            || self.window_title.as_deref() != window.window_title.as_deref()
    }

    fn update_from(&mut self, window: &FocusedWindow) {
        self.process_name = window.process_name.clone();
        self.window_title = window.window_title.clone();
    }
}

#[inline]
fn should_stop(stop_signal: Option<&AtomicBool>) -> bool {
    stop_signal.is_some_and(|stop| stop.load(Ordering::Relaxed))
}

impl ImplFocusTracker {
    pub fn track_focus<F>(&self, on_focus: F, config: &FocusTrackerConfig) -> FocusTrackerResult<()>
    where
        F: FnMut(FocusedWindow) -> FocusTrackerResult<()>,
    {
        self.run(on_focus, None, config)
    }

    pub fn track_focus_with_stop<F>(
        &self,
        on_focus: F,
        stop_signal: &AtomicBool,
        config: &FocusTrackerConfig,
    ) -> FocusTrackerResult<()>
    where
        F: FnMut(FocusedWindow) -> FocusTrackerResult<()>,
    {
        self.run(on_focus, Some(stop_signal), config)
    }

    #[cfg(feature = "async")]
    pub async fn track_focus_async<F, Fut>(
        &self,
        on_focus: F,
        config: &FocusTrackerConfig,
    ) -> FocusTrackerResult<()>
    where
        F: FnMut(FocusedWindow) -> Fut,
        Fut: Future<Output = FocusTrackerResult<()>>,
    {
        self.run_async(on_focus, None, config).await
    }

    #[cfg(feature = "async")]
    pub async fn track_focus_async_with_stop<F, Fut>(
        &self,
        on_focus: F,
        stop_signal: &AtomicBool,
        config: &FocusTrackerConfig,
    ) -> FocusTrackerResult<()>
    where
        F: FnMut(FocusedWindow) -> Fut,
        Fut: Future<Output = FocusTrackerResult<()>>,
    {
        self.run_async(on_focus, Some(stop_signal), config).await
    }

    #[cfg(feature = "async")]
    async fn run_async<F, Fut>(
        &self,
        mut on_focus: F,
        stop_signal: Option<&AtomicBool>,
        config: &FocusTrackerConfig,
    ) -> FocusTrackerResult<()>
    where
        F: FnMut(FocusedWindow) -> Fut,
        Fut: Future<Output = FocusTrackerResult<()>>,
    {
        let mut prev_state = FocusState::default();

        loop {
            if should_stop(stop_signal) {
                debug!("Stop signal received, exiting focus tracking loop");
                break;
            }

            match utils::get_frontmost_window_basic_info() {
                Ok(mut window) => {
                    if prev_state.has_changed(&window) {
                        match utils::fetch_icon_for_pid(window.process_id as i32, &config.icon) {
                            Ok(icon) => window.icon = icon,
                            Err(e) => debug!("Error fetching icon: {}", e),
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
    ) -> FocusTrackerResult<()>
    where
        F: FnMut(FocusedWindow) -> FocusTrackerResult<()>,
    {
        let mut prev_state = FocusState::default();

        loop {
            if should_stop(stop_signal) {
                debug!("Stop signal received, exiting focus tracking loop");
                break;
            }

            match utils::get_frontmost_window_basic_info() {
                Ok(mut window) => {
                    if prev_state.has_changed(&window) {
                        match utils::fetch_icon_for_pid(window.process_id as i32, &config.icon) {
                            Ok(icon) => window.icon = icon,
                            Err(e) => debug!("Error fetching icon: {}", e),
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
