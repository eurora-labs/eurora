use crate::client::AuthClient;
use crate::error::{AuthError, AuthResult};
use crate::events::AuthEvent;
use crate::secret_store::SecretStore;
use anyhow::Result;
use auth_core::{Claims, TokenResponse};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use euro_endpoint::EndpointManager;
use jsonwebtoken::dangerous::insecure_decode;
use rand::Rng;
use secrecy::{ExposeSecret, SecretString};
use sha2::{Digest, Sha256};
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};

/// How long before the access-token's `exp` we proactively refresh.
///
/// Algorithmic constant: tuned against the access-token TTL set in
/// `be_auth_core::JwtConfig` (currently 1h) and the round-trip cost
/// of a refresh. Not operator-tunable.
const REFRESH_OFFSET_MINUTES: i64 = 15;

#[derive(Debug, Clone, Copy)]
struct JwtConfig {
    refresh_offset_seconds: i64,
}

impl JwtConfig {
    const fn new() -> Self {
        Self {
            refresh_offset_seconds: REFRESH_OFFSET_MINUTES * 60,
        }
    }
}

/// Capacity of the [`AuthEvent`] broadcast channel. Auth transitions are
/// infrequent (a handful per session at most) and consumers — the cloud
/// settings sync engine, a frontend bridge — drain promptly, so a small
/// buffer absorbs any startup-time ordering without risking `Lagged`.
const AUTH_EVENT_CAPACITY: usize = 16;

/// Hint to UI callers describing how long the PKCE verifier slot is
/// considered valid. The backend enforces the actual expiry on the
/// challenge it stamps as OAuth `state`; this value just shapes the
/// "your link expires in N minutes" copy on the device. 20 minutes
/// matches the previous hard-coded value in the Tauri IPC layer.
const LOGIN_CHALLENGE_TTL_SECS: u32 = 60 * 20;

/// Returned by [`AuthManager::begin_login`]; consumed by the caller to
/// build the web sign-in URL.
///
/// Carries only the public PKCE challenge — the verifier stays inside
/// the secret store until [`AuthManager::complete_login`] redeems it.
#[derive(Debug, Clone)]
pub struct LoginChallenge {
    /// SHA-256 hash of the verifier, base64url-encoded. The backend
    /// stamps this as the OAuth `state` for the web sign-in flow.
    pub code_challenge: String,
    /// How long callers should treat this challenge as valid, in
    /// seconds. Advisory: see [`LOGIN_CHALLENGE_TTL_SECS`].
    pub expires_in: u32,
}

/// Centralised authentication state.
///
/// `AuthManager` is cheap to clone — all clones share the same inner state via
/// an `Arc`. In particular, they share a single refresh lock so that concurrent
/// callers coalesce into one server-side refresh: the winning task performs the
/// rotation, and queued callers observe the freshly stored access token after
/// the lock is released. This is critical because the backend invalidates a
/// refresh token on first use, so naive concurrent refreshes would cause all
/// but one caller to receive `InvalidToken` and log the user out.
///
/// The manager is the *sole* writer of session state: every method that
/// mutates the stored tokens (`login`, `register`, `login_by_*`,
/// `refresh_tokens`, `logout`, and the implicit refresh inside
/// [`get_or_refresh_access_token`]) publishes the resulting transition
/// on the internal [`AuthEvent`] bus before returning. Consumers observe
/// state by [`subscribe`]ing rather than by polling, which keeps the
/// in-process view and any frontend bridge in lock-step with the keyring.
#[derive(Debug, Clone)]
pub struct AuthManager(Arc<AuthManagerInner>);

