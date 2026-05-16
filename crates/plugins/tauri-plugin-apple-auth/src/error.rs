use serde::{Serialize, Serializer};

/// `std::result::Result` specialized to this crate's [`Error`] type.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors raised by the plugin.
///
/// Note: user-driven outcomes (cancellation, authorization rejection)
/// are **not** errors. They are values of the [`crate::AppleSignInOutcome`]
/// returned on the `Ok` path. This enum covers genuine bridge failures
/// — invalid request payloads, the platform not supporting the flow,
/// or the mobile plugin bridge itself crashing.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The caller's request was malformed before it reached the
    /// native side (e.g. empty raw nonce).
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    /// Plugin invoked from a target that does not ship Sign in with
    /// Apple. Desktop and Android always return this.
    #[error("Sign in with Apple is only supported on iOS")]
    UnsupportedPlatform,

    /// The mobile plugin bridge itself failed (serde mismatch, native
    /// crash, host-side wiring error). Carries the underlying Tauri
    /// error so the caller can decide how to recover.
    #[cfg(mobile)]
    #[error(transparent)]
    PluginInvoke(#[from] tauri::plugin::mobile::PluginInvokeError),
}

impl Error {
    /// Stable, machine-readable code that the JS layer (or the
    /// `euro-mobile` procedure) can switch on without parsing
    /// free-form messages.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Error::InvalidRequest(_) => "INVALID_REQUEST",
            Error::UnsupportedPlatform => "UNSUPPORTED_PLATFORM",
            #[cfg(mobile)]
            Error::PluginInvoke(_) => "PLUGIN_INVOKE_FAILED",
        }
    }
}

#[derive(Serialize)]
struct SerializedError {
    code: &'static str,
    message: String,
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        SerializedError {
            code: self.code(),
            message: self.to_string(),
        }
        .serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::Error;

    #[test]
    fn codes_are_stable_strings() {
        // These literals are part of the host ↔ plugin contract.
        // Renaming them would silently break callers that switch on
        // `err.code()`.
        assert_eq!(
            Error::InvalidRequest(String::new()).code(),
            "INVALID_REQUEST"
        );
        assert_eq!(Error::UnsupportedPlatform.code(), "UNSUPPORTED_PLATFORM");
    }
}
