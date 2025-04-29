use super::{utils::wayland_detect, xorg_focus_tracker};

#[derive(Debug, Clone)]
pub(crate) struct ImplFocusTracker {}

impl ImplFocusTracker {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl ImplFocusTracker {
    pub fn track_focus<F>(&self, mut on_focus: F) -> anyhow::Result<()>
    where
        F: FnMut(crate::FocusEvent) -> anyhow::Result<()>,
    {
        if wayland_detect() {
            return Err(anyhow::anyhow!("Wayland is not supported yet"));
        } else {
            xorg_focus_tracker::track_focus(on_focus)
        }
    }
}
