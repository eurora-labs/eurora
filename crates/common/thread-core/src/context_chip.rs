//! Per-asset context chip surfaced alongside chat content blocks.

use serde::{Deserialize, Serialize};

#[cfg(feature = "specta")]
use specta::Type;

/// Per-asset context chip surfaced alongside [`ChatContext`] content blocks.
///
/// Lives here (rather than in `euro-activity`) because both desktop and
/// mobile chat IPC layers emit it: desktop populates it from the timeline,
/// mobile populates it from native pickers. Keeping the wire shape in
/// `thread-core` lets the IPC commands stay app-agnostic and avoids
/// dragging the desktop-only `euro-activity` graph into mobile.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct ContextChip {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub domain: Option<String>,
}
