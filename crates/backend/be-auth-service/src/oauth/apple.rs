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
use std::sync::Arc;
use std::time::{Duration, Instant};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, decode_header,
    jwk::{AlgorithmParameters, JwkSet},
};
use openidconnect::{
    ClientId, EndpointMaybeSet, EndpointNotSet, EndpointSet, IssuerUrl, Nonce, PkceCodeChallenge,
    RedirectUrl,
    core::{CoreClient, CoreIdToken, CoreIdTokenClaims, CoreProviderMetadata},
};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize, de::Deserializer};
use tokio::sync::RwLock;

use super::OAuthError;

#[cfg(test)]
mod notification_test_fixtures;

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

/// Apple's JWKS endpoint. Used to verify server-to-server notification
/// JWTs and (indirectly via the OIDC client) ID tokens. Hard-coded
/// because Apple has shipped this URL for the lifetime of the service —
/// a change here would break Sign in with Apple across every consumer
/// at once.
const APPLE_JWKS_ENDPOINT: &str = "https://appleid.apple.com/auth/keys";

/// Lifetime of the per-request client-secret JWT. Apple's ceiling is
/// six months; five minutes is the upper bound for a single token
/// exchange's round-trip even on a pathological network, so anything
/// longer is wasted attack surface.
const CLIENT_SECRET_TTL_SECS: i64 = 5 * 60;

/// Maximum age (in seconds) of an Apple server-to-server notification
/// JWT, measured against `iat`. The 10-minute window absorbs clock skew
/// without opening a meaningful replay door — Apple's own retry cadence
/// is shorter, and the side-effects we perform are idempotent anyway.
const NOTIFICATION_FRESHNESS_SECS: i64 = 600;

/// Leeway applied to the *future* side of the `iat` check. Symmetric
/// rejection means a forged token with `iat` arbitrarily in the future
/// can't bypass replay defence by inverting the sign of the freshness
/// delta. Matches the [`Validation::leeway`] used for `exp` validation
/// so the two clocks agree on what "close enough" means.
const NOTIFICATION_FUTURE_IAT_LEEWAY_SECS: i64 = 60;

/// JWKS cache TTL. Apple rotates keys infrequently; an hour keeps the
/// hot path off the network without making rotations user-visible.
/// Refreshes also happen on `kid` miss (subject to the rate cap below),
/// so a rotation propagates well before this TTL elapses in practice.
const JWKS_CACHE_TTL: Duration = Duration::from_secs(3_600);

/// Minimum wall-clock interval between refresh attempts, regardless of
/// `kid` misses. Prevents a rogue caller from spamming JWTs with
/// unknown `kid`s to force unbounded fetches against Apple's endpoint.
const JWKS_REFRESH_MIN_INTERVAL: Duration = Duration::from_secs(60);

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
    /// iOS Bundle IDs accepted as ID-token audiences on the native
    /// path. Empty when no `APPLE_BUNDLE_ID` is configured — the
    /// native-iOS flow then has no acceptable audience and is
    /// effectively unavailable.
    ///
    /// Multiple values let one backend simultaneously accept tokens
    /// from several iOS builds — typically the production bundle,
    /// the `.dev` dev-build bundle, and the `.nightly` TestFlight
    /// bundle. The env-var form is comma-separated:
    ///
    /// ```text
    /// APPLE_BUNDLE_ID=com.eurora-labs.eurora,com.eurora-labs.eurora.dev,com.eurora-labs.eurora.nightly
    /// ```
    ///
    /// Parsed by [`parse_bundle_ids`]; surrounding whitespace and
    /// empty entries are dropped, duplicates kept as-is (the
    /// underlying audience-verifier callback short-circuits on the
    /// first match anyway).
    pub bundle_ids: Vec<String>,
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
        let bundle_ids = env::var("APPLE_BUNDLE_ID")
            .ok()
            .map(|raw| parse_bundle_ids(&raw))
            .unwrap_or_default();

        Ok(Self {
            team_id,
            service_id,
            key_id,
            private_key_pem,
            web_redirect_uri,
            mobile_redirect_uri,
            bundle_ids,
        })
    }
}

