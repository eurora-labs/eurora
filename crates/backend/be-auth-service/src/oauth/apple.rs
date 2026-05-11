//! Sign in with Apple — OAuth client.
//!
//! Apple's OAuth surface has three quirks the Google and GitHub modules
//! don't deal with:
//!
//! 1. **The `client_secret` is a JWT.** ES256-signed over Apple's
//!    EC P-256 private key (`.p8`), max six-month lifetime. We mint a
//!    fresh five-minute JWT for every token-endpoint call rather than
//!    caching: ES256 signing is sub-millisecond, caching adds a
//!    mutable-state surface for nothing, and a shorter lifetime bounds
//!    replay exposure if the JWT is ever captured between minting and
//!    the POST.
//! 2. **`response_mode=form_post`.** Required when requesting
//!    `name`/`email`. Apple POSTs the callback to the backend; the SPA
//!    never sees `code`/`state` — code exchange happens server-side and
//!    the SPA only sees the post-redirect success page.
//! 3. **Dual-audience ID tokens.** Web tokens have `aud = APPLE_SERVICE_ID`;
//!    native iOS tokens have `aud = APPLE_BUNDLE_ID`. We accept both via
//!    a custom `set_other_audience_verifier_fn`, identical pattern to
//!    Google's `ios_client_id` handling.
//!
//! For ID-token verification we still use the [`openidconnect`] crate's
//! `CoreClient`: discovery against `https://appleid.apple.com` builds
//! the JWKS-cached verifier we want, even though Apple's token endpoint
//! is hit manually so we can inject the per-request client secret.

use std::env;
use std::str::FromStr;
use std::time::Duration;

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use openidconnect::{
    ClientId, EndpointMaybeSet, EndpointNotSet, EndpointSet, IssuerUrl, Nonce, PkceCodeChallenge,
    RedirectUrl,
    core::{CoreClient, CoreIdToken, CoreIdTokenClaims, CoreProviderMetadata},
};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

use super::OAuthError;

/// Apple's discovery URL. Apple does support OIDC discovery — the
/// metadata at `/.well-known/openid-configuration` is what gives us the
/// JWKS endpoint to verify ID tokens against.
const APPLE_ISSUER: &str = "https://appleid.apple.com";

/// Apple's token endpoint. We hit this manually (not via the OIDC
/// client) so we can inject a freshly-minted JWT client secret per
/// request — see `mint_client_secret`.
const APPLE_TOKEN_ENDPOINT: &str = "https://appleid.apple.com/auth/token";

/// Apple's authorization endpoint. We build the authorize URL manually
/// so we can add `response_mode=form_post` (which the OIDC builder
/// doesn't expose) and so the wire format stays stable regardless of
/// what the discovery document says.
const APPLE_AUTHORIZE_ENDPOINT: &str = "https://appleid.apple.com/auth/authorize";

/// Lifetime of the per-request client-secret JWT. Apple's ceiling is
/// six months; five minutes is the upper bound for a single token
/// exchange's round-trip even on a pathological network, so anything
/// longer is wasted attack surface.
const CLIENT_SECRET_TTL_SECS: i64 = 5 * 60;

type DiscoveredClient = CoreClient<
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointMaybeSet,
    EndpointMaybeSet,
>;

#[derive(Debug, Clone)]
pub struct AppleOAuthConfig {
    /// 10-character Apple Developer Team ID. Used as the JWT `iss` claim.
    pub team_id: SecretString,
    /// Services ID (e.g. `com.eurora.web`). The OAuth `client_id` for
    /// the web/desktop flow and the expected `aud` of web-issued ID
    /// tokens.
    pub service_id: String,
    /// 10-character Sign-in-with-Apple key identifier. Used as the JWT
    /// `kid` header.
    pub key_id: SecretString,
    /// PEM-encoded EC P-256 private key (`.p8` contents). The
    /// loader (`from_env`) accepts both real-newline and `\n`-escaped
    /// values — see `normalize_pem`.
    pub private_key_pem: SecretString,
    /// Web-flow redirect URI (registered in the Apple Developer Portal).
    pub web_redirect_uri: String,
    /// Optional mobile-flow redirect URI. When unset the mobile path
    /// fails soft at use; mirrors the Google/GitHub convention.
    pub mobile_redirect_uri: Option<String>,
    /// iOS Bundle ID (e.g. `com.eurora.app`). The expected `aud` of
    /// native-iOS ID tokens, returned by `ASAuthorizationController`.
    /// When unset the native-iOS path is unavailable.
    pub bundle_id: Option<String>,
}

