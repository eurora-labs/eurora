use super::{utils::wayland_detect, wayland_focus_tracker, xorg_focus_tracker};

#[derive(Debug, Clone)]
pub(crate) struct ImplFocusTracker {}

impl ImplFocusTracker {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl ImplFocusTracker {
    pub fn track_focus<F>(&self, on_focus: F) -> anyhow::Result<()>
    where
        F: FnMut(crate::FocusEvent) -> anyhow::Result<()>,
    {
        if wayland_detect() {
            wayland_focus_tracker::track_focus(on_focus)
        } else {
            xorg_focus_tracker::track_focus(on_focus)
        }
    }
}
