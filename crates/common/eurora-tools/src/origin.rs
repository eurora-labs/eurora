//! Typed routing information for tool dispatch.
//!
//! Every active context on the client carries an [`Origin`] that names a
//! specific destination — a browser tab, an OS window, an ACP session. The
//! per-turn snapshot freezes one origin per context key; the dispatcher
//! later receives that frozen origin inside an `IncomingCall` and uses it
//! to construct the actual transport-level request (a bridge frame, a
//! native command, an ACP session message).
//!
//! `Origin` is intentionally **never** serialized over the chat WebSocket
//! — the server only sees [`thread_core::WireActiveContext`], which omits
//! the routing fields. Origins exist purely client-side.
//!
//! v1 wires `Origin::Browser` end-to-end. The other variants are declared
//! so the enum is stable and adapter methods can target them at compile
//! time; their plumbing arrives in later phases.

/// Routing target for a tool call.
///
/// The variant is determined by the context key — `youtube::watch_page`
/// always produces `Origin::Browser`, `focus::app::<name>` always produces
/// `Origin::Focused`, an ACP session always produces `Origin::Acp`. The
/// macro-generated dispatcher matches on the variant; a mismatch between
/// the adapter method's declared target type and the runtime variant
/// yields `ToolError::OriginMismatch`.
///
/// `#[non_exhaustive]` so new origin kinds can be added without breaking
/// downstream `match` expressions inside this crate's consumers.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Origin {
    /// A specific tab inside a bridge-registered browser process.
    Browser(BrowserOrigin),
    /// A specific OS-level window of a focused application.
    Focused(FocusedOrigin),
    /// A specific ACP session connected to the desktop app.
    Acp(AcpOrigin),
}

impl Origin {
    /// Stable, human-readable name of the active variant.
    ///
    /// Used by `ToolError::OriginMismatch` so the error message can name
    /// what the dispatcher actually received without exposing the inner
    /// data. The strings are part of the public contract — don't rename
    /// them lightly.
    pub fn variant_name(&self) -> &'static str {
        match self {
            Origin::Browser(_) => "Browser",
            Origin::Focused(_) => "Focused",
            Origin::Acp(_) => "Acp",
        }
    }
}

/// Routing target for tools backed by the browser bridge.
///
/// `process_id` is the bridge-registered app PID (the OS process id for
/// real browsers, a stable assigned id for sandboxed clients). `tab_id`
/// is the browser's per-process tab identifier (passed verbatim to
/// `chrome.tabs.sendMessage`). The window id and page URL are carried
/// for diagnostics and for the LLM-facing context payload.
#[derive(Debug, Clone)]
pub struct BrowserOrigin {
    pub process_id: u32,
    pub tab_id: i64,
    pub window_id: Option<String>,
    pub page_url: String,
}

/// Routing target for tools backed by the OS focus tracker.
#[derive(Debug, Clone)]
pub struct FocusedOrigin {
    pub process_id: u32,
    pub window_id: Option<u64>,
    pub app_name: String,
}

/// Routing target for tools piped through an ACP session.
#[derive(Debug, Clone)]
pub struct AcpOrigin {
    pub process_id: u32,
    pub session_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_browser() -> Origin {
        Origin::Browser(BrowserOrigin {
            process_id: 4242,
            tab_id: 19,
            window_id: Some("win-0".into()),
            page_url: "https://www.youtube.com/watch?v=abc123".into(),
        })
    }

    fn sample_focused() -> Origin {
        Origin::Focused(FocusedOrigin {
            process_id: 7777,
            window_id: Some(101),
            app_name: "Visual Studio Code".into(),
        })
    }

    fn sample_acp() -> Origin {
        Origin::Acp(AcpOrigin {
            process_id: 1234,
            session_id: "session-1".into(),
        })
    }

    #[test]
    fn variant_name_matches_variant() {
        assert_eq!(sample_browser().variant_name(), "Browser");
        assert_eq!(sample_focused().variant_name(), "Focused");
        assert_eq!(sample_acp().variant_name(), "Acp");
    }

    #[test]
    fn origin_is_clone() {
        let origin = sample_browser();
        let cloned = origin.clone();
        // Inspect the cloned variant to confirm clone is deep.
        match cloned {
            Origin::Browser(b) => {
                assert_eq!(b.process_id, 4242);
                assert_eq!(b.tab_id, 19);
                assert_eq!(b.window_id.as_deref(), Some("win-0"));
                assert_eq!(b.page_url, "https://www.youtube.com/watch?v=abc123");
            }
            other => panic!("expected Browser variant, got {other:?}"),
        }
    }
}
