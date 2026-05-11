//! Apple server-to-server notifications handler.
//!
//! Apple delivers four event types via signed JWTs to a registered
//! endpoint: `consent-revoked`, `account-delete`, `email-disabled`,
//! `email-enabled`. The first two collapse to the same in-system
//! action — the user can no longer sign in via Apple — and the
//! remaining two are logged-only today (we don't route transactional
//! mail through the Hide-My-Email relay).
//!
//! The status-code policy distinguishes three failure shapes:
//!
//! - **Forged or tampered payload** (signature / iss / aud / freshness
//!   fail) → **401**. Silently dropping a forgery with a 200 would
//!   mute alerting on a real attack.
//! - **Forwards-compat unknowns** (malformed JSON we can decode but
//!   don't recognise, unknown event type, unknown `sub`) → **200**
//!   with a `warn` log emitted by the HTTP handler. Apple retries
//!   non-2xx, so an unknown future event type must not loop indefinitely.
//! - **Genuine internal failures** (DB down, etc.) → **5xx**. Apple
//!   will retry; idempotent side-effects make that safe.
//!
//! Side-effects on termination are idempotent by construction:
//! revoking an already-revoked refresh token is a no-op; deleting an
//! already-deleted `oauth_credentials` row is a no-op. So we don't
//! need to track `jti` for dedup.
//!
//! Telemetry policy: this module is a *pure* dispatcher. Every log
//! line that an operator might read about a notification is emitted
//! by the HTTP handler in [`crate::handlers::apple_notifications`],
//! based on the [`AppleNotificationOutcome`] returned here. Putting
//! the policy in one place keeps the field set consistent and makes
//! the service method straightforward to test in isolation.

use be_remote_db::OAuthProvider;
use uuid::Uuid;

use crate::error::AuthError;
use crate::oauth::OAuthError;
use crate::oauth::apple::AppleEventKind;
use crate::service::AuthService;

/// Reason an Apple termination event was emitted. Mirrors the two
/// terminal-event variants of [`AppleEventKind`] but lifted to a flat
/// enum so the outcome doesn't have to re-wrap the wire-format kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminationCause {
    /// User revoked this app's Sign-in-with-Apple consent.
    ConsentRevoked,
    /// User deleted their entire Apple ID.
    AccountDelete,
}

impl TerminationCause {
    /// Lift the two terminal-event kinds to a [`TerminationCause`];
    /// returns `None` for non-terminal kinds (the email-flag toggles
    /// and the forwards-compat `Unknown` arm).
    fn from_kind(kind: &AppleEventKind) -> Option<Self> {
        match kind {
            AppleEventKind::ConsentRevoked => Some(Self::ConsentRevoked),
            AppleEventKind::AccountDelete => Some(Self::AccountDelete),
            _ => None,
        }
    }

    /// Stable string label suitable for structured log fields.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ConsentRevoked => "consent-revoked",
            Self::AccountDelete => "account-delete",
        }
    }
}

/// Outcome of processing a single Apple notification.
///
/// Every variant maps to a 200 at the HTTP boundary — including the
/// "we don't know this user / event" variants, because Apple retries
/// non-2xx and we don't want a tombstoned user to loop forever.
/// Differentiation is for structured logs at the handler, not status
/// codes.
///
/// The flat field set (rather than re-wrapping [`AppleEventKind`])
/// keeps each variant carrying only what telemetry consumes: the
/// `email` / `is_private_email` payload that `EmailDisabled` /
/// `EmailEnabled` events carry on the wire never leaves the verifier,
/// because nothing downstream branches on it today.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppleNotificationOutcome {
    /// `consent-revoked` or `account-delete` against a known user.
    /// Sessions were torn down and the Apple credential link removed.
    /// `residual_credentials_missing` is `true` when the user has no
    /// other sign-in method left after termination — the handler
    /// turns this into a loud `warn!` because it's both a UX edge
    /// case and a forensic anchor for App-Store-deletion audits.
    AccountTerminated {
        user_id: Uuid,
        sessions_revoked: u64,
        cause: TerminationCause,
        residual_credentials_missing: bool,
    },
    /// `email-enabled` / `email-disabled`. We don't act on these
    /// today; preserved as a distinct outcome so a future change
    /// (e.g. switching transactional mail to the relay) can wire in
    /// behaviour without reshaping the public surface.
    EmailFlagToggled { sub: String, enabled: bool },
    /// Apple referenced a user we don't have. Possible causes:
    /// pre-launch test event, a user we deleted server-side, replay
    /// of a stale event. Idempotent — Apple will retry these on its
    /// own cadence and we want them ack'd.
    UnknownUser {
        sub: String,
        cause: TerminationCause,
    },
    /// Apple introduced an event type we haven't taught the parser.
    /// Ack and log so a deploy of the next handler version can pick
    /// up the backlog (Apple resends on request, or future events
    /// share the new type).
    UnknownEvent { raw_type: String, sub: String },
}

