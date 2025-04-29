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
        todo!()
    }
}