/// Parse `APPLE_BUNDLE_ID` into the list of bundle IDs accepted as
/// ID-token audiences on the native iOS path.
///
/// The wire format is a comma-separated list with optional surrounding
/// whitespace; empty entries (`"a,,b"`, trailing comma) are silently
/// dropped. Dropping rather than erroring keeps the loader tolerant of
/// the kinds of accidents that creep in through deployment systems
/// (trailing commas, accidental double-commas after edits), and
/// downstream verification will simply reject any audience whose
/// expected match is missing from the resulting list.
pub(crate) fn parse_bundle_ids(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
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
    /// accepts. Currently populated with the configured iOS Bundle IDs
    /// for the native iOS path; empty for web-only deployments. Pre-
    /// computed at boot so `verify_id_token` can install the audience
    /// callback with a single `.clone()` instead of re-projecting
    /// `accepted_audiences[1..]` on every call.
    extra_audiences: Vec<String>,
    /// Used only for ID-token verification — JWKS-cached, discovered
    /// once at boot. The token endpoint is hit manually so this
    /// client's redirect-URI binding is irrelevant.
    id_token_verifier_client: DiscoveredClient,
    /// Shared HTTP client kept alive for connection pooling.
    http: reqwest::Client,
    /// JWKS used exclusively for verifying Apple's server-to-server
    /// notification JWTs. Kept separate from
    /// `id_token_verifier_client` because `openidconnect`'s JWKS-
    /// caching machinery is bound to the ID-token verifier surface,
    /// while notification verification needs `jsonwebtoken` (different
    /// claim shape, no nonce). Both caches ultimately point at the
    /// same Apple endpoint, so they stay in lock-step in practice; the
    /// duplication is the cost of decoupling the two verifiers.
    notification_jwks: Arc<RwLock<JwksCache>>,
}

/// Cached Apple JWKS with a wall-clock timestamp for TTL and rate-limit
/// decisions.
///
/// The cache is populated at boot (one network round-trip during
/// `discover`) and refreshed lazily under three conditions:
///
/// 1. Cache age exceeds [`JWKS_CACHE_TTL`] (the steady-state refresh path).
/// 2. A notification arrives with an unknown `kid` and the rate-limit
///    gate has cleared (the key-rotation refresh path).
/// 3. Cache is empty because the boot-time fetch failed in a soft mode
///    (currently impossible — `discover` propagates that error).
///
/// `fetched_at` is `Instant` rather than `DateTime<Utc>` so wall-clock
/// jumps (NTP adjustments, system suspend) don't invalidate the cache
/// or starve the refresh path.
struct JwksCache {
    keys: JwkSet,
    fetched_at: Instant,
    /// Last attempted fetch, regardless of success. Drives the
    /// [`JWKS_REFRESH_MIN_INTERVAL`] rate-cap so a stream of unknown
    /// `kid`s can't force unbounded outbound traffic to Apple.
    last_refresh_attempt: Instant,
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

        let extra_audiences = config.bundle_ids;

        // Pre-populate the notification JWKS cache: one extra
        // round-trip at boot keeps the first inbound notification off
        // the network. Failures here fail the boot — Apple is
        // configured but the JWKS endpoint is unreachable, which is
        // almost certainly a deployment-side misconfiguration we want
        // surfaced loudly.
        tracing::info!("Fetching Apple JWKS for notification verification");
        let keys = fetch_apple_jwks(&http).await?;
        tracing::info!(
            key_count = keys.keys.len(),
            "Apple JWKS cache populated at boot",
        );
        let now = Instant::now();
        let notification_jwks = Arc::new(RwLock::new(JwksCache {
            keys,
            fetched_at: now,
            last_refresh_attempt: now,
        }));

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
            notification_jwks,
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

    /// Verify an Apple-signed server-to-server notification JWT.
    ///
    /// Returns the parsed event payload on success. On failure returns
    /// either [`OAuthError::NotificationVerification`] (signature /
    /// issuer / audience / structural error) or
    /// [`OAuthError::NotificationOutsideFreshnessWindow`] (`iat` more
    /// than [`NOTIFICATION_FRESHNESS_SECS`] in the past or more than
    /// [`NOTIFICATION_FUTURE_IAT_LEEWAY_SECS`] in the future). The
    /// handler maps both to **401** to avoid silently dropping
    /// forgeries with a 200.
    ///
    /// The freshness check is a replay defence rather than a primary
    /// security boundary — signature + audience + issuer already
    /// authenticate the message. The window forecloses an attacker
    /// who somehow captures a real notification from replaying it
    /// indefinitely (e.g. via a long-lived log capture). The
    /// future-side leeway forecloses the inverse: a forged `iat` in
    /// the future would otherwise pass an asymmetric "too old" check.
    ///
    /// Side-effects: may refresh the JWKS cache exactly once per call,
    /// gated by [`JWKS_REFRESH_MIN_INTERVAL`].
    pub async fn verify_notification(
        &self,
        payload_jwt: &str,
    ) -> Result<AppleNotificationEvent, OAuthError> {
        let header = parse_notification_header(payload_jwt)?;
        let kid = header
            .kid
            .as_deref()
            .ok_or_else(|| OAuthError::NotificationVerification("missing kid in header".into()))?;
        let decoding_key = self.resolve_jwk_decoding_key(kid).await?;
        verify_notification_inner(
            payload_jwt,
            &header,
            &self.service_id,
            &decoding_key,
            Utc::now().timestamp(),
        )
    }

