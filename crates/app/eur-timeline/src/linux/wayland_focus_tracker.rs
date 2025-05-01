use crate::FocusEvent;
use anyhow::Result;

pub fn track_focus<F>(_on_focus: F) -> Result<()>
where
    F: FnMut(FocusEvent) -> anyhow::Result<()>,
{
    todo!()
}