/// Failure modes returned by [`AuthService::handle_apple_notification`].
///
/// Kept separate from [`AuthError`] because the HTTP mapping is
/// notification-specific: a verification failure here must produce a
/// 401 with empty body, not the JSON error envelope every other auth
/// handler uses. Apple isn't a UX surface — it doesn't read messages,
/// only status codes.
#[derive(Debug, thiserror::Error)]
pub enum AppleNotificationError {
    /// Payload failed verification (signature, audience, issuer,
    /// freshness, structural). Always maps to **401**.
    #[error("notification verification failed: {0}")]
    Verification(#[source] OAuthError),

    /// Genuine internal failure (DB unreachable, etc.). Maps to
    /// **503** so Apple retries. The inner `AuthError` carries the
    /// structured cause for logging at the handler.
    #[error("notification processing internal error: {0}")]
    Internal(#[from] AuthError),
}

// `thiserror`'s `#[from]` only generates the *direct* conversion, not
// transitive ones. We want `?` through DB calls in
// `terminate_apple_user` to compose: `DbError` -> `AuthError::Database`
// -> `AppleNotificationError::Internal`. Without this manual bridge
// each call site would need an explicit `.map_err`.
impl From<be_remote_db::DbError> for AppleNotificationError {
    fn from(e: be_remote_db::DbError) -> Self {
        Self::Internal(AuthError::Database(e))
    }
}

impl AuthService {
    /// Top-level entry point for `POST /auth/oauth/apple/notifications`.
    ///
    /// Verifies the inbound JWT against Apple's JWKS, dispatches on
    /// the event type, and returns a structured outcome. The HTTP
    /// handler is a thin wrapper that maps the result to a status
    /// code (200 / 401 / 503) and emits the audit log line.
    pub async fn handle_apple_notification(
        &self,
        payload_jwt: &str,
    ) -> Result<AppleNotificationOutcome, AppleNotificationError> {
        let client = self.apple_oauth()?;
        let event = client
            .verify_notification(payload_jwt)
            .await
            .map_err(AppleNotificationError::Verification)?;

        // Terminal events (`consent-revoked` / `account-delete`)
        // dispatch through `terminate_apple_user`. Done as an early
        // return so the remaining `match` can move `event` by value
        // without cloning the kind.
        if let Some(cause) = TerminationCause::from_kind(&event.kind) {
            return self.terminate_apple_user(event.sub, cause).await;
        }

        Ok(match event.kind {
            AppleEventKind::EmailEnabled { .. } => AppleNotificationOutcome::EmailFlagToggled {
                sub: event.sub,
                enabled: true,
            },
            AppleEventKind::EmailDisabled { .. } => AppleNotificationOutcome::EmailFlagToggled {
                sub: event.sub,
                enabled: false,
            },
            AppleEventKind::Unknown(raw_type) => AppleNotificationOutcome::UnknownEvent {
                raw_type,
                sub: event.sub,
            },
            // Unreachable: terminal cases are handled by the
            // early-return above. Spelt out explicitly so adding a
            // new terminal variant to `AppleEventKind` produces a
            // compile-time miss-match in `TerminationCause::from_kind`
            // before silently falling through here.
            AppleEventKind::ConsentRevoked | AppleEventKind::AccountDelete => unreachable!(),
        })
    }

    /// Tear down a user's Apple sign-in: revoke refresh tokens, drop
    /// the `oauth_credentials` row, and report the "now orphaned"
    /// case so the handler can warn for ops without auto-deleting
    /// the user.
    ///
    /// Auto-deletion is intentionally deferred to a product flow:
    /// silently removing the user row from inside a webhook hides a
    /// destructive action from any audit trail that anchors on
    /// "intentional user-deletion request".
    async fn terminate_apple_user(
        &self,
        sub: String,
        cause: TerminationCause,
    ) -> Result<AppleNotificationOutcome, AppleNotificationError> {
        let user = match self
            .db()
            .get_user_by_oauth_provider()
            .provider(OAuthProvider::Apple)
            .provider_user_id(&sub)
            .call()
            .await
        {
            Ok(user) => user,
            Err(e) if e.is_not_found() => {
                return Ok(AppleNotificationOutcome::UnknownUser { sub, cause });
            }
            Err(e) => return Err(e.into()),
        };

        let sessions_revoked = self
            .db()
            .revoke_all_refresh_tokens_for_user()
            .user_id(user.id)
            .call()
            .await?;

        self.db()
            .delete_oauth_credentials()
            .provider(OAuthProvider::Apple)
            .user_id(user.id)
            .call()
            .await?;

        let has_password = self
            .db()
            .user_has_password_credentials()
            .user_id(user.id)
            .call()
            .await?;
        let remaining_oauth = self
            .db()
            .count_oauth_credentials_for_user()
            .user_id(user.id)
            .call()
            .await?;
        let residual_credentials_missing = !has_password && remaining_oauth == 0;

        Ok(AppleNotificationOutcome::AccountTerminated {
            user_id: user.id,
            sessions_revoked,
            cause,
            residual_credentials_missing,
        })
    }
}

#[cfg(test)]
mod tests {
    //! Type-shape smoke tests for the notification surface.
    //!
    //! The orchestrator (`handle_apple_notification`) needs a working
    //! `AuthService` (DB + Apple client), which is out of scope for
    //! a pure unit test — that surface is covered by the verifier
    //! tests in `oauth::apple` plus future DB-integration tests
    //! once the harness lands. Here we pin the
    //! `AppleNotificationError` taxonomy and the
    //! `TerminationCause::from_kind` projection: the handler must
    //! keep `Verification` and `Internal` discriminable (their HTTP
    //! status codes diverge) and `from_kind` must stay aligned with
    //! [`AppleEventKind`]'s terminal variants.
    use super::*;
    use crate::oauth::OAuthError;

    #[test]
    fn verification_variant_carries_oauth_error_source() {
        let err =
            AppleNotificationError::Verification(OAuthError::NotificationOutsideFreshnessWindow {
                iat: 100,
                now: 1_000,
            });
        assert!(format!("{err}").contains("verification failed"));
        let source = std::error::Error::source(&err).expect("source chain present");
        assert!(source.to_string().contains("outside freshness window"));
    }

    #[test]
    fn internal_variant_wraps_auth_error() {
        let inner = AuthError::Internal("db down".into());
        let err: AppleNotificationError = inner.into();
        assert!(matches!(err, AppleNotificationError::Internal(_)));
    }

    #[test]
    fn db_error_lifts_to_internal() {
        // The blanket `From<DbError>` impl is what makes `?`
        // through DB calls in `terminate_apple_user` work; pin its
        // landing variant explicitly so a refactor doesn't lose
        // the lift.
        let db_err = be_remote_db::DbError::not_found("user");
        let err: AppleNotificationError = db_err.into();
        assert!(matches!(err, AppleNotificationError::Internal(_)));
    }

    #[test]
    fn termination_cause_projects_terminal_kinds() {
        assert_eq!(
            TerminationCause::from_kind(&AppleEventKind::ConsentRevoked),
            Some(TerminationCause::ConsentRevoked),
        );
        assert_eq!(
            TerminationCause::from_kind(&AppleEventKind::AccountDelete),
            Some(TerminationCause::AccountDelete),
        );
        assert_eq!(
            TerminationCause::from_kind(&AppleEventKind::Unknown("future".into())),
            None,
        );
        assert_eq!(
            TerminationCause::from_kind(&AppleEventKind::EmailEnabled {
                email: "x@y".into(),
                is_private_email: false,
            }),
            None,
        );
    }

    #[test]
    fn termination_cause_labels_are_stable() {
        // Log-field stability: dashboards / alerts grep on these
        // strings. Pinning them prevents an accidental rename.
        assert_eq!(TerminationCause::ConsentRevoked.as_str(), "consent-revoked");
        assert_eq!(TerminationCause::AccountDelete.as_str(), "account-delete");
    }
}
