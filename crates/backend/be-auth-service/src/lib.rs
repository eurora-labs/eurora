//! HTTP authentication service.
//!
//! Exposes an Axum router under `/auth` that handles email+password and
//! third-party (Google, GitHub) authentication, refresh-token rotation,
//! email-verification, and the device-pairing login-token flow.
//!
//! Unlike the activity / asset services, the global `authz_middleware`
//! bypasses the `/auth/*` prefix entirely so unauthenticated callers can
//! reach login / register / refresh. Routes that *do* require a token
//! validate it inline via the [`auth::AccessClaims`] / [`auth::RefreshClaims`]
//! extractors using the shared [`be_auth_core::JwtConfig`].

pub mod auth;
pub mod crypto;
pub mod error;
pub mod handlers;
pub mod oauth;
pub mod service;

use std::sync::Arc;

use anyhow::Result;
use auth_core::{Provider, TokenResponse};
use axum::{Router, routing::post};
use be_auth_core::JwtConfig;
use be_email_service::EmailService;
use be_remote_db::{DatabaseManager, DbError, OAuthProvider};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{Algorithm, Header, encode};
use openidconnect::{Nonce, PkceCodeChallenge, PkceCodeVerifier};
use rand::Rng;
use sha2::{Digest, Sha256};
use tokio::sync::OnceCell;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};

pub use auth_core::{Claims, Role};
pub use error::{AuthError, AuthResult};
pub use service::AppState;

use crate::crypto::{decrypt_sensitive_string, encrypt_sensitive_string};
use crate::oauth::github::GitHubOAuthClient;
use crate::oauth::google::GoogleOAuthClient;
use secrecy::ExposeSecret;

const MIN_PASSWORD_LENGTH: usize = 8;
const MAX_PASSWORD_LENGTH: usize = 128;
const LOGIN_TOKEN_EXPIRY_MINUTES: i64 = 20;
const OAUTH_STATE_EXPIRY_MINUTES: i64 = 10;
const VERIFICATION_TOKEN_EXPIRY_HOURS: i64 = 24;
const VERIFICATION_RESEND_COOLDOWN_SECONDS: i64 = 60;

/// PostgreSQL auto-generates this name for the `UNIQUE` constraint on
/// `users.email` (format: `<table>_<column>_key`). If the column or
/// constraint is ever renamed, this constant must be updated to match.
const USERS_EMAIL_UNIQUE_CONSTRAINT: &str = "users_email_key";

/// OAuth identity tokens, already encrypted at rest.
pub struct OAuthTokenBundle {
    pub encrypted_access_token: Vec<u8>,
    pub encrypted_refresh_token: Option<Vec<u8>>,
    pub access_token_expiry: Option<DateTime<Utc>>,
    pub scope: String,
}

/// A freshly authenticated identity returned by an OAuth provider.
///
/// The service either (a) finds an existing user via
/// `(provider, provider_user_id)` and refreshes their tokens, or
/// (b) creates a brand-new user. It never silently links to an existing
/// account on email match — that path is a known account-takeover vector
/// and is explicitly rejected with [`AuthError::OAuthEmailConflict`].
pub struct NewOAuthIdentity {
    pub provider: OAuthProvider,
    pub provider_user_id: String,
    pub email: String,
    pub email_verified: bool,
    pub name: String,
    pub tokens: OAuthTokenBundle,
}

pub struct AuthService {
    db: Arc<DatabaseManager>,
    jwt_config: JwtConfig,
    email_service: Option<Arc<EmailService>>,
    google_oauth_client: OnceCell<GoogleOAuthClient>,
    github_oauth_client: std::sync::OnceLock<GitHubOAuthClient>,
}

impl AuthService {
    pub fn new(
        db: Arc<DatabaseManager>,
        jwt_config: JwtConfig,
        email_service: Option<Arc<EmailService>>,
    ) -> Self {
        tracing::info!("Creating new AuthService instance");
        Self {
            db,
            jwt_config,
            email_service,
            google_oauth_client: OnceCell::new(),
            github_oauth_client: std::sync::OnceLock::new(),
        }
    }

    async fn google_oauth_client(&self) -> AuthResult<&GoogleOAuthClient> {
        self.google_oauth_client
            .get_or_try_init(|| async {
                let config = oauth::google::GoogleOAuthConfig::from_env()?;
                Ok(GoogleOAuthClient::discover(config).await?)
            })
            .await
    }

