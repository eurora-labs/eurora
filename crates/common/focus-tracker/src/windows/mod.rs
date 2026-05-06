pub(crate) mod utils;

mod impl_focus_tracker;

pub(crate) use impl_focus_tracker::track_focus;

use crate::FocusTrackerResult;

/// Windows placeholder for [`crate::focused_document_url`].
///
/// UI Automation exposes `ValuePattern` and `LegacyIAccessible::Value`, but
/// PDF viewers on Windows do not consistently surface the document URL
/// through them. Until we have a working Windows mapping we always report
/// "no document URL"; callers must treat `Ok(None)` as a soft signal.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn focused_document_url(_pid: u32) -> FocusTrackerResult<Option<String>> {
    Ok(None)
}