    /// Look up a JWK by `kid`, refreshing the cache once on miss.
    ///
    /// Refresh is rate-limited by [`JWKS_REFRESH_MIN_INTERVAL`]; once
    /// the interval elapses the next miss triggers a single network
    /// round-trip. A successful refresh updates `fetched_at`; an
    /// unsuccessful refresh still updates `last_refresh_attempt` so
    /// the rate-cap holds even when Apple is down.
    async fn resolve_jwk_decoding_key(&self, kid: &str) -> Result<DecodingKey, OAuthError> {
        if let Some(key) = self.try_jwk_from_cache(kid).await? {
            return Ok(key);
        }
        self.refresh_jwks_if_due().await?;
        match self.try_jwk_from_cache(kid).await? {
            Some(key) => Ok(key),
            None => Err(OAuthError::NotificationVerification(format!(
                "unknown kid {kid} (not present in JWKS after refresh)"
            ))),
        }
    }

    /// Read-side helper: look up a JWK in the current cache and
    /// convert it to a [`DecodingKey`]. Returns `Ok(None)` for "kid
    /// not in cache" so the caller can decide whether to refresh.
    async fn try_jwk_from_cache(&self, kid: &str) -> Result<Option<DecodingKey>, OAuthError> {
        let guard = self.notification_jwks.read().await;
        match guard.keys.find(kid) {
            Some(jwk) => {
                // Apple's JWKS publishes RSA keys (`AlgorithmParameters::RSA`).
                // Rejecting non-RSA keys up-front gives a precise error
                // (rather than a downstream `from_jwk` failure) and
                // future-proofs against forgeries that smuggle an
                // unexpected key type into a kid lookup.
                if !matches!(jwk.algorithm, AlgorithmParameters::RSA(_)) {
                    return Err(OAuthError::NotificationVerification(format!(
                        "JWK for kid {kid} is not RSA"
                    )));
                }
                let key = DecodingKey::from_jwk(jwk).map_err(|e| {
                    OAuthError::NotificationVerification(format!("invalid JWK: {e}"))
                })?;
                Ok(Some(key))
            }
            None => Ok(None),
        }
    }

    /// Conditionally refresh the JWKS cache.
    ///
    /// Refresh runs iff TTL has elapsed _or_ the rate-cap has cleared
    /// since the previous attempt. Two concurrent callers that arrive
    /// while neither condition holds both no-op; the one that wins
    /// the write-lock race is the only one that performs the fetch.
    async fn refresh_jwks_if_due(&self) -> Result<(), OAuthError> {
        // Cheap read first: skip the write-lock acquisition entirely
        // when both gates clearly haven't cleared. This keeps the
        // steady-state cost of an unknown `kid` (e.g. a fuzzer) bounded
        // to one read-lock acquisition per request.
        {
            let guard = self.notification_jwks.read().await;
            let now = Instant::now();
            let stale = now.duration_since(guard.fetched_at) >= JWKS_CACHE_TTL;
            let rate_cleared =
                now.duration_since(guard.last_refresh_attempt) >= JWKS_REFRESH_MIN_INTERVAL;
            if !stale && !rate_cleared {
                return Ok(());
            }
        }

        let mut guard = self.notification_jwks.write().await;
        // Re-check under the write lock: another task may have
        // refreshed between us dropping the read lock and acquiring
        // the write lock. If they did, we're done.
        let now = Instant::now();
        if now.duration_since(guard.last_refresh_attempt) < JWKS_REFRESH_MIN_INTERVAL
            && now.duration_since(guard.fetched_at) < JWKS_CACHE_TTL
        {
            return Ok(());
        }
        guard.last_refresh_attempt = now;

        let previous_age_secs = now.duration_since(guard.fetched_at).as_secs();
        match fetch_apple_jwks(&self.http).await {
            Ok(keys) => {
                let key_count = keys.keys.len();
                guard.keys = keys;
                guard.fetched_at = Instant::now();
                tracing::info!(key_count, previous_age_secs, "Refreshed Apple JWKS cache",);
                Ok(())
            }
            Err(e) => {
                // Don't propagate fetch failures up to the caller:
                // the cache still contains the prior keys, which may
                // be sufficient. The verifier returns "unknown kid"
                // if the lookup still misses, which is the
                // semantically correct error for the caller.
                tracing::warn!(
                    error = %e,
                    previous_age_secs,
                    "Apple JWKS refresh failed; keeping stale cache",
                );
                Ok(())
            }
        }
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

/// Parsed Apple server-to-server notification event.
///
/// One event per JWT — Apple delivers each event type in its own
/// envelope, even when several state changes happen close together.
/// `event_time_ms` is preserved (rather than reduced to a chrono
/// `DateTime`) so the original wire value survives into structured
/// logs, where ops can correlate against Apple's developer-portal
/// history without a unit-conversion step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppleNotificationEvent {
    pub sub: String,
    pub event_time_ms: i64,
    pub kind: AppleEventKind,
}

/// Apple's four documented event types, plus a forwards-compatible
/// `Unknown` arm that surfaces an unrecognised event-type string.
///
/// Variant payloads cluster the fields that actually accompany each
/// event per Apple's docs — `email` / `is_private_email` are only
/// present on the email-toggle events, never on revocation. Pulling
/// them onto the type avoids `Option`s the caller would have to
/// destructure with a "should always be Some here" comment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppleEventKind {
    /// User revoked this app's Sign-in-with-Apple consent from iOS
    /// Settings. The Apple ID still exists; we can no longer
    /// authenticate this user via Apple.
    ConsentRevoked,
    /// User deleted their _entire_ Apple ID. Same in-system action as
    /// `ConsentRevoked`; preserved as a distinct variant so forensic
    /// logs can tell which path the user took.
    AccountDelete,
    /// Hide-My-Email forwarding for this app was turned off. We don't
    /// route mail through the relay today; logged-only.
    EmailDisabled {
        email: String,
        is_private_email: bool,
    },
    /// Hide-My-Email forwarding was turned back on. Logged-only.
    EmailEnabled {
        email: String,
        is_private_email: bool,
    },
    /// Event-type string that doesn't match any of the four known
    /// variants. The string is preserved so a future-proof handler
    /// can log it without losing forwards-compat. Apple will retry
    /// non-2xx responses, so the surrounding code must 200-and-log
    /// rather than fail-and-retry on this arm.
    Unknown(String),
}