    fn github_oauth_client(&self) -> AuthResult<&GitHubOAuthClient> {
        if let Some(client) = self.github_oauth_client.get() {
            return Ok(client);
        }
        let config = oauth::github::GitHubOAuthConfig::from_env()?;
        let client = GitHubOAuthClient::new(config);
        Ok(self.github_oauth_client.get_or_init(|| client))
    }

    fn validate_password(password: &str) -> AuthResult<()> {
        if password.len() < MIN_PASSWORD_LENGTH {
            return Err(AuthError::InvalidInput(format!(
                "Password must be at least {MIN_PASSWORD_LENGTH} characters"
            )));
        }
        if password.len() > MAX_PASSWORD_LENGTH {
            return Err(AuthError::InvalidInput(format!(
                "Password must be at most {MAX_PASSWORD_LENGTH} characters"
            )));
        }
        Ok(())
    }

    fn validate_email(email: &str) -> AuthResult<()> {
        let at_pos = email.find('@');
        let valid = match at_pos {
            Some(pos) => {
                let local = &email[..pos];
                let domain = &email[pos + 1..];
                !local.is_empty() && domain.contains('.') && domain.len() >= 3
            }
            None => false,
        };
        if !valid {
            return Err(AuthError::InvalidInput(
                "Invalid email address format".into(),
            ));
        }
        Ok(())
    }