impl Deref for AuthManager {
    type Target = AuthManagerInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub struct AuthManagerInner {
    auth_client: AuthClient,
    jwt_config: JwtConfig,
    refresh_lock: Mutex<()>,
    /// Encrypted file-backed store for session state. Owned here so
    /// the manager is the sole writer; external crates observe state
    /// through `AuthEvent` subscriptions, not by reaching into storage.
    secret_store: SecretStore,
    /// Backend event bus. Owning the `Sender` here keeps the channel
    /// alive for the manager's lifetime; subscribers obtain receivers
    /// via [`AuthManager::subscribe`]. The send path never blocks: a
    /// missing-receiver error is normal (nobody listening yet) and is
    /// dropped silently.
    auth_event_tx: broadcast::Sender<AuthEvent>,
}

impl AuthManager {
    /// Open the encrypted secret store under `data_dir` and build a
    /// manager around it. Fallible because the store can fail to
    /// initialise — usually that means the OS keychain is unavailable
    /// on a build where the main key is keychain-backed, and there's
    /// nothing the caller can do beyond surfacing it.
    pub fn new(endpoint_manager: Arc<EndpointManager>, data_dir: &Path) -> Result<Self, AuthError> {
        let secret_store = SecretStore::open(data_dir)?;
        let (auth_event_tx, _) = broadcast::channel(AUTH_EVENT_CAPACITY);
        Ok(Self(Arc::new(AuthManagerInner {
            auth_client: AuthClient::new(endpoint_manager),
            jwt_config: JwtConfig::new(),
            refresh_lock: Mutex::new(()),
            secret_store,
            auth_event_tx,
        })))
    }

    /// Subscribe to auth-state transitions.
    ///
    /// Each subscriber sees every transition published after it
    /// subscribes; missed events (slow consumer) surface as
    /// `RecvError::Lagged`, which subscribers should log and continue
    /// past — auth state is observed via `claims.sub`, not derived from
    /// a chain of deltas, so a dropped event is recoverable on the next
    /// one.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<AuthEvent> {
        self.0.auth_event_tx.subscribe()
    }

    /// Publish an auth-state transition to the bus.
    ///
    /// Internal — the manager itself publishes after every successful
    /// mutation. External callers observe transitions through
    /// [`subscribe`] rather than driving the bus directly.
    pub(crate) fn publish_auth_event(&self, event: AuthEvent) {
        // `send` errors only when no receiver is alive — which is the
        // normal case before any subscriber has connected and is
        // benign. Subscribers that genuinely fall behind surface
        // through `Lagged` on `recv`, not through this call.
        let _ = self.0.auth_event_tx.send(event);
    }