/// Inner JWT claims of an Apple notification.
///
/// `events` is a stringified JSON object (Apple's choice, presumably
/// to keep the outer envelope schema stable across event-type
/// additions). The shape is decoded in two steps: this struct
/// captures the outer envelope; `RawAppleEvent` is decoded from
/// `events` separately in [`AppleOAuthClient::verify_notification`].
///
/// `iss`, `aud`, and `exp` are validated by `jsonwebtoken::decode`
/// against the JWT JSON directly (before deserialisation into this
/// struct), so they don't need to appear here.
#[derive(Debug, Deserialize)]
struct NotificationClaims {
    iat: i64,
    events: String,
}

/// Inner event payload, decoded from the stringified `events` claim.
#[derive(Debug, Deserialize)]
struct RawAppleEvent {
    #[serde(rename = "type")]
    kind: String,
    sub: String,
    /// Milliseconds since the Unix epoch. Apple's docs explicitly
    /// say "in millis". The field is `i64` (not `u64`) because
    /// chrono's `timestamp_millis` returns `i64` everywhere else
    /// in the crate; keeping types in lockstep avoids spurious casts
    /// at log boundaries.
    event_time: i64,
    #[serde(default)]
    email: Option<String>,
    /// `is_private_email` arrives as either a JSON boolean or a
    /// stringified boolean depending on event source. Decode both
    /// shapes — silently treating `"true"` as falsy would mislabel
    /// every Hide-My-Email toggle.
    #[serde(default, deserialize_with = "deserialize_bool_or_string")]
    is_private_email: Option<bool>,
}

/// Parse and validate-shape an Apple notification JWT header.
///
/// Centralises the "malformed header" error wording so the
/// `verify_notification` look-ahead (for `kid`) and the inner
/// verifier path can share it without drifting. Doesn't validate
/// algorithm or signature — that happens in
/// [`verify_notification_inner`].
fn parse_notification_header(payload_jwt: &str) -> Result<jsonwebtoken::Header, OAuthError> {
    decode_header(payload_jwt)
        .map_err(|e| OAuthError::NotificationVerification(format!("malformed header: {e}")))
}

/// Pure verifier path for an Apple notification JWT.
///
/// Given a pre-parsed header and a [`DecodingKey`] (already resolved
/// from the JWKS cache), this function:
///
/// 1. Rejects non-RS256 headers — accepting `HS256` against a public
///    key would let an attacker forge tokens by HMAC-ing with the
///    public key as the secret, the classic JWT-downgrade footgun.
/// 2. Validates signature, issuer (`appleid.apple.com`), audience
///    (must equal `accepted_audience`, normally the Services ID),
///    and expiry via [`jsonwebtoken::Validation`].
/// 3. Applies the symmetric freshness window against the caller-
///    supplied `now_ts` — injectable so tests can pin clock behaviour
///    without monkey-patching `Utc::now`.
/// 4. Decodes the stringified `events` claim and projects it onto
///    [`AppleNotificationEvent`].
///
/// Extracted from [`AppleOAuthClient::verify_notification`] so unit
/// tests can exercise verification end-to-end without standing up
/// the OIDC client (which requires a discovery round-trip). The
/// caller supplies the parsed header so the same `decode_header`
/// result drives both the `kid` lookup and the algorithm check —
/// no duplicate parsing on the hot path.
pub(crate) fn verify_notification_inner(
    payload_jwt: &str,
    header: &jsonwebtoken::Header,
    accepted_audience: &str,
    decoding_key: &DecodingKey,
    now_ts: i64,
) -> Result<AppleNotificationEvent, OAuthError> {
    if header.alg != Algorithm::RS256 {
        return Err(OAuthError::NotificationVerification(format!(
            "unexpected algorithm {:?}; expected RS256",
            header.alg
        )));
    }

    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_issuer(&[APPLE_ISSUER]);
    validation.set_audience(&[accepted_audience]);
    validation.validate_exp = true;
    validation.leeway = 60;

    let token = decode::<NotificationClaims>(payload_jwt, decoding_key, &validation)
        .map_err(|e| OAuthError::NotificationVerification(e.to_string()))?;
    let claims = token.claims;

    // Symmetric `iat` window: tokens claiming to be issued too far in
    // either direction are rejected. The past-side bound is the replay
    // defence; the future-side bound forecloses an attacker inverting
    // the delta sign by forging `iat` in the future. `i64` subtraction
    // is safe here — both values are seconds-since-epoch and the
    // difference always fits.
    let delta = now_ts - claims.iat;
    let fresh_window = -NOTIFICATION_FUTURE_IAT_LEEWAY_SECS..=NOTIFICATION_FRESHNESS_SECS;
    if !fresh_window.contains(&delta) {
        return Err(OAuthError::NotificationOutsideFreshnessWindow {
            iat: claims.iat,
            now: now_ts,
        });
    }

    // Apple's `events` claim is a **stringified** JSON object, not a
    // nested object. Forgetting the second decode would silently
    // produce empty events and miss real revocations.
    let raw_event: RawAppleEvent = serde_json::from_str(&claims.events).map_err(|e| {
        OAuthError::NotificationVerification(format!("malformed events claim: {e}"))
    })?;

    Ok(parse_event(raw_event))
}