    fn hash_password(&self, password: &str) -> AuthResult<String> {
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| AuthError::PasswordHash(e.to_string()))?;
        Ok(hash.to_string())
    }

    fn hash_refresh_token(&self, token: &str) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        hasher.finalize().to_vec()
    }

    fn is_approved_email(&self, email: &str) -> bool {
        let email = email.to_lowercase();
        self.jwt_config.approved_emails.contains(&email)
    }

    async fn resolve_role(&self, user_id: Uuid) -> Role {
        let local_mode = std::env::var("RUNNING_EURORA_FULLY_LOCAL")
            .map(|v| v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        if local_mode {
            return Role::Tier1;
        }

        match self.db.get_plan_id_for_user().user_id(user_id).call().await {
            Ok(Some(ref plan)) if plan == "tier1" => Role::Tier1,
            _ => Role::Free,
        }
    }

    async fn ensure_plan_and_resolve_role(&self, user_id: Uuid, email: &str) -> AuthResult<Role> {
        let plan_id = if self.is_approved_email(email) {
            "tier1"
        } else {
            "free"
        };

        self.db
            .ensure_user_plan()
            .executor(&self.db.pool)
            .user_id(user_id)
            .plan_id(plan_id)
            .call()
            .await?;

        Ok(self.resolve_role(user_id).await)
    }

    fn generate_jwt_tokens(
        &self,
        user_id: &str,
        email: &str,
        display_name: Option<String>,
        role: Role,
        email_verified: bool,
    ) -> AuthResult<(String, String, Vec<u8>, DateTime<Utc>)> {
        let now = Utc::now();
        let access_exp = now + Duration::hours(self.jwt_config.access_token_expiry_hours);
        let refresh_exp = now + Duration::days(self.jwt_config.refresh_token_expiry_days);

        let access_claims = Claims {
            sub: user_id.to_string(),
            email: email.to_string(),
            display_name: display_name.clone(),
            exp: access_exp.timestamp(),
            iat: now.timestamp(),
            token_type: "access".to_string(),
            role: role.clone(),
            aud: "eurora".to_string(),
            email_verified,
            jti: Uuid::now_v7().to_string(),
        };

        let refresh_claims = Claims {
            sub: user_id.to_string(),
            email: email.to_string(),
            display_name,
            exp: refresh_exp.timestamp(),
            iat: now.timestamp(),
            token_type: "refresh".to_string(),
            role,
            aud: "eurora".to_string(),
            email_verified,
            jti: Uuid::now_v7().to_string(),
        };

        let header = Header::new(Algorithm::HS256);

        let access_token = encode(
            &header,
            &access_claims,
            &self.jwt_config.access_token_encoding_key,
        )
        .map_err(|e| AuthError::TokenGeneration(e.to_string()))?;

        let refresh_token = encode(
            &header,
            &refresh_claims,
            &self.jwt_config.refresh_token_encoding_key,
        )
        .map_err(|e| AuthError::TokenGeneration(e.to_string()))?;

        let token_hash = self.hash_refresh_token(&refresh_token);

        Ok((access_token, refresh_token, token_hash, refresh_exp))
    }

    async fn generate_tokens(
        &self,
        user_id: &str,
        email: &str,
        display_name: Option<String>,
        role: Role,
        email_verified: bool,
    ) -> AuthResult<(String, String)> {
        let (access_token, refresh_token, token_hash, refresh_exp) =
            self.generate_jwt_tokens(user_id, email, display_name, role, email_verified)?;

        let user_uuid = Uuid::parse_str(user_id)
            .map_err(|e| AuthError::Internal(format!("Invalid user ID format: {e}")))?;

        self.db
            .create_refresh_token()
            .user_id(user_uuid)
            .token_hash(token_hash)
            .expires_at(refresh_exp)
            .call()
            .await?;

        Ok((access_token, refresh_token))
    }

    fn generate_random_string(&self, length: usize) -> AuthResult<String> {
        let byte_len = length.div_ceil(2);
        let mut bytes = vec![0u8; byte_len];
        rand::rng().fill_bytes(&mut bytes);

        let mut hex = hex::encode(bytes);
        hex.truncate(length);
        Ok(hex)
    }

    async fn try_associate_login_token_with_user(
        &self,
        user: &be_remote_db::User,
        code_challenge: &str,
    ) {
        let token_hash = self.hash_login_token(code_challenge);

        match self
            .db
            .create_login_token()
            .token_hash(token_hash)
            .user_id(user.id)
            .expires_at(Utc::now() + Duration::minutes(LOGIN_TOKEN_EXPIRY_MINUTES))
            .call()
            .await
        {
            Ok(_) => {
                tracing::info!(
                    "Successfully associated login token with user: {}",
                    user.email
                );
            }
            Err(e) => {
                tracing::error!("Failed to update login token with user_id: {}", e);
            }
        }
    }

    fn hash_login_token(&self, token: &str) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        hasher.finalize().to_vec()
    }

    fn code_verifier_to_challenge(&self, code_verifier: &str) -> String {
        let verifier = PkceCodeVerifier::new(code_verifier.to_string());
        let challenge = PkceCodeChallenge::from_code_verifier_sha256(&verifier);
        challenge.as_str().to_string()
    }

    /// Resolve the user for an OAuth-provided identity.
    ///
    /// Returns the matching user if one exists for this
    /// `(provider, provider_user_id)` pair (refreshing stored tokens as a
    /// side-effect), or creates a brand-new user.
    ///
    /// Returns [`AuthError::OAuthEmailConflict`] if the email belongs to an
    /// existing account under a different identity (password credentials or
    /// another OAuth provider). Linking new providers to an existing account
    /// must go through an explicit, authenticated flow — not through an
    /// anonymous OAuth callback. Detection is race-free: the conflict surfaces
    /// from the `users.email` unique-index violation at insert time.
    async fn resolve_oauth_user(
        &self,
        identity: NewOAuthIdentity,
    ) -> AuthResult<be_remote_db::User> {
        if let Some(user) = self
            .lookup_existing_oauth_identity(identity.provider, &identity.provider_user_id)
            .await?
        {
            self.refresh_oauth_credentials(user.id, identity.provider, identity.tokens)
                .await;
            return Ok(user);
        }

        self.create_oauth_user(identity).await
    }

    async fn lookup_existing_oauth_identity(
        &self,
        provider: OAuthProvider,
        provider_user_id: &str,
    ) -> AuthResult<Option<be_remote_db::User>> {
        match self
            .db
            .get_user_by_oauth_provider()
            .provider(provider)
            .provider_user_id(provider_user_id)
            .call()
            .await
        {
            Ok(user) => Ok(Some(user)),
            Err(e) if e.is_not_found() => Ok(None),
            Err(e) => Err(AuthError::Database(e)),
        }
    }

    /// Rotate stored OAuth credentials in-place. Failures are logged but not
    /// propagated — the user is already authenticated via the provider and a
    /// transient credential-update failure must not break login.
    async fn refresh_oauth_credentials(
        &self,
        user_id: Uuid,
        provider: OAuthProvider,
        tokens: OAuthTokenBundle,
    ) {
        let oauth_creds = match self
            .db
            .get_oauth_credentials_by_provider_and_user()
            .provider(provider)
            .user_id(user_id)
            .call()
            .await
        {
            Ok(creds) => creds,
            Err(e) => {
                tracing::warn!(
                    %user_id,
                    ?provider,
                    error = %e,
                    "Failed to locate OAuth credentials for refresh"
                );
                return;
            }
        };

        if let Err(e) = self
            .db
            .update_oauth_credentials()
            .id(oauth_creds.id)
            .access_token(tokens.encrypted_access_token)
            .maybe_refresh_token(tokens.encrypted_refresh_token)
            .maybe_access_token_expiry(tokens.access_token_expiry)
            .scope(tokens.scope)
            .call()
            .await
        {
            tracing::warn!(
                %user_id,
                ?provider,
                error = %e,
                "Failed to update OAuth credentials"
            );
        }
    }

    async fn create_oauth_user(
        &self,
        identity: NewOAuthIdentity,
    ) -> AuthResult<be_remote_db::User> {
        let NewOAuthIdentity {
            provider,
            provider_user_id,
            email,
            email_verified,
            name,
            tokens,
        } = identity;

        match self
            .db
            .create_user_with_oauth()
            .email(email.clone())
            .display_name(name)
            .email_verified(email_verified)
            .provider(provider)
            .provider_user_id(provider_user_id)
            .access_token(tokens.encrypted_access_token)
            .maybe_refresh_token(tokens.encrypted_refresh_token)
            .maybe_access_token_expiry(tokens.access_token_expiry)
            .scope(tokens.scope)
            .call()
            .await
        {
            Ok(user) => Ok(user),
            Err(DbError::UniqueViolation { ref constraint })
                if constraint == USERS_EMAIL_UNIQUE_CONSTRAINT =>
            {
                tracing::warn!(
                    ?provider,
                    email_hash = %Self::hash_email_for_log(&email),
                    "OAuth login rejected: email already registered under a different identity"
                );
                Err(AuthError::OAuthEmailConflict)
            }
            Err(e) => Err(AuthError::Database(e)),
        }
    }

    pub async fn cleanup_expired_data(&self) -> AuthResult<()> {
        self.db.cleanup_expired_auth_data().call().await?;
        Ok(())
    }

    async fn send_verification_email(&self, user: &be_remote_db::User) -> AuthResult<()> {
        let Some(email_service) = &self.email_service else {
            tracing::warn!("Email service not configured, skipping verification email");
            return Ok(());
        };

        let raw_token = self.generate_random_string(64)?;
        let token_hash = Self::hash_verification_token(&raw_token);

        self.db
            .create_email_verification_token()
            .user_id(user.id)
            .token_hash(token_hash)
            .expires_at(Utc::now() + Duration::hours(VERIFICATION_TOKEN_EXPIRY_HOURS))
            .call()
            .await?;

        email_service
            .send_verification_email(&user.email, &raw_token, user.display_name.as_deref())
            .await
            .map_err(|e| AuthError::Internal(format!("Failed to send verification email: {e}")))?;

        Ok(())
    }

    async fn sync_oauth_email_verified(
        &self,
        user: &be_remote_db::User,
        provider_verified: bool,
    ) -> bool {
        if !user.email_verified
            && provider_verified
            && let Err(e) = self.db.set_email_verified().user_id(user.id).call().await
        {
            tracing::warn!(
                "Failed to update email_verified for user {}: {}",
                user.id,
                e
            );
        }
        user.email_verified || provider_verified
    }

    fn hash_verification_token(token: &str) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        hasher.finalize().to_vec()
    }

    /// Produce a stable, non-reversible fingerprint of an email address for
    /// structured logs. Lowercased first so provider casing drift doesn't
    /// split the same email across two hashes. Lets SecOps correlate
    /// attempted-takeover events without storing PII.
    fn hash_email_for_log(email: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(email.to_ascii_lowercase().as_bytes());
        hex::encode(hasher.finalize())
    }

    fn token_response(&self, access_token: String, refresh_token: String) -> TokenResponse {
        TokenResponse {
            access_token,
            refresh_token,
            expires_in: self.jwt_config.access_token_expiry_hours * 3600,
        }
    }

    pub async fn register_user(
        &self,
        email: &str,
        password: &str,
        display_name: Option<String>,
    ) -> AuthResult<TokenResponse> {
        Self::validate_email(email)?;
        Self::validate_password(password)?;

        if self.db.user_exists_by_email().email(email).call().await? {
            return Err(AuthError::InvalidInput("Email already taken".into()));
        }

        let password_hash = self.hash_password(password)?;

        let mut user = self
            .db
            .create_user()
            .email(email.to_string())
            .maybe_display_name(display_name)
            .password_hash(password_hash)
            .call()
            .await?;

        if self.email_service.is_some() {
            if let Err(e) = self.send_verification_email(&user).await {
                tracing::error!(user_id = %user.id, "Failed to send verification email: {e}");
            }
        } else {
            self.db.set_email_verified().user_id(user.id).call().await?;
            user.email_verified = true;
        }

        let role = self
            .ensure_plan_and_resolve_role(user.id, &user.email)
            .await?;
        let (access_token, refresh_token) = self
            .generate_tokens(
                &user.id.to_string(),
                &user.email,
                user.display_name.clone(),
                role,
                user.email_verified,
            )
            .await?;

        Ok(self.token_response(access_token, refresh_token))
    }

    pub async fn login_email_password(
        &self,
        email: &str,
        password: &str,
    ) -> AuthResult<TokenResponse> {
        let email = email.trim();
        if email.is_empty() || password.is_empty() {
            return Err(AuthError::InvalidInput(
                "Email and password are required".into(),
            ));
        }

        let user = self
            .db
            .get_user()
            .email(email.to_string())
            .call()
            .await
            .map_err(|_| AuthError::InvalidCredentials)?;

        let pw_creds = self
            .db
            .get_password_credentials()
            .user_id(user.id)
            .call()
            .await
            .map_err(|_| AuthError::InvalidCredentials)?;

        let parsed_hash = PasswordHash::new(&pw_creds.password_hash)
            .map_err(|_| AuthError::Internal("Invalid stored password hash".into()))?;

        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|_| AuthError::InvalidCredentials)?;

        let role = self
            .ensure_plan_and_resolve_role(user.id, &user.email)
            .await?;
        let (access_token, refresh_token) = self
            .generate_tokens(
                &user.id.to_string(),
                &user.email,
                user.display_name.clone(),
                role,
                user.email_verified,
            )
            .await?;

        Ok(self.token_response(access_token, refresh_token))
    }

    pub async fn refresh_access_token(&self, refresh_token: &str) -> AuthResult<TokenResponse> {
        let token_hash = self.hash_refresh_token(refresh_token);

        let existing = self
            .db
            .get_refresh_token_by_hash()
            .token_hash(&token_hash)
            .call()
            .await
            .map_err(|_| AuthError::InvalidToken)?;

        let user = self.db.get_user().id(existing.user_id).call().await?;

        let role = self
            .ensure_plan_and_resolve_role(user.id, &user.email)
            .await?;

        let (access_token, new_refresh_token, new_token_hash, new_refresh_exp) = self
            .generate_jwt_tokens(
                &user.id.to_string(),
                &user.email,
                user.display_name.clone(),
                role,
                user.email_verified,
            )?;

        self.db
            .rotate_refresh_token()
            .old_token_hash(&token_hash)
            .new_token_hash(new_token_hash)
            .new_expires_at(new_refresh_exp)
            .call()
            .await
            .map_err(|_| AuthError::InvalidToken)?;

        Ok(self.token_response(access_token, new_refresh_token))
    }

    pub async fn logout(&self, refresh_token: &str) -> AuthResult<()> {
        let token_hash = self.hash_refresh_token(refresh_token);
        self.db
            .revoke_refresh_token()
            .token_hash(&token_hash)
            .call()
            .await
            .map_err(|_| AuthError::InvalidToken)?;
        Ok(())
    }

    pub async fn login_third_party(
        &self,
        provider: Provider,
        code: &str,
        state: &str,
        login_token: Option<String>,
    ) -> AuthResult<TokenResponse> {
        match provider {
            Provider::Google => self.handle_google_login(code, state, login_token).await,
            Provider::Github => self.handle_github_login(code, state, login_token).await,
        }
    }

    async fn handle_google_login(
        &self,
        code: &str,
        state: &str,
        login_token: Option<String>,
    ) -> AuthResult<TokenResponse> {
        if code.is_empty() {
            tracing::warn!("Google login attempt with empty authorization code");
            return Err(AuthError::InvalidInput(
                "Authorization code is required".into(),
            ));
        }

        if state.is_empty() {
            tracing::warn!("Google login attempt with empty state parameter");
            return Err(AuthError::InvalidInput(
                "State parameter is required".into(),
            ));
        }

        let oauth_state = self
            .db
            .consume_oauth_state()
            .state(state)
            .call()
            .await
            .map_err(|_| {
                tracing::warn!("Invalid or expired Google OAuth state");
                AuthError::InvalidInput("Invalid or expired state parameter".into())
            })?;

        let pkce_verifier = decrypt_sensitive_string(&oauth_state.pkce_verifier)?;

        let nonce = match &oauth_state.nonce {
            Some(encrypted_nonce) => {
                let nonce_str = decrypt_sensitive_string(encrypted_nonce)?;
                Nonce::new(nonce_str)
            }
            None => {
                return Err(AuthError::InvalidInput(
                    "Missing nonce for OIDC verification".into(),
                ));
            }
        };

        let google_client = self.google_oauth_client().await?;
        let user_info = google_client
            .exchange_code(code, pkce_verifier, &nonce)
            .await?;

        if !user_info.verified_email {
            tracing::warn!(
                "Google login rejected: email {} not verified",
                user_info.email
            );
            return Err(AuthError::EmailNotVerified);
        }

        let encrypted_access_token =
            encrypt_sensitive_string(user_info.access_token.expose_secret())?;
        let encrypted_refresh_token = user_info
            .refresh_token
            .as_ref()
            .map(|t| encrypt_sensitive_string(t.expose_secret()))
            .transpose()?;
        let access_token_expiry = user_info
            .expires_in
            .map(|duration| Utc::now() + Duration::seconds(duration.as_secs() as i64));

        let user = self
            .resolve_oauth_user(NewOAuthIdentity {
                provider: OAuthProvider::Google,
                provider_user_id: user_info.id.clone(),
                email: user_info.email.clone(),
                email_verified: user_info.verified_email,
                name: user_info.name.clone(),
                tokens: OAuthTokenBundle {
                    encrypted_access_token,
                    encrypted_refresh_token,
                    access_token_expiry,
                    scope: "openid email profile".to_string(),
                },
            })
            .await?;

        let email_verified = self
            .sync_oauth_email_verified(&user, user_info.verified_email)
            .await;

        if let Some(token) = login_token {
            self.try_associate_login_token_with_user(&user, &token)
                .await;
        }

        let role = self
            .ensure_plan_and_resolve_role(user.id, &user.email)
            .await?;
        let (access_token, refresh_token) = self
            .generate_tokens(
                &user.id.to_string(),
                &user.email,
                user.display_name.clone(),
                role,
                email_verified,
            )
            .await?;

        Ok(self.token_response(access_token, refresh_token))
    }

    async fn handle_github_login(
        &self,
        code: &str,
        state: &str,
        login_token: Option<String>,
    ) -> AuthResult<TokenResponse> {
        if code.is_empty() {
            return Err(AuthError::InvalidInput(
                "Authorization code is required".into(),
            ));
        }

        if state.is_empty() {
            return Err(AuthError::InvalidInput(
                "State parameter is required".into(),
            ));
        }

        let oauth_state = self
            .db
            .consume_oauth_state()
            .state(state)
            .call()
            .await
            .map_err(|_| {
                tracing::warn!("Invalid or expired GitHub OAuth state");
                AuthError::InvalidInput("Invalid or expired state parameter".into())
            })?;

        let pkce_verifier = decrypt_sensitive_string(&oauth_state.pkce_verifier)?;

        let github_client = self.github_oauth_client()?;
        let user_info = github_client.exchange_code(code, &pkce_verifier).await?;

        if !user_info.verified_email {
            tracing::warn!(
                "GitHub login rejected: email {} not verified",
                user_info.email
            );
            return Err(AuthError::EmailNotVerified);
        }

        let encrypted_access_token =
            encrypt_sensitive_string(user_info.access_token.expose_secret())?;

        let user = self
            .resolve_oauth_user(NewOAuthIdentity {
                provider: OAuthProvider::Github,
                provider_user_id: user_info.id.clone(),
                email: user_info.email.clone(),
                email_verified: user_info.verified_email,
                name: user_info.name.clone(),
                tokens: OAuthTokenBundle {
                    encrypted_access_token,
                    encrypted_refresh_token: None,
                    access_token_expiry: None,
                    scope: user_info.scope.clone(),
                },
            })
            .await?;

        let email_verified = self
            .sync_oauth_email_verified(&user, user_info.verified_email)
            .await;

        if let Some(token) = login_token {
            self.try_associate_login_token_with_user(&user, &token)
                .await;
        }

        let role = self
            .ensure_plan_and_resolve_role(user.id, &user.email)
            .await?;
        let (access_token, refresh_token) = self
            .generate_tokens(
                &user.id.to_string(),
                &user.email,
                user.display_name.clone(),
                role,
                email_verified,
            )
            .await?;

        Ok(self.token_response(access_token, refresh_token))
    }

    pub async fn third_party_auth_url(&self, provider: Provider) -> AuthResult<String> {
        match provider {
            Provider::Google => {
                tracing::info!("Generating Google OAuth URL");
                let google_client = self.google_oauth_client().await?;

                let state = self.generate_random_string(32)?;
                let (_, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
                let pkce_verifier_secret = pkce_verifier.secret().to_string();
                let nonce = Nonce::new_random();
                let nonce_secret = nonce.secret().to_string();

                let expires_at = Utc::now() + Duration::minutes(OAUTH_STATE_EXPIRY_MINUTES);

                let encrypted_pkce_verifier = encrypt_sensitive_string(&pkce_verifier_secret)?;
                let encrypted_nonce = encrypt_sensitive_string(&nonce_secret)?;

                self.db
                    .create_oauth_state()
                    .state(state.clone())
                    .pkce_verifier(encrypted_pkce_verifier)
                    .redirect_uri(google_client.redirect_uri().to_string())
                    .expires_at(expires_at)
                    .nonce(encrypted_nonce)
                    .call()
                    .await?;

                Ok(google_client.get_authorization_url_with_state_and_pkce(
                    &state,
                    &pkce_verifier_secret,
                    &nonce,
                ))
            }
            Provider::Github => {
                tracing::info!("Generating GitHub OAuth URL");
                let github_client = self.github_oauth_client()?;

                let state = self.generate_random_string(32)?;
                let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
                let pkce_verifier_secret = pkce_verifier.secret().to_string();

                let expires_at = Utc::now() + Duration::minutes(OAUTH_STATE_EXPIRY_MINUTES);

                let encrypted_pkce_verifier = encrypt_sensitive_string(&pkce_verifier_secret)?;

                self.db
                    .create_oauth_state()
                    .state(state.clone())
                    .pkce_verifier(encrypted_pkce_verifier)
                    .redirect_uri(github_client.redirect_uri().to_string())
                    .expires_at(expires_at)
                    .nonce(vec![])
                    .call()
                    .await?;

                Ok(github_client.get_authorization_url(&state, pkce_challenge.as_str()))
            }
        }
    }

    pub async fn login_by_login_token(&self, code_verifier: &str) -> AuthResult<TokenResponse> {
        if code_verifier.is_empty() {
            tracing::warn!("Login by login token request received with empty token");
            return Err(AuthError::InvalidInput("Login token is required".into()));
        }

        let code_challenge = self.code_verifier_to_challenge(code_verifier);
        let login_token_hash = self.hash_login_token(&code_challenge);

        let login_token = self
            .db
            .get_login_token_by_hash()
            .token_hash(&login_token_hash)
            .call()
            .await
            .map_err(|e| {
                tracing::warn!("Failed to find login token: {}", e);
                AuthError::InvalidToken
            })?;

        let user = self
            .db
            .get_user()
            .id(login_token.user_id)
            .call()
            .await
            .map_err(|e| {
                tracing::warn!("Failed to fetch user for login token: {}", e);
                AuthError::InvalidToken
            })?;

        let role = self.resolve_role(user.id).await;

        let (access_token, refresh_token, refresh_token_hash, refresh_exp) = self
            .generate_jwt_tokens(
                &user.id.to_string(),
                &user.email,
                user.display_name.clone(),
                role,
                user.email_verified,
            )?;

        self.db
            .consume_login_token_and_create_refresh_token()
            .login_token_hash(&login_token_hash)
            .user_id(user.id)
            .refresh_token_hash(refresh_token_hash)
            .refresh_token_expires_at(refresh_exp)
            .call()
            .await
            .map_err(|e| {
                tracing::warn!("Failed to exchange login token: {}", e);
                AuthError::InvalidToken
            })?;

        Ok(self.token_response(access_token, refresh_token))
    }

    pub async fn associate_login_token(
        &self,
        user_id: Uuid,
        code_challenge: &str,
    ) -> AuthResult<()> {
        if code_challenge.len() != 43
            || !code_challenge
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err(AuthError::InvalidInput("Invalid code challenge".into()));
        }

        let token_hash = self.hash_login_token(code_challenge);
        self.db
            .create_login_token()
            .token_hash(token_hash)
            .user_id(user_id)
            .expires_at(Utc::now() + Duration::minutes(LOGIN_TOKEN_EXPIRY_MINUTES))
            .call()
            .await
            .map_err(|e| {
                tracing::error!("Failed to associate login token: {}", e);
                AuthError::Internal("Failed to associate login token".into())
            })?;

        Ok(())
    }

    pub async fn check_email(
        &self,
        email: &str,
    ) -> AuthResult<(auth_core::CheckEmailStatus, Option<Provider>)> {
        if email.is_empty() {
            return Err(AuthError::InvalidInput("Email is required".into()));
        }

        let user = match self.db.get_user().email(email.to_string()).call().await {
            Ok(user) => user,
            Err(_) => return Ok((auth_core::CheckEmailStatus::NotFound, None)),
        };

        if let Ok(Some(oauth_provider)) = self
            .db
            .get_oauth_provider_for_user()
            .user_id(user.id)
            .call()
            .await
        {
            let provider = match oauth_provider {
                OAuthProvider::Google => Provider::Google,
                OAuthProvider::Github => Provider::Github,
            };
            return Ok((auth_core::CheckEmailStatus::Oauth, Some(provider)));
        }

        Ok((auth_core::CheckEmailStatus::Password, None))
    }

    pub async fn verify_email(&self, token: &str) -> AuthResult<TokenResponse> {
        if token.is_empty() {
            return Err(AuthError::InvalidInput(
                "Verification token is required".into(),
            ));
        }

        let token_hash = Self::hash_verification_token(token);

        let user = self
            .db
            .consume_email_verification_token()
            .token_hash(&token_hash)
            .call()
            .await
            .map_err(|e| {
                tracing::warn!("Email verification failed: {}", e);
                AuthError::InvalidInput("Invalid or expired verification token".into())
            })?;

        let role = self
            .ensure_plan_and_resolve_role(user.id, &user.email)
            .await?;

        let (access_token, refresh_token) = self
            .generate_tokens(
                &user.id.to_string(),
                &user.email,
                user.display_name.clone(),
                role,
                true,
            )
            .await?;

        tracing::info!(user_id = %user.id, "Email verified successfully");

        Ok(self.token_response(access_token, refresh_token))
    }

    pub async fn resend_verification_email(&self, user_id: Uuid) -> AuthResult<()> {
        let user = self
            .db
            .get_user()
            .id(user_id)
            .call()
            .await
            .map_err(|_| AuthError::InvalidCredentials)?;

        if user.email_verified {
            return Err(AuthError::EmailAlreadyVerified);
        }

        if let Ok(latest) = self
            .db
            .get_latest_verification_token_for_user()
            .user_id(user.id)
            .call()
            .await
        {
            let elapsed = Utc::now() - latest.created_at;
            if elapsed.num_seconds() < VERIFICATION_RESEND_COOLDOWN_SECONDS {
                return Err(AuthError::VerificationResendCooldown);
            }
        }

        self.send_verification_email(&user).await
    }
}

