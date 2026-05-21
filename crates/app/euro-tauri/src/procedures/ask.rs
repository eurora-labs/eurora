//! Procedures backing the Spotlight-style "ask" overlay.
//!
//! The overlay has two windows:
//!
//! - The compact ask bar ([`crate::window::ASK_WINDOW_LABEL`]). Input
//!   only; the user types a question and presses Return.
//! - The taller answer pane ([`crate::window::ANSWER_WINDOW_LABEL`]).
//!   Streams the response in. Reusable across invocations — a second
//!   submission while the pane is open reuses the same window rather
//!   than stacking up overlays.
//!
//! All invocation paths funnel through [`ask_open_answer_window`]:
//!
//! - The compact bar's submit handler (`prompt: Some(text)`).
//! - The global hotkey when [`AskBarSettings::enabled`] is `false`
//!   (`prompt: None` — the user types directly into the answer pane).
//! - The `eurora://ask?q=…` deep link from a Shortcut, Raycast macro,
//!   or App Intent (`prompt: Some(query)`).
//!
//! Keeping all three on a single command means the App Intent path
//! (v2) plugs into the same seam without re-touching the overlay
//! plumbing.

use serde::Serialize;
use specta::Type;
use tauri::{AppHandle, Manager};
use thiserror::Error;
use url::form_urlencoded;

use crate::window::{
    ANSWER_WINDOW_LABEL, ASK_WINDOW_LABEL, anchor_answer_window, anchor_ask_window,
    create_answer_window, create_ask_window,
};

/// Typed error surface for the `ask_*` IPC commands. Externally tagged
/// so the frontend can branch on `type` without parsing strings.
#[derive(Debug, Error, Serialize, Type)]
#[serde(tag = "type", content = "data")]
pub enum AskError {
    #[error("window: {0}")]
    Window(String),
}

impl From<tauri::Error> for AskError {
    fn from(value: tauri::Error) -> Self {
        AskError::Window(value.to_string())
    }
}

/// Summon the compact ask bar. Creates the window if it does not exist
/// yet, and (re)anchors it to the top-center of the active monitor on
/// every call so the bar always lands in its canonical position when
/// summoned via the hotkey or tray entry.
///
/// Implemented as a free function so the hotkey handler and the tray
/// menu callback can summon the bar without round-tripping through
/// the IPC layer. The matching `#[tauri::command]` ([`ask_open_window`])
/// is a thin wrapper over this helper.
pub fn open_ask_bar(app_handle: &AppHandle) -> Result<(), AskError> {
    let window = match app_handle.get_webview_window(ASK_WINDOW_LABEL) {
        Some(existing) => existing,
        None => create_ask_window(app_handle)?,
    };
    anchor_ask_window(&window)?;
    Ok(())
}

/// Open the answer window, optionally pre-filled with a prompt the
/// chat service should send immediately on mount.
///
/// `prompt` is encoded into the answer window's URL as `?q=<encoded>`
/// so the frontend can read it synchronously from
/// `window.location.search` during `onMount` — there is no IPC
/// round-trip between window creation and the first user-visible
/// frame.
///
/// Placement: when the ask bar is currently visible the answer window
/// is stacked directly above or below it (whichever side has more
/// space); otherwise the answer window lands at the canonical
/// top-center anchor, matching the position the ask bar would have
/// occupied. This makes deep-link / App Intent invocation feel
/// indistinguishable from the bar-driven flow.
pub fn open_answer_pane(app_handle: &AppHandle, prompt: Option<&str>) -> Result<(), AskError> {
    let url = answer_window_url(prompt);

    // Destroy any prior answer window. A previous invocation may have
    // mounted with a different `?q=` payload that the SvelteKit
    // router would not re-trigger on a simple `set_url` — closing and
    // recreating guarantees the frontend observes the new prompt
    // through its `onMount`. The cost (a fresh webview boot, a few
    // hundred ms) is acceptable for an interactive Spotlight-style
    // surface; we pay it at most once per ask submission.
    if let Some(existing) = app_handle.get_webview_window(ANSWER_WINDOW_LABEL) {
        existing.close()?;
    }

    let answer = create_answer_window(app_handle, &url)?;
    let ask = app_handle.get_webview_window(ASK_WINDOW_LABEL);
    anchor_answer_window(&answer, ask.as_ref())?;
    Ok(())
}