fn parse_event(raw: RawAppleEvent) -> AppleNotificationEvent {
    let kind = match raw.kind.as_str() {
        "consent-revoked" => AppleEventKind::ConsentRevoked,
        "account-delete" => AppleEventKind::AccountDelete,
        "email-disabled" => AppleEventKind::EmailDisabled {
            email: raw.email.unwrap_or_default(),
            is_private_email: raw.is_private_email.unwrap_or(false),
        },
        "email-enabled" => AppleEventKind::EmailEnabled {
            email: raw.email.unwrap_or_default(),
            is_private_email: raw.is_private_email.unwrap_or(false),
        },
        other => AppleEventKind::Unknown(other.to_owned()),
    };
    AppleNotificationEvent {
        sub: raw.sub,
        event_time_ms: raw.event_time,
        kind,
    }
}

/// Deserialize a value that may be either a JSON boolean
/// (`true`/`false`) or a stringified boolean (`"true"`/`"false"`).
///
/// Apple has historically sent `is_private_email` in both shapes
/// depending on which subsystem emitted the event; the doc lags the
/// wire format. Returning `Err` for invalid strings (rather than
/// `Ok(None)`) catches typos at the parser boundary instead of
/// silently coercing them to `false`.
fn deserialize_bool_or_string<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum BoolOrString {
        Bool(bool),
        Str(String),
    }

    let raw = Option::<BoolOrString>::deserialize(deserializer)?;
    match raw {
        None => Ok(None),
        Some(BoolOrString::Bool(b)) => Ok(Some(b)),
        Some(BoolOrString::Str(s)) => match s.as_str() {
            "true" => Ok(Some(true)),
            "false" => Ok(Some(false)),
            other => Err(D::Error::custom(format!(
                "expected boolean or \"true\"/\"false\" string, got {other:?}"
            ))),
        },
    }
}

