use crate::{
    FocusTrackerConfig, FocusTrackerResult, FocusedWindow,
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

    pub fn subscribe_focus_changes(&self) -> FocusTrackerResult<mpsc::Receiver<FocusedWindow>> {
        let (sender, receiver) = mpsc::channel();
        let stop_signal = AtomicBool::new(false);

        let tracker = self.clone();

        std::thread::spawn(move || {
            let _ = tracker.track_focus_with_stop(
                move |window: FocusedWindow| -> FocusTrackerResult<()> {
                    if sender.send(window).is_err() {
                        return Err(crate::FocusTrackerError::Error(
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
