use crate::platform::impl_focus_tracker::ImplFocusTracker;

pub struct FocusEvent {
    pub process: String,
    pub title: String,
    pub icon_base64: String,
}

pub struct FocusTracker {
    pub(crate) impl_focus_tracker: ImplFocusTracker,
}

impl FocusTracker {
    pub(crate) fn new(impl_focus_tracker: ImplFocusTracker) -> Self {
        Self { impl_focus_tracker }
    }
}

impl FocusTracker {
    pub fn track_focus<F>(&self, mut on_focus: F) -> anyhow::Result<()>
    where
        F: FnMut(crate::FocusEvent) -> anyhow::Result<()>,
    {
        self.impl_focus_tracker.track_focus(on_focus)
    }
}
