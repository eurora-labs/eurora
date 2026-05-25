use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct NativeMetadata {
    /// Browser-side tab id of the focused tab. `i32` rather than `i64`
    /// because specta's TypeScript binding emits `number` (an `f64`) for
    /// any integer and the safe-integer range is `±2^53 - 1` — `i64`
    /// values can silently lose precision when round-tripping. Chrome
    /// tab ids are well within `i32` range in practice.
    ///
    /// Required: the extension only sends metadata when it can identify
    /// a concrete tab, so the desktop can rely on this for `LIST_TOOLS`
    /// / `INVOKE_TOOL` / `CANCEL_TOOL` routing without a fallback path.
    /// URL, title, and icon all come from the same tab record, so there
    /// is no scenario where the desktop receives metadata without a tab
    /// id.
    pub tab_id: i32,
    pub url: Option<String>,
    pub icon_base64: Option<String>,
    pub title: Option<String>,
}