    /// Sign in with email + password. Returns the access-token
    /// [`Claims`] on success; the new session is already persisted and
    /// published on the bus by the time this resolves.
    pub async fn login(
        &self,
        login: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<Claims> {
        let response = self.auth_client.login_by_password(login, password).await?;
        self.complete_session(response)
    }

    /// Register a new account. Returns the access-token [`Claims`]
    /// on success; the new session is already persisted and published
    /// on the bus by the time this resolves.
    pub async fn register(
        &self,
        email: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<Claims> {
        let response = self.auth_client.register(email, password, None).await?;
        self.complete_session(response)
    }

    /// Exchange a login-token (web-flow PKCE verifier) for a session.
    pub async fn login_by_login_token(&self, login_token: String) -> Result<Claims> {
        let response = self.auth_client.login_by_login_token(login_token).await?;
        self.complete_session(response)
    }

    /// Trade a Google ID token (from the native iOS / Android SDKs) for
    /// a session.
    pub async fn login_by_google_id_token(
        &self,
        id_token: impl Into<String>,
        nonce: Option<String>,
    ) -> Result<Claims> {
        let response = self
            .auth_client
            .login_by_google_id_token(id_token, nonce)
            .await?;
        self.complete_session(response)
    }

    /// Trade an Apple ID token (from `ASAuthorizationController` on
    /// iOS) for a session. `raw_nonce` is the unhashed nonce the
    /// plugin generated; the backend re-derives the hash to match
    /// against the ID token's `nonce` claim. `user` is the
    /// `fullName` Apple ships on the first sign-in only.
    pub async fn login_by_apple_id_token(
        &self,
        id_token: impl Into<String>,
        raw_nonce: impl Into<String>,
        user: Option<auth_core::AppleNativeUser>,
    ) -> Result<Claims> {
        let response = self
            .auth_client
            .login_by_apple_id_token(id_token, raw_nonce, user)
            .await?;
        self.complete_session(response)
    }

    /// Tear down the current session.
    ///
    /// Best-effort: the server-side refresh-token revocation and the
    /// local secret-store wipe are independent steps. If the server is
    /// unreachable we still clear local credentials — otherwise the user
    /// would be wedged in a "kinda logged out" state with stale tokens
    /// on disk. Either step's failure is logged but never propagated,
    /// since the user's intent ("I'm signed out") is honoured by the
    /// `AuthEvent { claims: None }` that always fires at the end.
    pub async fn logout(&self) {
        if let Some(refresh_token) = self.refresh_token_silent()
            && let Err(err) = self.auth_client.logout(refresh_token.expose_secret()).await
        {
            tracing::warn!(
                error = %err,
                "server-side logout failed; clearing local credentials anyway"
            );
        }
        if let Err(err) = self.secret_store.wipe() {
            tracing::warn!(error = %err, "failed to wipe secret store on logout");
        }
        self.publish_auth_event(AuthEvent { claims: None });
    }

    /// Decode the current access-token's claims without verifying the
    /// signature (the issuer's public key isn't available client-side;
    /// the token has already been validated by the server when it
    /// minted it). Returns an error when no token is stored or when the
    /// stored token isn't a parseable JWT.
    pub fn get_access_token_payload(&self) -> Result<Claims> {
        let token = self
            .secret_store
            .access_token()?
            .ok_or_else(|| anyhow::anyhow!("no access token stored"))?;
        let token = insecure_decode::<Claims>(token.expose_secret())?;
        Ok(token.claims)
    }

    /// Convenience for "what's the current session, if any". Returns
    /// `None` for both the no-token and malformed-token cases — callers
    /// uniformly treat both as "not signed in".
    #[must_use]
    pub fn current_claims(&self) -> Option<Claims> {
        self.get_access_token_payload().ok()
    }

    pub fn get_refresh_token_payload(&self) -> Result<Claims> {
        let token = self
            .secret_store
            .refresh_token()?
            .ok_or_else(|| anyhow::anyhow!("no refresh token stored"))?;
        let token = insecure_decode::<Claims>(token.expose_secret())?;
        Ok(token.claims)
    }

    /// Returns a valid access token, refreshing from the server only if the
    /// stored token is missing or within the refresh-offset window of expiry.
    ///
    /// Callers can inspect the returned [`AuthError`] to distinguish a true
    /// "logged out" state (refresh token rejected by the server) from a
    /// transient failure (server unreachable, etc.); see [`AuthError::is_logged_out`]
    /// and [`AuthError::is_transient`].
    pub async fn get_or_refresh_access_token(&self) -> AuthResult<SecretString> {
        if self.has_fresh_access_token() {
            return self.read_access_token();
        }
        self.ensure_refresh().await?;
        self.read_access_token()
    }

    /// Force a refresh, coalescing with any concurrent refresh already in
    /// flight. If another task completes a refresh while this one is waiting
    /// for the lock, the freshly stored token's claims are returned without
    /// a second round-trip to the server.
    pub async fn refresh_tokens(&self) -> AuthResult<Claims> {
        self.ensure_refresh().await
    }

    async fn ensure_refresh(&self) -> AuthResult<Claims> {
        let _guard = self.refresh_lock.lock().await;

        // Double-checked: another task may have refreshed while we
        // waited. In that case the bus already fired for the
        // refresh — we just observe the freshly stored claims.
        if self.has_fresh_access_token() {
            return self
                .get_access_token_payload()
                .map_err(|_| AuthError::MissingAccessToken);
        }

        self.perform_refresh().await
    }

    async fn perform_refresh(&self) -> AuthResult<Claims> {
        let refresh_token = self
            .secret_store
            .refresh_token()?
            .ok_or(AuthError::MissingRefreshToken)?;
        let response = self
            .auth_client
            .refresh_token(refresh_token.expose_secret())
            .await?;

        self.complete_session(response)
            .map_err(AuthError::Transient)
    }

    fn complete_session(&self, response: TokenResponse) -> Result<Claims> {
        self.secret_store
            .set_access_token(SecretString::from(response.access_token))?;
        self.secret_store
            .set_refresh_token(SecretString::from(response.refresh_token))?;
        let claims = self.get_access_token_payload()?;
        self.publish_auth_event(AuthEvent {
            claims: Some(claims.clone()),
        });
        Ok(claims)
    }

    fn read_access_token(&self) -> AuthResult<SecretString> {
        self.secret_store
            .access_token()?
            .ok_or(AuthError::MissingAccessToken)
    }

    /// Like [`secret_store::SecretStore::refresh_token`] but collapses
    /// I/O errors into `None`. Used by [`AuthManager::logout`], where a
    /// failing read shouldn't block local credential cleanup.
    fn refresh_token_silent(&self) -> Option<SecretString> {
        match self.secret_store.refresh_token() {
            Ok(token) => token,
            Err(err) => {
                tracing::warn!(
                    error = %err,
                    "failed to read refresh token from secret store"
                );
                None
            }
        }
    }

    fn has_fresh_access_token(&self) -> bool {
        let Ok(claims) = self.get_access_token_payload() else {
            return false;
        };
        let now = chrono::Utc::now().timestamp();
        now < claims
            .exp
            .saturating_sub(self.jwt_config.refresh_offset_seconds)
    }

    /// Generate a fresh PKCE `(verifier, challenge)` pair.
    ///
    /// Pure function — does not persist anything. Use this for flows
    /// where the verifier lives only in the awaiting frame and is
    /// handed straight to [`AuthManager::login_by_login_token`] (e.g.
    /// the mobile in-app browser session, where the redirect resolves
    /// synchronously and the verifier never needs to outlive the
    /// process). For the desktop flow — where the browser hand-off
    /// crosses a process boundary and the verifier must survive — use
    /// [`AuthManager::begin_login`] / [`AuthManager::complete_login`].
    #[must_use]
    pub fn generate_pkce_pair(&self) -> (String, String) {
        let mut verifier_bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut verifier_bytes);
        let code_verifier = URL_SAFE_NO_PAD.encode(verifier_bytes);

        let mut hasher = Sha256::new();
        hasher.update(&code_verifier);
        let code_challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());

        (code_verifier, code_challenge)
    }

