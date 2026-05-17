use auth_core::AuthErrorResponse;
use reqwest::StatusCode;
use thiserror::Error;

use crate::secret_store::SecretStoreError;

/// Errors produced by the authentication layer.
///
/// The key distinction for callers is between [`AuthError::InvalidRefreshToken`]
/// / [`AuthError::MissingRefreshToken`] — which mean "the user is logged out
/// and must sign in again" — and [`AuthError::Transient`] — which means
/// "we couldn't talk to the server, but the locally stored credentials are
/// still intact and a retry may succeed." Conflating the two causes spurious
/// logouts whenever the network hiccups.
#[derive(Debug, Error)]
pub enum AuthError {
    #[error("no refresh token stored")]
    MissingRefreshToken,

    #[error("no access token stored")]
    MissingAccessToken,

    /// The backend rejected our refresh token. The user must log in again.
    #[error("refresh token invalid or expired")]
    InvalidRefreshToken,

    /// The PKCE login challenge is missing — either it expired, was already
    /// consumed, or [`AuthManager::begin_login`] was never called. The
    /// caller should restart the login flow.
    ///
    /// [`AuthManager::begin_login`]: crate::AuthManager::begin_login
    #[error("PKCE login challenge missing or expired")]
    LoginChallengeExpired,

    /// The local secret store failed to read or write session state.
    /// Treated as fatal-to-the-current-operation; the underlying
    /// [`SecretStoreError`] is wrapped behind `anyhow::Error` so the
    /// storage module stays `pub(crate)`.
    #[error("secret store: {0}")]
    Storage(#[source] anyhow::Error),

    /// The refresh attempt failed for a reason unrelated to token validity
    /// (server unreachable, timeout, internal server error, etc.). Stored
    /// credentials are untouched; the operation is safe to retry.
    #[error("transient auth failure: {0}")]
    Transient(#[source] anyhow::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<SecretStoreError> for AuthError {
    fn from(err: SecretStoreError) -> Self {
        AuthError::Storage(anyhow::Error::new(err))
    }
}

pub type AuthResult<T> = Result<T, AuthError>;

impl AuthError {
    /// Classify a non-2xx HTTP response from the auth service.
    ///
    /// `body` is the decoded [`AuthErrorResponse`] envelope, if available.
    /// Falls back to a free-form message when the response wasn't valid
    /// JSON or didn't match the envelope shape.
    pub fn from_http_response(status: StatusCode, body: Option<AuthErrorResponse>) -> Self {
        let detail = body
            .as_ref()
            .map(|b| format!("{} ({})", b.message, b.error))
            .unwrap_or_else(|| status.to_string());

        match status {
            StatusCode::UNAUTHORIZED => AuthError::InvalidRefreshToken,
            _ => AuthError::Transient(anyhow::anyhow!("auth service returned {status}: {detail}")),
        }
    }

    /// Classify a network-level (i.e. transport, DNS, body decode) failure.
    /// Always transient — the server may not have seen the request at all.
    pub fn from_transport(err: reqwest::Error) -> Self {
        AuthError::Transient(anyhow::Error::new(err))
    }

    /// Whether this error means the user's session is definitively gone.
    pub fn is_logged_out(&self) -> bool {
        matches!(
            self,
            AuthError::InvalidRefreshToken | AuthError::MissingRefreshToken
        )
    }

    /// Whether this error is transient (server/network). Local credentials
    /// are still intact and the caller should not force a logout.
    pub fn is_transient(&self) -> bool {
        matches!(self, AuthError::Transient(_))
    }
}
