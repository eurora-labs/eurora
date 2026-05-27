//! In-memory types for the activity tracking pipeline.
//!
//! The strategy layer produces [`ActivitySession`]s — one per focus run.
//! Each carries an [`ActivityIdentity`] (the stable parent key the
//! session rolls up to) plus the per-visit details (process info, window
//! title, URL, icon). The backend uses the identity to upsert a parent
//! activity row and inserts the session as its child, so chats linked to
//! that parent naturally aggregate across every visit.
//!
//! Identity computation rules:
//! * **Default strategy**: `key = lowercased process name with `.exe`
//!   stripped`. `display_name = capitalize_first(key)`.
//! * **Browser strategy**: `key = second-level domain label`, computed
//!   from the URL via `psl::domain` and then stripping the public
//!   suffix — so `youtube.com`, `m.youtube.com`, and `youtube.co.uk` all
//!   yield `"youtube"`. URLs without a registrable apex (localhost, IPs,
//!   `chrome-extension://`, `file://`) fall back to the bare host string
//!   or get filtered out at the strategy level.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
pub use thread_core::ContextChip;
use url::Url;
use uuid::Uuid;

/// Stable identity for the parent activity a session rolls up to.
///
/// `key` is the canonical lookup column on the server's `activities`
/// table — comparisons (e.g. the strategy's intra-domain dedupe) must
/// always use `key`, not `display_name`. `display_name` is the value the
/// server stores on first sight and never overwrites — a future user-
/// driven rename endpoint is the only thing that mutates it, so passing
/// `capitalize_first(key)` is always safe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityIdentity {
    pub key: String,
    pub display_name: String,
}

impl ActivityIdentity {
    /// Construct an identity from a raw key, applying the default
    /// `capitalize_first` display name. Callers that already have both
    /// values handy should build the struct literally.
    pub fn from_key(key: impl Into<String>) -> Self {
        let key = key.into();
        let display_name = capitalize_first(&key);
        Self { key, display_name }
    }
}

/// One continuous focus run on a single activity.
///
/// Naming note: the strategy enum variant that carries this is still
/// [`crate::strategies::ActivityReport::NewActivity`] — from the
/// strategy's perspective, "a new activity instance is starting". The
/// backend's parent/child split is downstream of that signal, not
/// reflected in the report's verb.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivitySession {
    pub id: Uuid,
    pub activity: ActivityIdentity,
    pub process_name: String,
    pub process_id: u32,
    pub window_title: Option<String>,
    pub url: Option<Url>,
    #[serde(skip)]
    pub icon: Option<Arc<image::RgbaImage>>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

impl ActivitySession {
    /// Build a process-level session — the default-strategy shape.
    ///
    /// `identity_key` defaults to the lowercased process name (with
    /// `.exe` stripped on Windows). For browsers running without a tab
    /// URL yet, callers can use the same helper so the session lands
    /// against the browser-process parent rather than colliding with a
    /// later YouTube session under a domain identity.
    pub fn new_process(
        process_name: String,
        process_id: u32,
        window_title: Option<String>,
        icon: Option<Arc<image::RgbaImage>>,
    ) -> Self {
        let key = normalize_process_name(&process_name);
        Self {
            id: Uuid::now_v7(),
            activity: ActivityIdentity::from_key(key),
            process_name,
            process_id,
            window_title,
            url: None,
            icon,
            started_at: Utc::now(),
            ended_at: None,
        }
    }

    /// Build a browser session for a focused web page.
    ///
    /// Returns `None` for URLs that have no registrable apex *and* no
    /// host string we can fall back on — e.g. `about:blank`. Callers in
    /// the browser strategy should fall back to [`Self::new_process`]
    /// in that case rather than synthesising an empty identity.
    pub fn new_browser(
        url: Url,
        window_title: Option<String>,
        icon: Option<Arc<image::RgbaImage>>,
        process_name: String,
        process_id: u32,
    ) -> Option<Self> {
        let key = base_domain_label(&url)?;
        Some(Self {
            id: Uuid::now_v7(),
            activity: ActivityIdentity::from_key(key),
            process_name,
            process_id,
            window_title,
            url: Some(url),
            icon,
            started_at: Utc::now(),
            ended_at: None,
        })
    }

    /// Title to render alongside the session.
    ///
    /// `window_title` is the OS-reported title when present; otherwise
    /// the activity's `display_name` so the UI always has *something*.
    pub fn window_title(&self) -> String {
        self.window_title
            .clone()
            .unwrap_or_else(|| self.activity.display_name.clone())
    }

    /// Replace the session's URL.
    ///
    /// Used by the intra-domain SPA-navigation path: the strategy
    /// already verified the base domain didn't change, so the identity
    /// stays put — we only need to update the URL (and the title is
    /// patched separately).
    pub fn set_url(&mut self, url: Url) {
        self.url = Some(url);
    }