impl AppleOAuthConfig {
    pub fn from_env() -> Result<Self, OAuthError> {
        let team_id = SecretString::from(
            env::var("APPLE_TEAM_ID").map_err(|_| OAuthError::MissingEnvVar("APPLE_TEAM_ID"))?,
        );
        let service_id = env::var("APPLE_SERVICE_ID")
            .map_err(|_| OAuthError::MissingEnvVar("APPLE_SERVICE_ID"))?;
        let key_id = SecretString::from(
            env::var("APPLE_KEY_ID").map_err(|_| OAuthError::MissingEnvVar("APPLE_KEY_ID"))?,
        );
        let raw_key = env::var("APPLE_PRIVATE_KEY")
            .map_err(|_| OAuthError::MissingEnvVar("APPLE_PRIVATE_KEY"))?;
        let private_key_pem = SecretString::from(normalize_pem(&raw_key));
        let web_redirect_uri = env::var("APPLE_WEB_REDIRECT_URI")
            .map_err(|_| OAuthError::MissingEnvVar("APPLE_WEB_REDIRECT_URI"))?;
        let mobile_redirect_uri = env::var("APPLE_MOBILE_REDIRECT_URI")
            .ok()
            .filter(|s| !s.is_empty());
        let bundle_id = env::var("APPLE_BUNDLE_ID").ok().filter(|s| !s.is_empty());

        Ok(Self {
            team_id,
            service_id,
            key_id,
            private_key_pem,
            web_redirect_uri,
            mobile_redirect_uri,
            bundle_id,
        })
    }
}

/// Normalise an `APPLE_PRIVATE_KEY` env value.
///
/// Multi-line env vars are routinely garbled by deployment systems
/// (k8s ConfigMaps, fly.io secrets, GitHub Actions secrets — each
/// handles newlines differently). The loader accepts both forms:
///
/// - Real newline-containing PEM (passes through unchanged).
/// - PEM with literal `\n` sequences (gets unescaped). The
///   `&& !raw.contains('\n')` guard means a value that already has
///   real newlines plus stray `\n`-looking text is left alone — the
///   intent is "single-line escaped PEM" detection, not blanket
///   unescaping.
pub(crate) fn normalize_pem(raw: &str) -> String {
    if raw.contains("\\n") && !raw.contains('\n') {
        raw.replace("\\n", "\n")
    } else {
        raw.to_string()
    }
}

pub struct AppleOAuthClient {
    web_redirect_uri: String,
    mobile_redirect_uri: Option<String>,
    service_id: String,
    team_id: SecretString,
    key_id: SecretString,
    /// Parsed once at boot — re-parsing on every mint would force us
    /// to retain the PEM in memory beyond startup. `EncodingKey` is
    /// `Clone` but not `Debug`, so it's safe to keep here.
    encoding_key: EncodingKey,
    /// Audiences besides `service_id` that ID-token verification
    /// accepts. Currently populated with `bundle_id` for the native
    /// iOS path; empty for web-only deployments. Pre-computed at boot
    /// so `verify_id_token` can install the audience callback with a
    /// single `.clone()` instead of re-projecting `accepted_audiences[1..]`
    /// on every call.
    extra_audiences: Vec<String>,
    /// Used only for ID-token verification — JWKS-cached, discovered
    /// once at boot. The token endpoint is hit manually so this
    /// client's redirect-URI binding is irrelevant.
    id_token_verifier_client: DiscoveredClient,
    /// Shared HTTP client kept alive for connection pooling.
    http: reqwest::Client,
}

