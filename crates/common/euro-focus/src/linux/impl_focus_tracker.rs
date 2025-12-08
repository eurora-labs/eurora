use super::{utils::wayland_detect, xorg_focus_tracker};
use crate::{FerrousFocusError, FerrousFocusResult, FocusTrackerConfig, FocusedWindow};
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

impl ImplFocusTracker {
    pub fn track_focus<F>(&self, on_focus: F, config: &FocusTrackerConfig) -> FerrousFocusResult<()>
    where
        F: FnMut(FocusedWindow) -> FerrousFocusResult<()>,
    {
        if wayland_detect() {
            // Wayland is not supported for the time being
            Err(FerrousFocusError::Unsupported)
        } else {
            xorg_focus_tracker::track_focus(on_focus, config)
        }
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
        if wayland_detect() {
            // Wayland is not supported for the time being
            Err(FerrousFocusError::Unsupported)
        } else {
            xorg_focus_tracker::track_focus_with_stop(on_focus, stop_signal, config)
        }
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
        if wayland_detect() {
            // Wayland is not supported for the time being
            Err(FerrousFocusError::Unsupported)
        } else {
            xorg_focus_tracker::track_focus_async(on_focus, config).await
        }
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
        if wayland_detect() {
            // Wayland is not supported for the time being
            Err(FerrousFocusError::Unsupported)
        } else {
            xorg_focus_tracker::track_focus_async_with_stop(on_focus, stop_signal, config).await
        }
    }
}