    /// Mark the session as ended at "now". Idempotent: a second call
    /// leaves the existing timestamp untouched.
    pub fn end_session(&mut self) {
        if self.ended_at.is_none() {
            self.ended_at = Some(Utc::now());
        }
    }

    pub fn get_context_chip(&self) -> ContextChip {
        ContextChip {
            id: self.id.to_string(),
            name: self.window_title(),
            icon: None,
            domain: self.url.as_ref().and_then(domain_from_url),
        }
    }
}

/// Lowercased process name with the `.exe` suffix stripped.
///
/// Idempotent: passing a name that's already normalized returns the
/// same value. Whitespace inside the name is preserved (macOS reports
/// "Google Chrome" with a space; that's the canonical form).
pub fn normalize_process_name(process_name: &str) -> String {
    let trimmed = process_name
        .strip_suffix(".exe")
        .or_else(|| process_name.strip_suffix(".EXE"))
        .or_else(|| process_name.strip_suffix(".Exe"))
        .unwrap_or(process_name);
    trimmed.to_ascii_lowercase()
}

/// Uppercase the first ASCII letter in `s`, leaving the rest untouched.
///
/// Designed for the default `display_name` derivation, where the input
/// is always a normalized identity key (lowercased ASCII for processes;
/// the second-level domain label for browser activities). Non-ASCII
/// first chars are passed through unchanged.
pub fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => {
            let mut out = String::with_capacity(s.len());
            for c in first.to_uppercase() {
                out.push(c);
            }
            out.push_str(chars.as_str());
            out
        }
    }
}

/// Apex-domain label for a URL, stripped of the public suffix.
///
/// `youtube.com` / `m.youtube.com` / `youtube.co.uk` → `"youtube"`,
/// `docs.google.com` → `"google"` (the registrable apex is `google.com`).
///
/// Returns `None` for URLs with no host (`about:blank`), schemes whose
/// host the desktop never tracks meaningfully (`chrome-extension://`,
/// `file://`), and any case `psl::domain` rejects (IDN edge cases the
/// embedded suffix list doesn't recognise). Falls back to the bare host
/// for IPs and `localhost`, so a local dev server reaches its own
/// bucket without colliding with `www.localhost.com`-style apex.
pub fn base_domain_label(url: &Url) -> Option<String> {
    if matches!(
        url.scheme(),
        "chrome-extension" | "moz-extension" | "file" | "about" | "data" | "blob"
    ) {
        return None;
    }

    // `url::Url::host()` distinguishes IP literals from named hosts — pass
    // those straight through as the identity key rather than letting PSL
    // misclassify them. `psl::domain("127.0.0.1")` returns `Some` with a
    // bogus apex; checking before the PSL call avoids that pitfall.
    match url.host()? {
        url::Host::Ipv4(addr) => return Some(addr.to_string()),
        url::Host::Ipv6(addr) => return Some(addr.to_string()),
        url::Host::Domain(_) => {}
    }

    let host = url.host_str()?.to_ascii_lowercase();

    if let Some(domain) = psl::domain(host.as_bytes()) {
        let domain_str = std::str::from_utf8(domain.as_bytes()).ok()?;
        let suffix_str = std::str::from_utf8(domain.suffix().as_bytes()).ok()?;
        if let Some(label) = domain_str.strip_suffix(&format!(".{suffix_str}"))
            && !label.is_empty()
        {
            return Some(label.to_owned());
        }
        // PSL recognised the host but it's literally a public suffix
        // (e.g. just "com", or a multi-tenant suffix like
        // `s3.amazonaws.com`); treat that the same as an unparseable
        // host and fall through to the bare-host fallback.
    }

    // Fallback for `localhost`, single-label hosts, and IDN cases PSL
    // can't classify. Strip a leading `www.` so the dev variant doesn't
    // fork from the prod one for trivial differences.
    let host = host.strip_prefix("www.").unwrap_or(&host);
    if host.is_empty() {
        None
    } else {
        Some(host.to_owned())
    }
}