/// Fetch and parse Apple's JWKS from [`APPLE_JWKS_ENDPOINT`].
///
/// Exposed at module scope so both [`AppleOAuthClient::discover`] (boot
/// fetch) and the refresh path inside `verify_notification` can call
/// the same code. Failures wrap into [`OAuthError::JwksFetch`] which
/// preserves the underlying `reqwest::Error` as a `source` chain — the
/// `Display` impl deliberately stays generic so the public error
/// surface doesn't leak Apple-side response detail.
async fn fetch_apple_jwks(http: &reqwest::Client) -> Result<JwkSet, OAuthError> {
    http.get(APPLE_JWKS_ENDPOINT)
        .send()
        .await
        .map_err(OAuthError::JwksFetch)?
        .error_for_status()
        .map_err(OAuthError::JwksFetch)?
        .json::<JwkSet>()
        .await
        .map_err(OAuthError::JwksFetch)
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
    fn parse_bundle_ids_handles_single_value() {
        // Backwards-compat path: a deployment that pre-dates
        // multi-bundle support uses a single value and continues to
        // work unchanged.
        assert_eq!(
            parse_bundle_ids("com.eurora-labs.eurora"),
            vec!["com.eurora-labs.eurora"],
        );
    }

    #[test]
    fn parse_bundle_ids_splits_on_commas() {
        assert_eq!(
            parse_bundle_ids(
                "com.eurora-labs.eurora,com.eurora-labs.eurora.dev,com.eurora-labs.eurora.nightly",
            ),
            vec![
                "com.eurora-labs.eurora",
                "com.eurora-labs.eurora.dev",
                "com.eurora-labs.eurora.nightly",
            ],
        );
    }

    #[test]
    fn parse_bundle_ids_trims_whitespace() {
        // Operator-formatted env files often put spaces after commas
        // — accept the natural shape rather than forcing a no-space
        // convention nobody remembers.
        assert_eq!(
            parse_bundle_ids("  com.eurora-labs.eurora , com.eurora-labs.eurora.dev  "),
            vec!["com.eurora-labs.eurora", "com.eurora-labs.eurora.dev"],
        );
    }

    #[test]
    fn parse_bundle_ids_drops_empty_entries() {
        // Trailing commas, accidental double-commas, and an entirely
        // whitespace token (`" "`) all reduce to dropped entries —
        // the alternative (erroring) would brick a backend over an
        // editor-introduced typo in a Sign-in flow we'd rather see
        // run.
        assert_eq!(
            parse_bundle_ids("com.eurora-labs.eurora,,com.eurora-labs.eurora.dev, ,"),
            vec!["com.eurora-labs.eurora", "com.eurora-labs.eurora.dev"],
        );
    }

    #[test]
    fn parse_bundle_ids_returns_empty_for_empty_input() {
        // `APPLE_BUNDLE_ID=""` (set-but-empty) and missing entirely
        // both reduce to an empty list. The downstream effect — the
        // native iOS audience set is empty, so no native ID token can
        // verify — is correct in either case.
        assert!(parse_bundle_ids("").is_empty());
        assert!(parse_bundle_ids("   ").is_empty());
        assert!(parse_bundle_ids(",,, ").is_empty());
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

    // ---------- Apple server-to-server notification tests ----------
    //
    // These tests pin notification verification end-to-end at the pure
    // layer ([`verify_notification_inner`]) — JWKS lookup and HTTP
    // refresh live in `AppleOAuthClient` and are covered by direct
    // JwksCache assertions plus future integration tests. The
    // cryptographic fixtures (RSA keypairs, JWK builder, signing
    // helpers) live in [`super::notification_test_fixtures`] so the
    // production source file isn't carrying ~70 lines of inert
    // PEM data.
    use super::notification_test_fixtures::{
        FORGED_PRIVATE_PEM, NOTIF_PUBLIC_N, TEST_KID, TEST_SERVICE_ID, envelope,
        events_consent_revoked, notif_decoding_key_from_jwk, sign_envelope, sign_envelope_with,
    };

    /// Test-side wrapper that walks the same path the production
    /// `verify_notification` uses: parse the header once, then hand
    /// the parsed header to `verify_notification_inner`. Keeps the
    /// call sites below tidy and ensures a header-parse regression
    /// surfaces from every test, not just one.
    fn verify_for_test(jwt: &str, now: i64) -> Result<AppleNotificationEvent, OAuthError> {
        let header = parse_notification_header(jwt)?;
        verify_notification_inner(
            jwt,
            &header,
            TEST_SERVICE_ID,
            &notif_decoding_key_from_jwk(),
            now,
        )
    }

    #[test]
    fn verify_notification_inner_happy_consent_revoked() {
        let now = Utc::now().timestamp();
        let events = events_consent_revoked("user-sub-001", now * 1000);
        let jwt = sign_envelope(envelope(now, now + 600, &events));

        let event = verify_for_test(&jwt, now).expect("verifies");

        assert_eq!(event.sub, "user-sub-001");
        assert_eq!(event.event_time_ms, now * 1000);
        assert_eq!(event.kind, AppleEventKind::ConsentRevoked);
    }

    #[test]
    fn verify_notification_inner_happy_account_delete() {
        let now = Utc::now().timestamp();
        let events = serde_json::json!({
            "type": "account-delete",
            "sub": "u",
            "event_time": now * 1000,
        })
        .to_string();
        let jwt = sign_envelope(envelope(now, now + 600, &events));

        let event = verify_for_test(&jwt, now).expect("verifies");
        assert_eq!(event.kind, AppleEventKind::AccountDelete);
    }

    #[test]
    fn verify_notification_inner_email_disabled_with_string_bool() {
        // Apple historically sends `is_private_email` as a JSON
        // string ("true"/"false") on some event sources. The
        // deserializer must accept both shapes; this test pins the
        // string-flavoured one.
        let now = Utc::now().timestamp();
        let events = serde_json::json!({
            "type": "email-disabled",
            "sub": "u",
            "event_time": now * 1000,
            "email": "x@privaterelay.appleid.com",
            "is_private_email": "true",
        })
        .to_string();
        let jwt = sign_envelope(envelope(now, now + 600, &events));

        let event = verify_for_test(&jwt, now).expect("verifies");
        assert_eq!(
            event.kind,
            AppleEventKind::EmailDisabled {
                email: "x@privaterelay.appleid.com".into(),
                is_private_email: true,
            }
        );
    }

    #[test]
    fn verify_notification_inner_email_enabled_with_bool() {
        // Boolean (not string) shape — must work too.
        let now = Utc::now().timestamp();
        let events = serde_json::json!({
            "type": "email-enabled",
            "sub": "u",
            "event_time": now * 1000,
            "email": "x@example.com",
            "is_private_email": false,
        })
        .to_string();
        let jwt = sign_envelope(envelope(now, now + 600, &events));

        let event = verify_for_test(&jwt, now).expect("verifies");
        assert_eq!(
            event.kind,
            AppleEventKind::EmailEnabled {
                email: "x@example.com".into(),
                is_private_email: false,
            }
        );
    }

    #[test]
    fn verify_notification_inner_unknown_event_type_preserves_string() {
        let now = Utc::now().timestamp();
        let events = serde_json::json!({
            "type": "future-event-kind",
            "sub": "u",
            "event_time": now * 1000,
        })
        .to_string();
        let jwt = sign_envelope(envelope(now, now + 600, &events));

        let event = verify_for_test(&jwt, now).expect("verifies");
        assert_eq!(
            event.kind,
            AppleEventKind::Unknown("future-event-kind".into())
        );
    }

    #[test]
    fn verify_notification_inner_rejects_forged_signature() {
        let now = Utc::now().timestamp();
        let events = events_consent_revoked("u", now * 1000);
        let forged_key =
            EncodingKey::from_rsa_pem(FORGED_PRIVATE_PEM.as_bytes()).expect("forged key parses");
        let jwt = sign_envelope_with(
            &forged_key,
            Some(TEST_KID),
            envelope(now, now + 600, &events),
            Algorithm::RS256,
        );

        let err = verify_for_test(&jwt, now).expect_err("forged signature must be rejected");
        assert!(matches!(err, OAuthError::NotificationVerification(_)));
    }

    #[test]
    fn verify_notification_inner_rejects_unexpected_algorithm() {
        // Build a token signed with HS256 against a fixed secret. The
        // verifier should refuse before attempting signature
        // validation against the RSA public key — this is the JWT
        // algorithm-downgrade defence.
        let now = Utc::now().timestamp();
        let events = events_consent_revoked("u", now * 1000);
        let hmac_key = EncodingKey::from_secret(b"hs256-secret-not-the-public-key");
        let jwt = sign_envelope_with(
            &hmac_key,
            Some(TEST_KID),
            envelope(now, now + 600, &events),
            Algorithm::HS256,
        );

        let err = verify_for_test(&jwt, now).expect_err("HS256 must be refused");
        match err {
            OAuthError::NotificationVerification(msg) => {
                assert!(
                    msg.contains("RS256"),
                    "error must name the expected alg: {msg}"
                )
            }
            other => panic!("expected NotificationVerification, got {other:?}"),
        }
    }

    #[test]
    fn verify_notification_inner_rejects_wrong_audience() {
        let now = Utc::now().timestamp();
        let events = events_consent_revoked("u", now * 1000);
        let mut env = envelope(now, now + 600, &events);
        env["aud"] = serde_json::json!("com.attacker.evil");
        let jwt = sign_envelope(env);

        let err = verify_for_test(&jwt, now).expect_err("wrong aud must be rejected");
        assert!(matches!(err, OAuthError::NotificationVerification(_)));
    }

    #[test]
    fn verify_notification_inner_rejects_wrong_issuer() {
        let now = Utc::now().timestamp();
        let events = events_consent_revoked("u", now * 1000);
        let mut env = envelope(now, now + 600, &events);
        env["iss"] = serde_json::json!("https://impersonator.example.com");
        let jwt = sign_envelope(env);

        let err = verify_for_test(&jwt, now).expect_err("wrong iss must be rejected");
        assert!(matches!(err, OAuthError::NotificationVerification(_)));
    }

    #[test]
    fn verify_notification_inner_rejects_expired_token() {
        let now = Utc::now().timestamp();
        let events = events_consent_revoked("u", now * 1000);
        // `exp` 10 minutes in the past — outside the 60s leeway.
        let jwt = sign_envelope(envelope(now - 1_000, now - 600, &events));

        let err = verify_for_test(&jwt, now).expect_err("expired token must be rejected");
        assert!(matches!(err, OAuthError::NotificationVerification(_)));
    }

    #[test]
    fn verify_notification_inner_rejects_stale_iat_outside_freshness_window() {
        // `iat` 11 minutes in the past — outside the 10-minute
        // freshness window. `exp` is still future so the validator
        // doesn't reject early.
        let now = Utc::now().timestamp();
        let iat = now - 11 * 60;
        let events = events_consent_revoked("u", iat * 1000);
        let jwt = sign_envelope(envelope(iat, iat + 6 * 3600, &events));

        let err = verify_for_test(&jwt, now).expect_err("stale iat must be rejected");
        match err {
            OAuthError::NotificationOutsideFreshnessWindow {
                iat: e_iat,
                now: e_now,
            } => {
                assert_eq!(e_iat, iat);
                assert_eq!(e_now, now);
            }
            other => panic!("expected NotificationOutsideFreshnessWindow, got {other:?}"),
        }
    }

    #[test]
    fn verify_notification_inner_rejects_future_iat_outside_freshness_window() {
        // `iat` 2 hours in the future — well past the future-side
        // leeway. Without the symmetric bound, `now - iat` would be
        // negative and slip under the past-side comparison; the
        // verifier must still reject.
        let now = Utc::now().timestamp();
        let iat = now + 2 * 3600;
        let events = events_consent_revoked("u", iat * 1000);
        let jwt = sign_envelope(envelope(iat, iat + 6 * 3600, &events));

        let err = verify_for_test(&jwt, now).expect_err("future iat must be rejected");
        assert!(matches!(
            err,
            OAuthError::NotificationOutsideFreshnessWindow { .. }
        ));
    }

    #[test]
    fn verify_notification_inner_accepts_iat_within_future_leeway() {
        // `iat` 30s in the future — inside the 60s future-side leeway.
        // Models a small forward clock skew on Apple's side.
        let now = Utc::now().timestamp();
        let iat = now + 30;
        let events = events_consent_revoked("u", iat * 1000);
        let jwt = sign_envelope(envelope(iat, iat + 6 * 3600, &events));

        verify_for_test(&jwt, now).expect("30s future skew must be accepted");
    }

    #[test]
    fn verify_notification_inner_accepts_fresh_iat_inside_freshness_window() {
        // `iat` 9 minutes in the past — inside the 10-minute window.
        let now = Utc::now().timestamp();
        let iat = now - 9 * 60;
        let events = events_consent_revoked("u", iat * 1000);
        let jwt = sign_envelope(envelope(iat, iat + 6 * 3600, &events));

        verify_for_test(&jwt, now).expect("9-minute-old iat must be accepted");
    }

    #[test]
    fn verify_notification_inner_double_decodes_stringified_events() {
        // Regression test for the most likely future-author footgun:
        // forgetting that `events` is a JSON *string*, not a nested
        // object. We feed a payload where `events` is a stringified
        // JSON object and assert the parser recovers the inner type
        // correctly; the negative side is covered by the next test.
        let now = Utc::now().timestamp();
        let events_str = serde_json::json!({
            "type": "consent-revoked",
            "sub": "stringified-events-test",
            "event_time": now * 1000,
        })
        .to_string();

        // Sanity: confirm the test fixture really is a string in the
        // outer payload, not a nested object — otherwise we'd be
        // testing the wrong thing.
        let outer = envelope(now, now + 600, &events_str);
        assert!(
            matches!(&outer["events"], serde_json::Value::String(_)),
            "test fixture must mirror Apple's stringified-events shape"
        );

        let jwt = sign_envelope(outer);
        let event = verify_for_test(&jwt, now).expect("verifies");
        assert_eq!(event.sub, "stringified-events-test");
    }

    #[test]
    fn verify_notification_inner_rejects_nested_object_events() {
        // The wrong shape — `events` as a JSON object literal — must
        // fail rather than silently being misinterpreted. Apple
        // doesn't send this format; the test fails the dev who
        // assumes "events is just JSON" without re-reading the spec.
        let now = Utc::now().timestamp();
        let env = serde_json::json!({
            "iss": APPLE_ISSUER,
            "aud": TEST_SERVICE_ID,
            "iat": now,
            "exp": now + 600,
            "events": {
                "type": "consent-revoked",
                "sub": "u",
                "event_time": now * 1000,
            },
        });
        let jwt = sign_envelope(env);

        let err = verify_for_test(&jwt, now).expect_err("nested events object must be rejected");
        assert!(matches!(err, OAuthError::NotificationVerification(_)));
    }

    #[test]
    fn deserialize_bool_or_string_rejects_unknown_string() {
        // Catches typos at the parser boundary rather than coercing
        // them to false (which would silently mislabel Hide-My-Email
        // toggles).
        #[derive(Deserialize)]
        struct Wrap {
            #[allow(dead_code)] // value read implicitly via the deserialize_with hook
            #[serde(deserialize_with = "super::super::apple::deserialize_bool_or_string")]
            flag: Option<bool>,
        }
        let parsed: Result<Wrap, _> = serde_json::from_str(r#"{"flag":"yes"}"#);
        assert!(parsed.is_err(), "non-true/false string must error");
    }

    #[test]
    fn unknown_kid_jwk_lookup_misses_in_test_set() {
        // The cache-miss path inside `resolve_jwk_decoding_key` is
        // covered by integration tests; here we just pin the shape
        // of `JwkSet::find` we depend on so a future jsonwebtoken
        // upgrade doesn't quietly change semantics underfoot.
        let jwks: jsonwebtoken::jwk::JwkSet = serde_json::from_value(serde_json::json!({
            "keys": [{
                "kty": "RSA",
                "use": "sig",
                "alg": "RS256",
                "kid": TEST_KID,
                "n": NOTIF_PUBLIC_N,
                "e": "AQAB",
            }]
        }))
        .unwrap();
        assert!(jwks.find("nope").is_none());
        assert!(jwks.find(TEST_KID).is_some());
    }
}
