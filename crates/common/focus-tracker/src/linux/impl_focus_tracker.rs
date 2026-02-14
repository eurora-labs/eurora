use super::{utils::wayland_detect, xorg_focus_tracker};
use crate::{FocusTrackerConfig, FocusTrackerError, FocusTrackerResult, FocusedWindow};
use std::sync::atomic::AtomicBool;

#[cfg(feature = "async")]
use std::future::Future;

#[derive(Debug, Clone)]
pub struct ImplFocusTracker {}

impl ImplFocusTracker {
    pub fn new() -> Self {
        Self {}
    }
}

fn qualify_x11_error(err: FocusTrackerError) -> FocusTrackerError {
    if matches!(err, FocusTrackerError::NoDisplay) && wayland_detect() {
        FocusTrackerError::Unsupported
    } else {
        err
    }
}

impl ImplFocusTracker {
    pub fn track_focus<F>(&self, on_focus: F, config: &FocusTrackerConfig) -> FocusTrackerResult<()>
    where
        F: FnMut(FocusedWindow) -> FocusTrackerResult<()>,
    {
        xorg_focus_tracker::track_focus(on_focus, config).map_err(qualify_x11_error)
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
        xorg_focus_tracker::track_focus_with_stop(on_focus, stop_signal, config)
            .map_err(qualify_x11_error)
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
        xorg_focus_tracker::track_focus_async(on_focus, config)
            .await
            .map_err(qualify_x11_error)
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
        xorg_focus_tracker::track_focus_async_with_stop(on_focus, stop_signal, config)
            .await
            .map_err(qualify_x11_error)
    }
}