/// Lowercased host string, `www.` stripped — used only by
/// [`ActivitySession::get_context_chip`] so the LLM's per-turn context
/// receives the full host (including any subdomain) rather than the
/// apex label that identifies the parent activity. Keeps the
/// previously-shipped chip semantics intact.
fn domain_from_url(url: &Url) -> Option<String> {
    let host = url.host_str()?.to_ascii_lowercase();
    Some(host.strip_prefix("www.").unwrap_or(&host).to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> Url {
        Url::parse(input).expect("valid test URL")
    }

    #[test]
    fn capitalize_first_handles_common_inputs() {
        assert_eq!(capitalize_first("youtube"), "Youtube");
        assert_eq!(capitalize_first("code"), "Code");
        assert_eq!(capitalize_first("x"), "X");
        assert_eq!(capitalize_first(""), "");
        // Idempotent for the first char only; later chars stay as-is.
        assert_eq!(capitalize_first("gitHub"), "GitHub");
    }

    #[test]
    fn normalize_process_name_strips_exe_and_lowercases() {
        assert_eq!(normalize_process_name("Code.exe"), "code");
        assert_eq!(normalize_process_name("notepad.EXE"), "notepad");
        assert_eq!(normalize_process_name("Google Chrome"), "google chrome");
        assert_eq!(normalize_process_name("librewolf"), "librewolf");
    }

    #[test]
    fn base_domain_label_extracts_simple_apex() {
        assert_eq!(
            base_domain_label(&parse("https://youtube.com/watch?v=abc")),
            Some("youtube".into())
        );
    }

    #[test]
    fn base_domain_label_collapses_subdomains() {
        assert_eq!(
            base_domain_label(&parse("https://m.youtube.com/watch?v=1")),
            Some("youtube".into())
        );
        assert_eq!(
            base_domain_label(&parse("https://WWW.YouTube.COM/")),
            Some("youtube".into())
        );
        assert_eq!(
            base_domain_label(&parse("https://docs.google.com/document/d/abc")),
            Some("google".into())
        );
    }

    #[test]
    fn base_domain_label_handles_country_code_tlds() {
        assert_eq!(
            base_domain_label(&parse("https://youtube.co.uk/watch?v=1")),
            Some("youtube".into())
        );
        assert_eq!(
            base_domain_label(&parse("https://www.bbc.co.uk/news")),
            Some("bbc".into())
        );
    }

    #[test]
    fn base_domain_label_falls_back_to_host_for_ips_and_localhost() {
        assert_eq!(
            base_domain_label(&parse("http://127.0.0.1:8080/")),
            Some("127.0.0.1".into())
        );
        assert_eq!(
            base_domain_label(&parse("http://localhost:3000/")),
            Some("localhost".into())
        );
    }

    #[test]
    fn base_domain_label_skips_non_tracked_schemes() {
        assert_eq!(
            base_domain_label(&parse("chrome-extension://abc/popup.html")),
            None
        );
        assert_eq!(base_domain_label(&parse("about:blank")), None);
        // file:// has no host at all — the early scheme check matches first.
        assert_eq!(base_domain_label(&parse("file:///tmp/x.html")), None);
    }

    #[test]
    fn new_browser_returns_none_for_about_blank() {
        let url = parse("about:blank");
        assert!(ActivitySession::new_browser(url, None, None, "chrome".into(), 0).is_none());
    }

    #[test]
    fn new_browser_assigns_identity_from_apex() {
        let session = ActivitySession::new_browser(
            parse("https://m.youtube.com/watch?v=abc"),
            Some("Great Video".into()),
            None,
            "chrome".into(),
            42,
        )
        .expect("session");
        assert_eq!(session.activity.key, "youtube");
        assert_eq!(session.activity.display_name, "Youtube");
        assert_eq!(session.process_id, 42);
        assert_eq!(
            session
                .url
                .as_ref()
                .map(|u| u.host_str().unwrap().to_owned()),
            Some("m.youtube.com".into())
        );
    }

    #[test]
    fn new_process_normalizes_identity_key() {
        let session =
            ActivitySession::new_process("Code.exe".into(), 7, Some("main.rs".into()), None);
        assert_eq!(session.activity.key, "code");
        assert_eq!(session.activity.display_name, "Code");
        assert_eq!(session.window_title(), "main.rs");
    }

    #[test]
    fn window_title_falls_back_to_display_name() {
        let session = ActivitySession::new_process("Code.exe".into(), 0, None, None);
        assert_eq!(session.window_title(), "Code");
    }

    #[test]
    fn set_url_does_not_change_identity_key() {
        let mut session = ActivitySession::new_browser(
            parse("https://youtube.com/a"),
            None,
            None,
            "chrome".into(),
            0,
        )
        .expect("session");
        let new_url = parse("https://youtube.com/b");
        session.set_url(new_url.clone());
        assert_eq!(session.url, Some(new_url));
        assert_eq!(session.activity.key, "youtube");
    }

    #[test]
    fn end_session_is_idempotent() {
        let mut session = ActivitySession::new_process("code".into(), 0, None, None);
        session.end_session();
        let first = session.ended_at;
        session.end_session();
        assert_eq!(session.ended_at, first);
    }

    #[test]
    fn context_chip_uses_full_host() {
        let session = ActivitySession::new_browser(
            parse("https://m.youtube.com/watch?v=1"),
            Some("Video".into()),
            None,
            "chrome".into(),
            0,
        )
        .expect("session");
        let chip = session.get_context_chip();
        // The context chip preserves subdomain so per-site adapters /
        // the LLM see the actual host they're looking at. Only the
        // parent activity_key collapses subdomains.
        assert_eq!(chip.domain.as_deref(), Some("m.youtube.com"));
        assert_eq!(chip.name, "Video");
    }
}
