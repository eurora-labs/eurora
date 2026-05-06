//! Canonical handling of the `WEB_ALLOWED_ORIGINS` environment variable.
//!
//! Three call sites need the same list of SPA origins:
//!
//! * `be_authz::OriginGuardConfig` — the cross-origin allowlist for
//!   cookie-mode mutating requests.
//! * `be_auth_service::CookieConfig` — used to detect whether an
//!   inbound request is cookie-mode (browser) vs. bearer-mode
//!   (desktop / mobile).
//! * The monolith's `CorsLayer` — which origins may make credentialed
//!   `fetch` calls.
//!
//! Keeping the env var name, default, and parse logic in one place
//! ensures the three layers can never disagree about which origins
//! count as "the SPA". This module lives in `be-auth-core` because
//! both `be-authz` and `be-auth-service` already depend on it, and
//! it's pure config plumbing — no axum, no async.

use std::collections::HashSet;

/// Name of the environment variable. Comma-separated list of
/// `scheme://host[:port]` entries — exactly what a browser sends in
/// the `Origin` header.
pub const WEB_ALLOWED_ORIGINS_ENV: &str = "WEB_ALLOWED_ORIGINS";

const PROD_DEFAULT: &str = "https://www.eurora-labs.com";
const DEV_DEFAULT: &str = "http://localhost:5173,http://localhost:3000";

/// Default origin list when `WEB_ALLOWED_ORIGINS` is unset.
///
/// Debug builds get the local Vite/Next dev hosts so a fresh checkout
/// works without a `.env`; release builds get only the canonical
/// production origin so a forgotten env var fails closed.
pub fn default_web_origins() -> &'static str {
    if cfg!(debug_assertions) {
        DEV_DEFAULT
    } else {
        PROD_DEFAULT
    }
}

/// Parse a comma-separated origin list into a set, trimming whitespace
/// and dropping empty entries.
pub fn parse_web_origins(raw: &str) -> HashSet<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Read and parse `WEB_ALLOWED_ORIGINS`, falling back to
/// [`default_web_origins`].
pub fn web_origins_from_env() -> HashSet<String> {
    let raw = std::env::var(WEB_ALLOWED_ORIGINS_ENV)
        .unwrap_or_else(|_| default_web_origins().to_owned());
    parse_web_origins(&raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_trims_and_drops_empty() {
        let set = parse_web_origins(" https://a.example , ,https://b.example ");
        assert!(set.contains("https://a.example"));
        assert!(set.contains("https://b.example"));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn default_is_non_empty() {
        assert!(!default_web_origins().is_empty());
    }
}