fn build_http_client() -> Result<reqwest::Client, OAuthError> {
    Ok(reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(30))
        .build()?)
}

impl AppleOAuthClient {
    pub async fn discover(config: AppleOAuthConfig) -> Result<Self, OAuthError> {
        let encoding_key =
            EncodingKey::from_ec_pem(config.private_key_pem.expose_secret().as_bytes())
                .map_err(|_| OAuthError::InvalidConfig("APPLE_PRIVATE_KEY: not a valid EC PEM"))?;

        let issuer_url = IssuerUrl::new(APPLE_ISSUER.to_string())
            .map_err(|e| OAuthError::Discovery(e.to_string()))?;

        let http = build_http_client()?;

        tracing::info!("Discovering Apple OIDC provider metadata");
        let provider_metadata = CoreProviderMetadata::discover_async(issuer_url, &http)
            .await
            .map_err(|e| OAuthError::Discovery(e.to_string()))?;

        // The verifier client's redirect URI is never used (we don't
        // call its `exchange_code`), but the OIDC builder requires one
        // up-front. Reuse the web redirect URI as a placeholder.
        let redirect_url = RedirectUrl::new(config.web_redirect_uri.clone())
            .map_err(|e| OAuthError::InvalidUrl(e.to_string()))?;
        let id_token_verifier_client = CoreClient::from_provider_metadata(
            provider_metadata,
            ClientId::new(config.service_id.clone()),
            None,
        )
        .set_redirect_uri(redirect_url);

        let extra_audiences: Vec<String> = config.bundle_id.iter().cloned().collect();

        Ok(Self {
            web_redirect_uri: config.web_redirect_uri,
            mobile_redirect_uri: config.mobile_redirect_uri,
            service_id: config.service_id,
            team_id: config.team_id,
            key_id: config.key_id,
            encoding_key,
            extra_audiences,
            id_token_verifier_client,
            http,
        })
    }

    pub fn web_redirect_uri(&self) -> &str {
        &self.web_redirect_uri
    }

    pub fn mobile_redirect_uri(&self) -> Option<&str> {
        self.mobile_redirect_uri.as_deref()
    }

    /// Build the web-flow authorize URL.
    ///
    /// The URL is constructed manually (not via the OIDC builder)
    /// because Apple requires `response_mode=form_post` when scopes
    /// include `name`/`email`, and the OIDC builder has no API for
    /// that parameter.
    pub fn authorization_url(
        &self,
        state: &str,
        pkce_challenge: &PkceCodeChallenge,
        nonce: &Nonce,
    ) -> String {
        build_authorization_url(
            &self.service_id,
            &self.web_redirect_uri,
            state,
            pkce_challenge,
            nonce,
        )
    }

    /// Mobile-flow authorize URL. Returns `None` when
    /// `APPLE_MOBILE_REDIRECT_URI` isn't configured.
    pub fn mobile_authorization_url(
        &self,
        state: &str,
        pkce_challenge: &PkceCodeChallenge,
        nonce: &Nonce,
    ) -> Option<String> {
        self.mobile_redirect_uri
            .as_deref()
            .map(|uri| build_authorization_url(&self.service_id, uri, state, pkce_challenge, nonce))
    }

    pub async fn exchange_code(
        &self,
        code: &str,
        pkce_verifier: String,
        nonce: &Nonce,
    ) -> Result<AppleUserInfo, OAuthError> {
        self.exchange_code_with_redirect(code, pkce_verifier, nonce, &self.web_redirect_uri)
            .await
    }

