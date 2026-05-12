use crate::client::AuthClient;
use crate::error::{AuthError, AuthResult};
use crate::events::AuthEvent;
use anyhow::{Result, anyhow};
use auth_core::Claims;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use euro_endpoint::EndpointManager;
use euro_secret::{ExposeSecret, SecretString, secret};
use jsonwebtoken::dangerous::insecure_decode;
use rand::Rng;
use sha2::{Digest, Sha256};
use std::ops::Deref;
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

pub const ACCESS_TOKEN_HANDLE: &str = "AUTH_ACCESS_TOKEN";
pub const REFRESH_TOKEN_HANDLE: &str = "AUTH_REFRESH_TOKEN";

/// Capacity of the [`AuthEvent`] broadcast channel. Auth transitions are
/// infrequent (a handful per session at most) and consumers — the cloud
/// settings sync engine, a frontend bridge — drain promptly, so a small
/// buffer absorbs any startup-time ordering without risking `Lagged`.
const AUTH_EVENT_CAPACITY: usize = 16;

/// Shared authentication state.
///
/// `AuthManager` is cheap to clone — all clones share the same inner state via
/// an `Arc`. In particular, they share a single refresh lock so that concurrent
/// callers coalesce into one server-side refresh: the winning task performs the
/// rotation, and queued callers observe the freshly stored access token after
/// the lock is released. This is critical because the backend invalidates a
/// refresh token on first use, so naive concurrent refreshes would cause all
/// but one caller to receive `InvalidToken` and log the user out.
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
    /// Backend event bus. Owning the `Sender` here keeps the channel
    /// alive for the manager's lifetime; subscribers obtain receivers
    /// via [`AuthManager::subscribe`]. The send path never blocks: a
    /// missing-receiver error is normal (nobody listening yet) and is
    /// dropped silently.
    auth_event_tx: broadcast::Sender<AuthEvent>,
}

impl AuthManager {
    pub fn new(endpoint_manager: Arc<EndpointManager>) -> Self {
        let (auth_event_tx, _) = broadcast::channel(AUTH_EVENT_CAPACITY);
        Self(Arc::new(AuthManagerInner {
            auth_client: AuthClient::new(endpoint_manager),
            jwt_config: JwtConfig::new(),
            refresh_lock: Mutex::new(()),
            auth_event_tx,
        }))
    }

    /// Subscribe to backend auth-state transitions.
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

    /// Publish an auth-state transition to the backend bus.
    ///
    /// Callers are the IPC procedures that already emit the Tauri-side
    /// `AuthStateChanged` event; this is the parallel publish so
    /// backend subscribers (sync engine, future observability hooks)
    /// observe the same boundaries without going through the frontend.
    pub fn publish_auth_event(&self, event: AuthEvent) {
        // `send` errors only when no receiver is alive — which is the
        // normal case before any subscriber has connected and is
        // benign. Subscribers that genuinely fall behind surface
        // through `Lagged` on `recv`, not through this call.
        let _ = self.0.auth_event_tx.send(event);
    }

