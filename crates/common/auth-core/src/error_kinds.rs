//! Stable machine-readable identifiers for entries in
//! [`crate::AuthErrorResponse::error`]. Used by clients to dispatch on
//! specific failure modes without parsing free-form messages.

/// Generic 400-class request validation failure.
pub const INVALID_ARGUMENT: &str = "invalid_argument";

/// Authentication failure (missing / malformed / expired credentials).
pub const UNAUTHENTICATED: &str = "unauthenticated";

/// Caller is authenticated but the email is not yet verified, and the
/// requested route requires a verified email.
pub const EMAIL_NOT_VERIFIED: &str = "email_not_verified";

/// The OAuth provider returned an email that is already registered to a
/// different identity (password credentials or another OAuth provider).
///
/// Auto-linking is intentionally rejected — see
/// `be_auth_service::AuthError::OAuthEmailConflict` for the security
/// rationale. Stable across releases; the desktop client dispatches on
/// this value to surface the correct UX.
pub const OAUTH_EMAIL_CONFLICT: &str = "oauth_email_conflict";

/// Caller exceeded a rate limit (failed-auth limiter, resend-cooldown,
/// etc.). Includes a `Retry-After`-style hint in the response message
/// when applicable.
pub const RATE_LIMITED: &str = "rate_limited";

/// Catch-all for unexpected server-side failures. Detail is logged but
/// never echoed to the client.
pub const INTERNAL_ERROR: &str = "internal_error";