    pub async fn mobile_exchange_code(
        &self,
        code: &str,
        pkce_verifier: String,
        nonce: &Nonce,
    ) -> Result<AppleUserInfo, OAuthError> {
        let redirect_uri = self
            .mobile_redirect_uri
            .as_deref()
            .ok_or(OAuthError::MissingEnvVar("APPLE_MOBILE_REDIRECT_URI"))?;
        self.exchange_code_with_redirect(code, pkce_verifier, nonce, redirect_uri)
            .await
    }

    async fn exchange_code_with_redirect(
        &self,
        code: &str,
        pkce_verifier: String,
        nonce: &Nonce,
        redirect_uri: &str,
    ) -> Result<AppleUserInfo, OAuthError> {
        let client_secret = self.mint_client_secret()?;

        let token_resp: AppleTokenResponse = self
            .http
            .post(APPLE_TOKEN_ENDPOINT)
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", code),
                ("redirect_uri", redirect_uri),
                ("client_id", &self.service_id),
                ("client_secret", client_secret.expose_secret()),
                ("code_verifier", &pkce_verifier),
            ])
            .send()
            .await?
            .error_for_status()
            .map_err(|e| OAuthError::CodeExchange(e.to_string()))?
            .json()
            .await?;

        if let Some(error) = &token_resp.error {
            let desc = token_resp.error_description.as_deref().unwrap_or(error);
            return Err(OAuthError::CodeExchange(desc.to_string()));
        }

        let id_token = token_resp
            .id_token
            .as_deref()
            .ok_or(OAuthError::MissingField("id_token"))?;

        // Verify against Apple's JWKS via the OIDC verifier. Apple's
        // tokens always carry an `email_verified` claim that is `true`
        // (Apple won't issue otherwise), but we surface the field
        // honestly rather than hard-coding it.
        self.verify_id_token(id_token, Some(nonce.secret().as_str()))
    }

    /// Verify an Apple-issued ID token.
    ///
    /// `expected_nonce` is compared byte-for-byte against the JWT's
    /// `nonce` claim. The caller chooses the form:
    ///
    /// - For the **web/desktop code-exchange flow**, that's the raw
    ///   nonce the backend generated and stored on `oauth_state`.
    /// - For the **native iOS flow** (handled in a later PR), that's
    ///   `base64url(sha256(raw_nonce))` — Apple echoes whatever the
    ///   client placed in `request.nonce`, and the iOS plugin pre-hashes.
    ///
    /// Passing `None` skips the nonce check; only do that if the
    /// caller has another replay-protection layer.
    pub fn verify_id_token(
        &self,
        id_token_str: &str,
        expected_nonce: Option<&str>,
    ) -> Result<AppleUserInfo, OAuthError> {
        let id_token = CoreIdToken::from_str(id_token_str)
            .map_err(|e| OAuthError::TokenVerification(format!("malformed id_token: {e}")))?;

        let mut verifier = self.id_token_verifier_client.id_token_verifier();
        if !self.extra_audiences.is_empty() {
            // Permit any audience in the configured allow-list besides
            // the primary `service_id`. The audience-verifier callback
            // requires `'static`, so we clone the pre-computed extras
            // vector once per call.
            let extras = self.extra_audiences.clone();
            verifier = verifier.set_other_audience_verifier_fn(move |aud| {
                extras.iter().any(|a| a == aud.as_str())
            });
        }

        let claims: &CoreIdTokenClaims = match expected_nonce {
            Some(expected) => {
                let expected_owned = expected.to_string();
                id_token.claims(&verifier, |nonce: Option<&Nonce>| match nonce {
                    Some(n) if n.secret() == &expected_owned => Ok(()),
                    _ => Err("nonce mismatch".to_string()),
                })
            }
            None => id_token.claims(&verifier, |_: Option<&Nonce>| Ok(())),
        }
        .map_err(|e| OAuthError::TokenVerification(e.to_string()))?;

        let sub = claims.subject().to_string();
        let email = claims
            .email()
            .ok_or(OAuthError::MissingField("email"))?
            .to_string();
        let email_verified = claims.email_verified().unwrap_or(false);

        if email.ends_with("@privaterelay.appleid.com") {
            // Observability hook: privately-relayed Apple emails can
            // be revoked by the user from the Apple ID dashboard,
            // breaking transactional mail. Surface as an info event
            // so the population is countable in aggregate without
            // attaching per-user correlation keys to the log line.
            tracing::info!("Apple sign-in: using Hide-My-Email private relay");
        }

        Ok(AppleUserInfo {
            sub,
            email,
            email_verified,
            // Apple never carries display name in the ID token — only
            // in the form-post `user` blob / native credential. Caller
            // layers it on top.
            display_name: None,
        })
    }

    /// Mint a fresh `client_secret` JWT for one token-endpoint call.
    fn mint_client_secret(&self) -> Result<SecretString, OAuthError> {
        mint_client_secret(
            self.team_id.expose_secret(),
            &self.service_id,
            self.key_id.expose_secret(),
            &self.encoding_key,
        )
    }
}

