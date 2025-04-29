use crate::FocusEvent;
use anyhow::{Context, Result};

pub fn track_focus<F>(mut on_focus: F) -> Result<()>
where
    F: FnMut(FocusEvent) -> anyhow::Result<()>,
{
    todo!()
}
