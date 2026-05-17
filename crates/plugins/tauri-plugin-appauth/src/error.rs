use serde::{Serialize, Serializer};

/// `std::result::Result` specialized to this crate's [`Error`] type.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors raised by the plugin. Codes mirror `AppAuth`'s iOS `OIDErrorCode` and
/// Android `AuthorizationException` categories so the JS layer sees the same
/// shape on both platforms.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The user dismissed the browser sheet or hit "Cancel".
    #[error("user canceled the authorization flow")]
    UserCanceled,

    /// The authorization endpoint returned an error response (e.g.
    /// `access_denied`) or the request never reached it.
    #[error("authorization failed: {message}")]
    AuthorizationFailed {
        /// Human-readable description of what went wrong.
        message: String,
        /// `error` token from the OAuth error response, if the server returned one.
        oauth_error: Option<String>,
        /// `error_description` from the OAuth error response, if present.
        oauth_error_description: Option<String>,
    },

    /// The token endpoint refused the code/refresh exchange (e.g.
    /// `invalid_grant`).
    #[error("token exchange failed: {message}")]
    TokenExchangeFailed {
        /// Human-readable description of what went wrong.
        message: String,
        /// `error` token from the OAuth error response, if the server returned one.
        oauth_error: Option<String>,
        /// `error_description` from the OAuth error response, if present.
        oauth_error_description: Option<String>,
    },

    /// Transport-level failure reaching the issuer (DNS, TLS, timeout).
    #[error("network error: {0}")]
    NetworkError(String),

    /// Dynamic Client Registration (RFC 7591) returned an unparseable body.
    #[error("dynamic client registration response was invalid: {0}")]
    InvalidRegistrationResponse(String),

    /// `id_token` failed signature, audience, issuer, or `nonce` validation.
    #[error("ID token validation failed: {0}")]
    IdTokenValidationFailed(String),

    /// No browser capable of completing the flow is installed (Android: no
    /// Custom Tabs-compatible browser).
    #[error("no compatible browser is available on this device")]
    BrowserNotAvailable,

    /// The caller's request was malformed before it reached the network.
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    /// 5xx or otherwise-protocol-violating response from the issuer.
    #[error("authorization server returned an error: {0}")]
    ServerError(String),

    /// Plugin invoked from a non-mobile target.
    #[error("AppAuth flows are only supported on iOS and Android")]
    UnsupportedPlatform,

    /// The mobile plugin bridge itself failed (serde mismatch, native crash,
    /// host-side wiring error). Carries the underlying Tauri error.
    #[cfg(mobile)]
    #[error(transparent)]
    PluginInvoke(#[from] tauri::plugin::mobile::PluginInvokeError),
}

impl Error {
    /// Stable, machine-readable code that the JS layer can switch on without
    /// parsing free-form messages.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Error::UserCanceled => "USER_CANCELED",
            Error::AuthorizationFailed { .. } => "AUTHORIZATION_FAILED",
            Error::TokenExchangeFailed { .. } => "TOKEN_EXCHANGE_FAILED",
            Error::NetworkError(_) => "NETWORK_ERROR",
            Error::InvalidRegistrationResponse(_) => "INVALID_REGISTRATION_RESPONSE",
            Error::IdTokenValidationFailed(_) => "ID_TOKEN_VALIDATION_FAILED",
            Error::BrowserNotAvailable => "BROWSER_NOT_AVAILABLE",
            Error::InvalidRequest(_) => "INVALID_REQUEST",
            Error::ServerError(_) => "SERVER_ERROR",
            Error::UnsupportedPlatform => "UNSUPPORTED_PLATFORM",
            #[cfg(mobile)]
            Error::PluginInvoke(_) => "PLUGIN_INVOKE_FAILED",
        }
    }
}

#[derive(Serialize)]
struct SerializedError<'a> {
    code: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    oauth_error: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    oauth_error_description: Option<&'a str>,
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let (oauth_error, oauth_error_description) = match self {
            Error::AuthorizationFailed {
                oauth_error,
                oauth_error_description,
                ..
            }
            | Error::TokenExchangeFailed {
                oauth_error,
                oauth_error_description,
                ..
            } => (oauth_error.as_deref(), oauth_error_description.as_deref()),
            _ => (None, None),
        };
        SerializedError {
            code: self.code(),
            message: self.to_string(),
            oauth_error,
            oauth_error_description,
        }
        .serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::Error;

    fn expected_code(err: &Error) -> &'static str {
        match err {
            Error::UserCanceled => "USER_CANCELED",
            Error::AuthorizationFailed { .. } => "AUTHORIZATION_FAILED",
            Error::TokenExchangeFailed { .. } => "TOKEN_EXCHANGE_FAILED",
            Error::NetworkError(_) => "NETWORK_ERROR",
            Error::InvalidRegistrationResponse(_) => "INVALID_REGISTRATION_RESPONSE",
            Error::IdTokenValidationFailed(_) => "ID_TOKEN_VALIDATION_FAILED",
            Error::BrowserNotAvailable => "BROWSER_NOT_AVAILABLE",
            Error::InvalidRequest(_) => "INVALID_REQUEST",
            Error::ServerError(_) => "SERVER_ERROR",
            Error::UnsupportedPlatform => "UNSUPPORTED_PLATFORM",
            #[cfg(mobile)]
            Error::PluginInvoke(_) => "PLUGIN_INVOKE_FAILED",
        }
    }

    #[test]
    fn code_covers_every_variant() {
        let cases = [
            Error::UserCanceled,
            Error::AuthorizationFailed {
                message: String::new(),
                oauth_error: None,
                oauth_error_description: None,
            },
            Error::TokenExchangeFailed {
                message: String::new(),
                oauth_error: None,
                oauth_error_description: None,
            },
            Error::NetworkError(String::new()),
            Error::InvalidRegistrationResponse(String::new()),
            Error::IdTokenValidationFailed(String::new()),
            Error::BrowserNotAvailable,
            Error::InvalidRequest(String::new()),
            Error::ServerError(String::new()),
            Error::UnsupportedPlatform,
        ];
        for err in &cases {
            assert_eq!(err.code(), expected_code(err));
        }
    }
}