/// Mint a fresh `client_secret` JWT.
///
/// ES256 signing is sub-millisecond; minting per request is cheaper
/// than the locking overhead of any cache and removes the only piece
/// of mutable state from `AppleOAuthClient`. The JWT is wrapped in
/// `SecretString` so it zeroises on drop and is never accidentally
/// logged.
///
/// Exposed at module scope (rather than only as a method) so unit
/// tests can exercise the JWT shape without standing up a full
/// `AppleOAuthClient` — the verifier client requires a network round
/// trip to construct.
pub(crate) fn mint_client_secret(
    team_id: &str,
    service_id: &str,
    key_id: &str,
    encoding_key: &EncodingKey,
) -> Result<SecretString, OAuthError> {
    let now = Utc::now().timestamp();
    let claims = ClientSecretClaims {
        iss: team_id,
        sub: service_id,
        aud: APPLE_ISSUER,
        iat: now,
        exp: now + CLIENT_SECRET_TTL_SECS,
    };
    let mut header = Header::new(Algorithm::ES256);
    header.kid = Some(key_id.to_owned());

    let jwt = jsonwebtoken::encode(&header, &claims, encoding_key)
        .map_err(OAuthError::ClientSecretMint)?;
    Ok(SecretString::from(jwt))
}

/// Build the authorize URL Apple expects.
///
/// Manual builder rather than `Url::query_pairs_mut` to keep the
/// parameter order stable across runs — Apple's documentation lists a
/// specific order, and consistency makes the request trivially
/// curl-reproducible from a log line. `state` and `nonce` are
/// percent-encoded by `url::form_urlencoded`.
fn build_authorization_url(
    service_id: &str,
    redirect_uri: &str,
    state: &str,
    pkce_challenge: &PkceCodeChallenge,
    nonce: &Nonce,
) -> String {
    let mut url = url::Url::parse(APPLE_AUTHORIZE_ENDPOINT).expect("static URL must parse");
    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("response_mode", "form_post")
        .append_pair("client_id", service_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("state", state)
        .append_pair("nonce", nonce.secret())
        .append_pair("scope", "name email")
        .append_pair("code_challenge", pkce_challenge.as_str())
        .append_pair("code_challenge_method", "S256");
    url.into()
}

#[derive(Serialize)]
struct ClientSecretClaims<'a> {
    iss: &'a str,
    sub: &'a str,
    aud: &'a str,
    iat: i64,
    exp: i64,
}

#[derive(Deserialize)]
struct AppleTokenResponse {
    id_token: Option<String>,
    #[allow(dead_code)]
    access_token: Option<String>,
    #[allow(dead_code)]
    refresh_token: Option<String>,
    #[allow(dead_code)]
    token_type: Option<String>,
    #[allow(dead_code)]
    expires_in: Option<i64>,
    error: Option<String>,
    error_description: Option<String>,
}

