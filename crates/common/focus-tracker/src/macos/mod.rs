pub(crate) mod utils;

mod impl_focus_tracker;

pub(crate) use impl_focus_tracker::track_focus;
pub(crate) use utils::focused_document_url;