/// Hide the ask bar without destroying it. The webview is preserved so
/// the next summon avoids a fresh boot.
pub fn hide_ask_bar(app_handle: &AppHandle) -> Result<(), AskError> {
    if let Some(window) = app_handle.get_webview_window(ASK_WINDOW_LABEL) {
        window.hide()?;
    }
    Ok(())
}

/// Read the `askBar.enabled` setting. Returns `true` (the fresh-install
/// default) when the settings state is unavailable — better to show
/// the bar than to silently swallow the hotkey because some initialization
/// race left the state empty.
pub async fn ask_bar_enabled(app_handle: &AppHandle) -> bool {
    let Some(state) = app_handle.try_state::<crate::shared_types::SharedSettingsState>() else {
        return true;
    };
    state.lock().await.cache.settings.desktop.ask_bar.enabled
}

/// Summon the overlay invoked by the hotkey or tray. Reads the
/// `askBar.enabled` setting and either opens the compact bar (default)
/// or the answer pane directly (when the user opted out of the bar).
/// All errors are logged in place — the caller is a fire-and-forget
/// callback with no way to surface failures to the user.
pub async fn summon_overlay(app_handle: AppHandle) {
    let want_bar = ask_bar_enabled(&app_handle).await;
    let result = if want_bar {
        open_ask_bar(&app_handle)
    } else {
        open_answer_pane(&app_handle, None)
    };
    if let Err(err) = result {
        tracing::error!("Failed to summon ask overlay: {err}");
    }
}

#[tauri::command]
#[specta::specta]
pub async fn ask_open_window(app_handle: AppHandle) -> Result<(), AskError> {
    open_ask_bar(&app_handle)
}

#[tauri::command]
#[specta::specta]
pub async fn ask_close_window(app_handle: AppHandle) -> Result<(), AskError> {
    hide_ask_bar(&app_handle)
}

#[tauri::command]
#[specta::specta]
pub async fn ask_open_answer_window(
    app_handle: AppHandle,
    prompt: Option<String>,
) -> Result<(), AskError> {
    open_answer_pane(&app_handle, prompt.as_deref())
}

/// Build the relative URL the answer webview loads. `?q=` is omitted
/// entirely when no prompt is supplied so the frontend can branch on
/// `URLSearchParams.has('q')` rather than treating an empty string
/// the same as "no prompt".
fn answer_window_url(prompt: Option<&str>) -> String {
    match prompt {
        Some(text) if !text.is_empty() => {
            let encoded: String = form_urlencoded::byte_serialize(text.as_bytes()).collect();
            format!("answer?q={encoded}")
        }
        _ => "answer".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::answer_window_url;

    #[test]
    fn url_without_prompt_omits_query_string() {
        assert_eq!(answer_window_url(None), "answer");
        assert_eq!(answer_window_url(Some("")), "answer");
    }

    #[test]
    fn url_with_prompt_percent_encodes_payload() {
        let url = answer_window_url(Some("hello world & friends"));
        assert!(url.starts_with("answer?q="));
        assert!(url.contains("hello+world"));
        assert!(url.contains("%26"));
        assert!(url.contains("friends"));
    }

    #[test]
    fn url_with_prompt_handles_unicode() {
        let url = answer_window_url(Some("café — résumé"));
        assert!(url.starts_with("answer?q="));
        // Multibyte UTF-8 bytes must be percent-encoded (not raw)
        assert!(!url.contains("café"));
    }
}