/// Apple-issued identity normalised to the same shape as
/// [`super::google::GoogleUserInfo`].
///
/// Apple never carries display name in the ID token — `display_name`
/// here is `None` after verification; the caller layers it in from the
/// form-post `user` blob (web flow) or the native credential
/// (iOS flow) before calling into `complete_oauth_login`.
///
/// Hide-My-Email status (`@privaterelay.appleid.com`) is observed at
/// the verification boundary via a `tracing::info!` rather than
/// threaded onto the struct: nothing downstream branches on it today,
/// so a field would just go stale.
#[derive(Debug, Clone)]
pub struct AppleUserInfo {
    pub sub: String,
    pub email: String,
    pub email_verified: bool,
    pub display_name: Option<String>,
}

/// Compute `base64url(sha256(raw))`.
///
/// Used by the native-iOS path: Apple echoes whatever the client puts
/// in `request.nonce` verbatim into the ID token's `nonce` claim, and
/// the iOS plugin assigns `request.nonce = base64url(sha256(raw))`.
/// The backend therefore has to re-derive the same value before
/// byte-comparing against the claim.
///
/// Exposed (rather than inlined into the future native handler) so a
/// unit test can pin the algorithm.
#[allow(dead_code)]
pub fn hash_native_nonce(raw: &str) -> String {
    use sha2::{Digest, Sha256};
    URL_SAFE_NO_PAD.encode(Sha256::digest(raw.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{DecodingKey, Validation};

    /// Test PEM keypair (P-256). Generated specifically for unit tests
    /// — not used in any production deployment. The matching public key
    /// (for verification) is `TEST_PUBLIC_PEM`.
    const TEST_PRIVATE_PEM: &str = "-----BEGIN PRIVATE KEY-----\n\
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgevZzL1gdAFr88hb2\n\
OF/2NxApJCzGCEDdfSp6VQO30hyhRANCAAQRWz+jn65BtOMvdyHKcvjBeBSDZH2r\n\
1RTwjmYSi9R/zpBnuQ4EiMnCqfMPWiZqB4QdbAd0E7oH50VpuZ1P087G\n\
-----END PRIVATE KEY-----\n";

    const TEST_PUBLIC_PEM: &str = "-----BEGIN PUBLIC KEY-----\n\
MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEEVs/o5+uQbTjL3chynL4wXgUg2R9\n\
q9UU8I5mEovUf86QZ7kOBIjJwqnzD1omageEHWwHdBO6B+dFabmdT9POxg==\n\
-----END PUBLIC KEY-----\n";

    fn test_encoding_key() -> EncodingKey {
        EncodingKey::from_ec_pem(TEST_PRIVATE_PEM.as_bytes()).expect("test PEM parses")
    }

    #[test]
    fn normalize_pem_passes_real_newlines_through() {
        let raw = "-----BEGIN PRIVATE KEY-----\nbody\n-----END PRIVATE KEY-----\n";
        assert_eq!(normalize_pem(raw), raw);
    }

    #[test]
    fn normalize_pem_unescapes_literal_backslash_n() {
        let raw = "-----BEGIN PRIVATE KEY-----\\nbody\\n-----END PRIVATE KEY-----\\n";
        let normalised = normalize_pem(raw);
        assert!(normalised.contains('\n'));
        assert!(!normalised.contains("\\n"));
        assert!(normalised.starts_with("-----BEGIN PRIVATE KEY-----"));
    }

    #[test]
    fn normalize_pem_leaves_mixed_input_alone() {
        // Real newlines plus stray backslash-n in a comment line:
        // intent is "single-line escaped PEM" detection, not blanket
        // unescaping.
        let raw = "-----BEGIN PRIVATE KEY-----\n// keep \\n\n-----END PRIVATE KEY-----\n";
        assert_eq!(normalize_pem(raw), raw);
    }

    #[test]
    fn malformed_pem_is_rejected() {
        let result = EncodingKey::from_ec_pem(b"not a pem");
        assert!(result.is_err());
    }

    #[test]
    fn mint_client_secret_round_trips_through_decode() {
        let key = test_encoding_key();
        let jwt = mint_client_secret("TEAMID1234", "com.example.web", "KEYID12345", &key)
            .expect("mint succeeds");

        let decoding_key =
            DecodingKey::from_ec_pem(TEST_PUBLIC_PEM.as_bytes()).expect("decoding key parses");
        let mut validation = Validation::new(Algorithm::ES256);
        validation.set_audience(&[APPLE_ISSUER]);
        validation.set_issuer(&["TEAMID1234"]);
        validation.validate_exp = true;
        validation.leeway = 60;

        #[derive(Deserialize)]
        struct Decoded {
            iss: String,
            sub: String,
            aud: String,
            iat: i64,
            exp: i64,
        }

        let data = jsonwebtoken::decode::<Decoded>(jwt.expose_secret(), &decoding_key, &validation)
            .expect("decode succeeds");

        assert_eq!(data.claims.iss, "TEAMID1234");
        assert_eq!(data.claims.sub, "com.example.web");
        assert_eq!(data.claims.aud, APPLE_ISSUER);
        assert_eq!(data.claims.exp - data.claims.iat, CLIENT_SECRET_TTL_SECS);
    }

    #[test]
    fn mint_client_secret_includes_kid_in_header() {
        let key = test_encoding_key();
        let jwt = mint_client_secret("TEAMID1234", "com.example.web", "KEYID12345", &key)
            .expect("mint succeeds");

        let header = jsonwebtoken::decode_header(jwt.expose_secret()).expect("header decodes");
        assert_eq!(header.alg, Algorithm::ES256);
        assert_eq!(header.kid.as_deref(), Some("KEYID12345"));
    }

    #[test]
    fn mint_client_secret_can_be_called_repeatedly() {
        // The mint function holds no state — verify two back-to-back
        // calls both succeed and produce valid JWTs. We deliberately
        // do *not* assert the outputs differ: `jsonwebtoken` uses
        // RFC 6979 deterministic ECDSA, so identical inputs (same
        // `iat` second) yield identical signatures. Determinism is
        // the right property here (better security than random `k`),
        // and the absence of caching is already enforced by the
        // function's pure-by-construction signature.
        let key = test_encoding_key();
        let a = mint_client_secret("TEAMID1234", "com.example.web", "KEYID12345", &key)
            .expect("mint a");
        let b = mint_client_secret("TEAMID1234", "com.example.web", "KEYID12345", &key)
            .expect("mint b");
        // Both decode cleanly — that's the actual contract.
        let _ = jsonwebtoken::decode_header(a.expose_secret()).expect("a header decodes");
        let _ = jsonwebtoken::decode_header(b.expose_secret()).expect("b header decodes");
    }

    #[test]
    fn hash_native_nonce_matches_known_vector() {
        // SHA-256 of the ASCII bytes "abc" is
        // ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad,
        // which base64url-no-pad encodes to:
        let expected = "ungWv48Bz-pBQUDeXa4iI7ADYaOWF3qctBD_YfIAFa0";
        assert_eq!(hash_native_nonce("abc"), expected);
    }

    #[test]
    fn build_authorization_url_includes_form_post_and_scopes() {
        let challenge =
            PkceCodeChallenge::from_code_verifier_sha256(&openidconnect::PkceCodeVerifier::new(
                "verifierverifierverifierverifierverifierxyz".into(),
            ));
        let nonce = Nonce::new("noncevalue".into());
        let url = build_authorization_url(
            "com.example.web",
            "https://api.example.com/auth/oauth/apple/web-callback",
            "the-state",
            &challenge,
            &nonce,
        );
        assert!(url.contains("response_mode=form_post"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=com.example.web"));
        assert!(url.contains("scope=name+email"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("state=the-state"));
        assert!(url.contains("nonce=noncevalue"));
    }
}