/// Build the auth router with the supplied dependencies.
///
/// Returns the bare router; the caller is expected to apply the cross-cutting
/// layers (CORS, body limit, auth middleware bypass for `/auth/*`) at the
/// monolith level so all REST services share the same outer pipeline.
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/auth/login", post(handlers::login))
        .route("/auth/register", post(handlers::register))
        .route("/auth/refresh", post(handlers::refresh))
        .route("/auth/logout", post(handlers::logout))
        .route("/auth/oauth/url", post(handlers::oauth_url))
        .route(
            "/auth/login-token/exchange",
            post(handlers::login_token_exchange),
        )
        .route(
            "/auth/login-token/associate",
            post(handlers::login_token_associate),
        )
        .route("/auth/email/check", post(handlers::email_check))
        .route("/auth/email/verify", post(handlers::email_verify))
        .route(
            "/auth/email/resend-verification",
            post(handlers::email_resend_verification),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Convenience constructor mirroring `be-payment-service::init_payment_service`
/// and `be-activity-service::init_activity_service`. Wires up application
/// state and returns the router ready to merge into the monolith HTTP
/// pipeline.
pub fn init_auth_service(
    db: Arc<DatabaseManager>,
    jwt_config: JwtConfig,
    email_service: Option<Arc<EmailService>>,
) -> Result<Router> {
    tracing::debug!("Initializing auth service");
    let state = Arc::new(AppState::new(db, jwt_config, email_service));
    Ok(create_router(state))
}
