use thiserror::Error;

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

    /// The refresh attempt failed for a reason unrelated to token validity
    /// (server unreachable, timeout, internal server error, etc.). Stored
    /// credentials are untouched; the operation is safe to retry.
    #[error("transient auth failure: {0}")]
    Transient(#[source] anyhow::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type AuthResult<T> = Result<T, AuthError>;

impl AuthError {
    /// Classify a `tonic::Status` returned by the auth service.
    pub fn from_refresh_status(status: tonic::Status) -> Self {
        match status.code() {
            tonic::Code::Unauthenticated => AuthError::InvalidRefreshToken,
            _ => AuthError::Transient(anyhow::Error::new(status)),
        }
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