    /// Start a desktop-style PKCE login.
    ///
    /// Generates a fresh PKCE pair, persists the verifier in the
    /// secret store, and returns the corresponding challenge. The
    /// caller hands the challenge to the web sign-in page; once the
    /// user completes the flow there, [`AuthManager::complete_login`]
    /// redeems the stored verifier for a session.
    ///
    /// Calling `begin_login` while a previous challenge is still
    /// outstanding overwrites it — only one login attempt can be in
    /// flight at a time. That matches the UX: the verifier slot is
    /// the device's single "I'm trying to sign in right now" hand.
    pub fn begin_login(&self) -> AuthResult<LoginChallenge> {
        let (verifier, challenge) = self.generate_pkce_pair();
        self.secret_store
            .set_pkce_verifier(SecretString::from(verifier))?;
        Ok(LoginChallenge {
            code_challenge: challenge,
            expires_in: LOGIN_CHALLENGE_TTL_SECS,
        })
    }

    /// Complete a desktop-style PKCE login.
    ///
    /// Reads the verifier persisted by [`AuthManager::begin_login`],
    /// exchanges it for a session, and clears the verifier on success.
    /// On failure the verifier is left in place so the caller can
    /// retry without re-running `begin_login`.
    ///
    /// Returns [`AuthError::LoginChallengeExpired`] when there's no
    /// stored verifier — either the user never started a login on
    /// this device or the slot was already consumed.
    pub async fn complete_login(&self) -> AuthResult<Claims> {
        let verifier = self
            .secret_store
            .pkce_verifier()?
            .ok_or(AuthError::LoginChallengeExpired)?;
        let claims = self
            .login_by_login_token(verifier.expose_secret().to_owned())
            .await?;
        self.secret_store.clear_pkce_verifier()?;
        Ok(claims)
    }

    pub async fn resend_verification_email(&self) -> Result<()> {
        let access_token = self
            .secret_store
            .access_token()?
            .ok_or_else(|| anyhow::anyhow!("no access token stored"))?;
        self.auth_client
            .resend_verification_email(access_token.expose_secret())
            .await
            .map_err(anyhow::Error::from)
    }

