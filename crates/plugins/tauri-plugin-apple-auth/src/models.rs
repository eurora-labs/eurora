use serde::{Deserialize, Serialize};

/// Input to [`crate::AppleAuth::sign_in_with_apple`].
///
/// `raw_nonce` is opaque to the plugin. The iOS bridge SHA-256-hashes
/// it (and base64url-encodes the digest) before assigning to
/// `ASAuthorizationAppleIDRequest.nonce` — Apple echoes whatever the
/// client puts there into the ID token's `nonce` claim, so the
/// backend must apply the same hash to verify. Keeping the raw value
/// on the caller's side means the caller is the only entity that ever
/// knows the unhashed nonce, which is the property the replay defence
/// relies on.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignInRequest {
    /// Opaque caller-generated nonce. Recommended 32 bytes of OS
    /// randomness, base64url-encoded. Must not be empty — the bridge
    /// rejects empty strings with [`crate::Error::InvalidRequest`].
    pub raw_nonce: String,
}

/// Payload of [`AppleSignInOutcome::Success`]: the identity material
/// `ASAuthorizationController` hands back on a successful sheet
/// dismissal.
///
/// `id_token` is the Apple-signed JWT bound to the iOS Bundle ID; the
/// backend verifies signature + nonce against Apple's JWKS.
/// `authorization_code` is preserved for callers that want to perform
/// the server-to-server code-exchange flow alongside; the
/// Eurora backend trusts the ID token directly and ignores the code.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignInResponse {
    /// Apple-issued OIDC ID token (JWT). Always present on success.
    pub id_token: String,
    /// Single-use authorization code Apple ships alongside the ID
    /// token. Present on success; `None` on the rare path where
    /// `ASAuthorizationAppleIDCredential.authorizationCode` decodes
    /// to an empty value (we surface `None` rather than `""` so the
    /// distinction stays explicit on the wire).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorization_code: Option<String>,
    /// First / last name from
    /// `ASAuthorizationAppleIDCredential.fullName`. Apple only ships
    /// this on the **very first** sign-in for a given user; absent on
    /// every subsequent one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<AppleNativeUser>,
}

/// First / last name from
/// `ASAuthorizationAppleIDCredential.fullName.{givenName, familyName}`.
///
/// Both halves are optional because Apple's `PersonNameComponents`
/// permits either side to be nil, and the user can also edit the name
/// shown in the consent sheet before tapping "Continue".
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppleNativeUser {
    /// Given name from `PersonNameComponents.givenName`. Absent when
    /// Apple's components have no `givenName` set or when the user
    /// blanked it before tapping "Continue".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,
    /// Family name from `PersonNameComponents.familyName`. Absent for
    /// the same reasons as [`Self::first_name`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,
}

/// Result of [`crate::AppleAuth::sign_in_with_apple`].
///
/// User-driven outcomes (cancellation, authorization rejection)
/// surface here on the `Ok` path so the caller can map them to UI
/// state without parsing error strings. Bridge failures still
/// propagate as [`crate::Error`].
///
/// Apple's documented `ASAuthorizationError` codes:
///
/// - `.canceled` → [`Cancelled`](Self::Cancelled)
/// - `.failed`, `.invalidResponse`, `.notHandled`, `.unknown`,
///   and anything Apple adds later → [`Rejected`](Self::Rejected)
///
/// `NativeUnavailable` is reserved for "not even reachable" cases —
/// the call site is Android / desktop, or the iOS version predates
/// Sign in with Apple. The frontend treats it as a signal to fall
/// back to the browser flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AppleSignInOutcome {
    /// Apple returned a usable credential. Carries the ID token and
    /// (on first sign-in only) the user's full name.
    Success(SignInResponse),
    /// The user dismissed the system sheet without consenting.
    Cancelled,
    /// Apple refused the request for a reason that isn't user
    /// cancellation. The string is the underlying error's
    /// `localizedDescription` — useful for logging, not for branching.
    Rejected(String),
    /// The platform has no Apple ID framework available
    /// (Android, desktop, very old iOS, missing entitlement).
    NativeUnavailable,
}
