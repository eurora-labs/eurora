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
//! Keeping the env var name and parse logic in one place ensures the
//! three layers can never disagree about which origins count as "the
//! SPA". This module lives in `be-auth-core` because both `be-authz`
//! and `be-auth-service` already depend on it, and it's pure config
//! plumbing — no axum, no async.
//!
//! There is no in-source fallback: dev origins live in `.env.example`,
//! production origins must be set explicitly. Missing the variable is
//! a startup error, not a "defaulted to localhost" silent success.

use std::collections::HashSet;

/// Name of the environment variable. Comma-separated list of
/// `scheme://host[:port]` entries — exactly what a browser sends in
/// the `Origin` header.
pub const WEB_ALLOWED_ORIGINS_ENV: &str = "WEB_ALLOWED_ORIGINS";

/// Returned by [`web_origins_from_env`] when the variable is unset,
/// blank, or parses to an empty set.
#[derive(Debug, thiserror::Error)]
#[error("`{}` is unset or empty", WEB_ALLOWED_ORIGINS_ENV)]
pub struct MissingWebOrigins;

/// Parse a comma-separated origin list into a set, trimming whitespace
/// and dropping empty entries.
pub fn parse_web_origins(raw: &str) -> HashSet<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Read and parse `WEB_ALLOWED_ORIGINS`. Returns
/// [`MissingWebOrigins`] if the variable is unset, blank after
/// trimming, or contains nothing but separators.
pub fn web_origins_from_env() -> Result<HashSet<String>, MissingWebOrigins> {
    let raw = std::env::var(WEB_ALLOWED_ORIGINS_ENV)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .ok_or(MissingWebOrigins)?;
    let parsed = parse_web_origins(&raw);
    if parsed.is_empty() {
        return Err(MissingWebOrigins);
    }
    Ok(parsed)
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
}