    /// Build a provider-authorisation URL for the mobile in-app browser
    /// flow. The backend stamps the supplied `code_challenge` as the
    /// OAuth `state`, so the same value identifies the device when the
    /// callback fires.
    pub async fn mobile_third_party_auth_url(
        &self,
        provider: auth_core::Provider,
        code_challenge: impl Into<String>,
    ) -> Result<String> {
        let response = self
            .auth_client
            .mobile_third_party_auth_url(provider, code_challenge)
            .await?;
        Ok(response.url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use auth_core::Role;
    use euro_endpoint::EndpointManager;

    fn manager_for_test() -> (AuthManager, tempfile::TempDir) {
        let endpoint =
            Arc::new(EndpointManager::new("https://example.invalid").expect("valid endpoint"));
        let data_dir = tempfile::tempdir().expect("temp dir");
        let manager = AuthManager::new(endpoint, data_dir.path()).expect("construct AuthManager");
        (manager, data_dir)
    }

    fn claims_for(sub: &str) -> Claims {
        Claims {
            sub: sub.to_owned(),
            email: "test@example.com".to_owned(),
            display_name: None,
            exp: 0,
            iat: 0,
            token_type: "access".to_owned(),
            role: Role::Free,
            aud: String::new(),
            email_verified: true,
            jti: String::new(),
        }
    }

    #[tokio::test]
    async fn publish_auth_event_delivers_to_subscriber() {
        let (manager, _dir) = manager_for_test();
        let mut rx = manager.subscribe();

        manager.publish_auth_event(AuthEvent {
            claims: Some(claims_for("00000000-0000-4000-8000-000000000001")),
        });
        let event = rx.recv().await.expect("event received");
        assert!(event.claims.is_some());
        assert_eq!(
            event.claims.unwrap().sub,
            "00000000-0000-4000-8000-000000000001"
        );

        manager.publish_auth_event(AuthEvent { claims: None });
        let event = rx.recv().await.expect("logout event received");
        assert!(event.claims.is_none());
    }

    #[tokio::test]
    async fn publish_with_no_subscriber_is_a_no_op() {
        // The bus must tolerate publishes that have no listener — at
        // startup the manager may publish before any sync engine has
        // subscribed, and that path must not return an error or panic.
        let (manager, _dir) = manager_for_test();
        manager.publish_auth_event(AuthEvent { claims: None });
    }

    #[tokio::test]
    async fn subscribers_share_an_independent_event_history() {
        // `broadcast` channels start each subscriber at the current
        // tail, so an event published before `subscribe()` is invisible
        // to that subscriber. Two subscribers established at different
        // points must observe different prefixes of the stream.
        let (manager, _dir) = manager_for_test();
        manager.publish_auth_event(AuthEvent {
            claims: Some(claims_for("11111111-1111-4111-8111-111111111111")),
        });
        let mut rx = manager.subscribe();

        let try_recv = rx.try_recv();
        assert!(
            matches!(try_recv, Err(broadcast::error::TryRecvError::Empty)),
            "subscriber that joined after the publish must not see it; got {try_recv:?}"
        );

        manager.publish_auth_event(AuthEvent {
            claims: Some(claims_for("22222222-2222-4222-8222-222222222222")),
        });
        let event = rx.recv().await.expect("post-subscribe event received");
        assert_eq!(
            event.claims.unwrap().sub,
            "22222222-2222-4222-8222-222222222222"
        );
    }

    #[tokio::test]
    async fn current_claims_returns_none_without_a_session() {
        // Fresh secret store has no access token; the convenience
        // accessor must collapse "missing" and "malformed" into a
        // single `None` so callers don't need to branch.
        let (manager, _dir) = manager_for_test();
        assert!(manager.current_claims().is_none());
    }

    #[tokio::test]
    async fn begin_login_persists_verifier_and_complete_login_consumes_it() {
        let (manager, _dir) = manager_for_test();

        // No challenge in flight → `complete_login` reports expiry
        // rather than firing a network call.
        let err = manager
            .complete_login()
            .await
            .expect_err("complete_login without begin_login");
        assert!(matches!(err, AuthError::LoginChallengeExpired));

        let challenge = manager.begin_login().expect("begin_login");
        assert!(!challenge.code_challenge.is_empty());
        assert_eq!(challenge.expires_in, LOGIN_CHALLENGE_TTL_SECS);

        // Round-tripping `begin_login` overwrites the previous verifier
        // — only one in-flight challenge per device.
        let next = manager.begin_login().expect("second begin_login");
        assert_ne!(next.code_challenge, challenge.code_challenge);
    }

    #[test]
    fn generate_pkce_pair_emits_distinct_random_pairs() {
        let (manager, _dir) = manager_for_test();
        let (v1, c1) = manager.generate_pkce_pair();
        let (v2, c2) = manager.generate_pkce_pair();
        assert_ne!(v1, v2);
        assert_ne!(c1, c2);
        // S256 challenges are base64url-encoded SHA-256 digests, which
        // are always 43 chars at no-pad.
        assert_eq!(c1.len(), 43);
        assert_eq!(c2.len(), 43);
    }
}