    pub async fn login(
        &self,
        login: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<SecretString> {
        let response = self.auth_client.login_by_password(login, password).await?;

        store_access_token(response.access_token.clone())?;
        store_refresh_token(response.refresh_token.clone())?;

        Ok(SecretString::from(response.access_token))
    }

    pub async fn register(
        &self,
        email: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<SecretString> {
        let response = self.auth_client.register(email, password, None).await?;

        store_access_token(response.access_token.clone())?;
        store_refresh_token(response.refresh_token.clone())?;

        Ok(SecretString::from(response.access_token))
    }

    fn get_access_token(&self) -> Result<SecretString> {
        secret::retrieve(ACCESS_TOKEN_HANDLE)?.ok_or_else(|| anyhow!("No access token found"))
    }

    fn get_refresh_token(&self) -> Result<SecretString> {
        secret::retrieve(REFRESH_TOKEN_HANDLE)?.ok_or_else(|| anyhow!("No refresh token found"))
    }

    pub fn get_access_token_payload(&self) -> Result<Claims> {
        let token = self.get_access_token()?;
        let token = insecure_decode::<Claims>(token.expose_secret())?;
        Ok(token.claims)
    }

    pub fn get_refresh_token_payload(&self) -> Result<Claims> {
        let token = self.get_refresh_token()?;
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
        self.ensure_refresh().await
    }

    /// Force a refresh, coalescing with any concurrent refresh already in
    /// flight. If another task completes a refresh while this one is waiting
    /// for the lock, the freshly stored token is returned without a second
    /// round-trip to the server.
    pub async fn refresh_tokens(&self) -> AuthResult<SecretString> {
        self.ensure_refresh().await
    }

    async fn ensure_refresh(&self) -> AuthResult<SecretString> {
        let _guard = self.refresh_lock.lock().await;

        // Double-checked: another task may have refreshed while we waited.
        if self.has_fresh_access_token() {
            return self.read_access_token();
        }

        self.perform_refresh().await?;
        self.read_access_token()
    }

    async fn perform_refresh(&self) -> AuthResult<()> {
        let refresh_token = self
            .get_refresh_token()
            .map_err(|_| AuthError::MissingRefreshToken)?;
        let response = self
            .auth_client
            .refresh_token(refresh_token.expose_secret())
            .await?;

        store_access_token(response.access_token)?;
        store_refresh_token(response.refresh_token)?;
        Ok(())
    }

    fn read_access_token(&self) -> AuthResult<SecretString> {
        self.get_access_token()
            .map_err(|_| AuthError::MissingAccessToken)
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

    pub async fn get_login_tokens(&self) -> Result<(String, String)> {
        let mut verifier_bytes = vec![0u8; 32];
        rand::rng().fill_bytes(&mut verifier_bytes);

        let code_verifier = URL_SAFE_NO_PAD.encode(verifier_bytes);
        let mut hasher = Sha256::new();
        hasher.update(&code_verifier);
        let code_challenge_raw = hasher.finalize();
        let code_challenge = URL_SAFE_NO_PAD.encode(code_challenge_raw);

        Ok((code_verifier, code_challenge))
    }

    pub async fn resend_verification_email(&self) -> Result<()> {
        let access_token = self.get_access_token()?;
        self.auth_client
            .resend_verification_email(access_token.expose_secret())
            .await
            .map_err(anyhow::Error::from)
    }

    pub async fn login_by_login_token(&self, login_token: String) -> Result<SecretString> {
        let response = self.auth_client.login_by_login_token(login_token).await?;

        store_access_token(response.access_token.clone())?;
        store_refresh_token(response.refresh_token.clone())?;

        Ok(SecretString::from(response.access_token))
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

    /// Trade a Google ID token (from the native iOS / Android SDKs) for
    /// a session.
    pub async fn login_by_google_id_token(
        &self,
        id_token: impl Into<String>,
        nonce: Option<String>,
    ) -> Result<SecretString> {
        let response = self
            .auth_client
            .login_by_google_id_token(id_token, nonce)
            .await?;

        store_access_token(response.access_token.clone())?;
        store_refresh_token(response.refresh_token.clone())?;

        Ok(SecretString::from(response.access_token))
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
    ) -> Result<SecretString> {
        let response = self
            .auth_client
            .login_by_apple_id_token(id_token, raw_nonce, user)
            .await?;

        store_access_token(response.access_token.clone())?;
        store_refresh_token(response.refresh_token.clone())?;

        Ok(SecretString::from(response.access_token))
    }
}

fn store_access_token(token: String) -> Result<()> {
    secret::persist(ACCESS_TOKEN_HANDLE, &SecretString::from(token))
        .map_err(|e| anyhow!("Failed to store access token: {}", e))
}

fn store_refresh_token(token: String) -> Result<()> {
    secret::persist(REFRESH_TOKEN_HANDLE, &SecretString::from(token))
        .map_err(|e| anyhow!("Failed to store refresh token: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use auth_core::Role;
    use euro_endpoint::EndpointManager;

    fn manager_for_test() -> AuthManager {
        let endpoint =
            Arc::new(EndpointManager::new("https://example.invalid").expect("valid endpoint"));
        AuthManager::new(endpoint)
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
        let manager = manager_for_test();
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
        // startup the procedures publish before any sync engine has
        // subscribed, and that path must not return an error or panic.
        let manager = manager_for_test();
        manager.publish_auth_event(AuthEvent { claims: None });
    }

    #[tokio::test]
    async fn subscribers_share_an_independent_event_history() {
        // `broadcast` channels start each subscriber at the current
        // tail, so an event published before `subscribe()` is invisible
        // to that subscriber. Two subscribers established at different
        // points must observe different prefixes of the stream.
        let manager = manager_for_test();
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
}
