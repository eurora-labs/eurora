//! Authenticated-identity surface for the cloud-settings sync engine.
//!
//! The engine needs the current user's `sub` claim for two reasons:
//!
//! - To gate network I/O: an unauthenticated boot sits in
//!   [`super::SyncStatus::LocalOnly`] without contacting the server.
//! - To enforce account-isolation: a cache stamped with user A's id
//!   must not be PUT under user B's credentials (and a 200 GET under
//!   B's credentials must not overlay A's local edits). The engine
//!   compares [`crate::CloudSettingsCache::last_user_id`] against the
//!   value returned here on every pull.
//!
//! Splitting identity off [`super::SettingsTransport`] keeps the test
//! transport — which speaks plain HTTP against `wiremock` and has no
//! keyring — free of auth state. The production implementation
//! ([`AuthManagerIdentity`]) wraps the shared `euro_auth::AuthManager`
//! so the engine refreshes through the same coalescing lock as every
//! other authenticated caller in the app.
//!
//! The trait is async because the production path goes through
//! `AuthManager::get_or_refresh_access_token`, which may hit the
//! network. It returns `Option<Uuid>` rather than a bare `Uuid` so a
//! definitively logged-out state is a value (handled by flipping to
//! `LocalOnly`) rather than an error (which would push the engine
//! through the retry loop).

use async_trait::async_trait;
use euro_auth::AuthManager;
use uuid::Uuid;

use super::error::{SyncError, SyncResult};

/// Resolve the currently authenticated user's id.
///
/// Three terminal states map onto the engine's [`super::SyncStatus`]:
///
/// - `Ok(Some(uid))` — the engine proceeds with a network round-trip
///   under `uid`'s credentials.
/// - `Ok(None)` — no authenticated user. The engine publishes
///   `LocalOnly` and performs no I/O.
/// - `Err(_)` — transient failure resolving the identity (auth
///   service unreachable, keyring read failed). The engine publishes
///   `Offline` and retries on the next trigger.
#[async_trait]
pub trait AuthIdentity: Send + Sync + 'static {
    async fn current_user_id(&self) -> SyncResult<Option<Uuid>>;
}

/// Production identity resolver: refreshes the access token through
/// the shared [`AuthManager`] (so concurrent settings + thread + user
/// callers coalesce through one lock), reads the `sub` claim, and
/// parses it as a [`Uuid`].
///
/// A non-parseable `sub` is treated as a hard internal error — the
/// server-side invariant is that every JWT carries a UUID, and a
/// regression there should surface loudly rather than silently
/// degrade to `LocalOnly`.
#[derive(Clone)]
pub struct AuthManagerIdentity {
    auth: AuthManager,
}

impl AuthManagerIdentity {
    #[must_use]
    pub fn new(auth: AuthManager) -> Self {
        Self { auth }
    }
}

impl std::fmt::Debug for AuthManagerIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthManagerIdentity")
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl AuthIdentity for AuthManagerIdentity {
    async fn current_user_id(&self) -> SyncResult<Option<Uuid>> {
        // Refresh-if-needed so a near-expiry token isn't mistaken for
        // a logout. `is_logged_out` distinguishes refresh-token reject
        // (definitive) from transport / transient errors (retry).
        if let Err(err) = self.auth.get_or_refresh_access_token().await {
            if err.is_logged_out() {
                return Ok(None);
            }
            return Err(SyncError::Auth(err));
        }

        let claims = match self.auth.get_access_token_payload() {
            Ok(claims) => claims,
            Err(_) => return Ok(None),
        };

        let uid = Uuid::parse_str(&claims.sub).map_err(|err| {
            SyncError::Internal(anyhow::anyhow!(
                "JWT subject is not a UUID ({err}); refusing to sync"
            ))
        })?;
        Ok(Some(uid))
    }
}
