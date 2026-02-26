pub use auth_core::{Claims, Role};
use be_auth_core::JwtConfig;
use be_remote_db::{DatabaseManager, OAuthProvider};
use bon::bon;
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, Header, encode};
use openidconnect::{Nonce, PkceCodeChallenge, PkceCodeVerifier};
use proto_gen::auth::{
    EmailPasswordCredentials, LoginByLoginTokenRequest, LoginRequest, Provider,
    RefreshTokenRequest, RegisterRequest, ThirdPartyAuthUrlRequest, ThirdPartyAuthUrlResponse,
    ThirdPartyCredentials, TokenResponse, login_request::Credential,
    proto_auth_service_server::ProtoAuthService,
};
use rand::TryRngCore;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::sync::OnceCell;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};

pub mod crypto;
pub mod error;
pub mod oauth;

use crypto::{decrypt_sensitive_string, encrypt_sensitive_string};
use error::AuthError;
use oauth::github::GitHubOAuthClient;
use oauth::google::GoogleOAuthClient;

pub struct AuthService {
    db: Arc<DatabaseManager>,
    jwt_config: JwtConfig,
    google_oauth_client: OnceCell<GoogleOAuthClient>,
    github_oauth_client: std::sync::OnceLock<GitHubOAuthClient>,
}

#[bon]
impl AuthService {
    pub fn new(db: Arc<DatabaseManager>, jwt_config: JwtConfig) -> Self {
        tracing::info!("Creating new AuthService instance");
        Self {
            db,
            jwt_config,
            google_oauth_client: OnceCell::new(),
            github_oauth_client: std::sync::OnceLock::new(),
        }
    }

    async fn google_oauth_client(&self) -> Result<&GoogleOAuthClient, AuthError> {
        self.google_oauth_client
            .get_or_try_init(|| async {
                let config = oauth::google::GoogleOAuthConfig::from_env()?;
                Ok(GoogleOAuthClient::discover(config).await?)
            })
            .await
    }

    fn github_oauth_client(&self) -> Result<&GitHubOAuthClient, AuthError> {
        if let Some(client) = self.github_oauth_client.get() {
            return Ok(client);
        }
        let config = oauth::github::GitHubOAuthConfig::from_env()?;
        let client = GitHubOAuthClient::new(config);
        Ok(self.github_oauth_client.get_or_init(|| client))
    }

    pub fn authenticate_request_access_token<T>(
        &self,
        request: &Request<T>,
    ) -> Result<Claims, AuthError> {
        let token = self.extract_bearer_token(request)?;
        self.jwt_config
            .validate_access_token(token)
            .map_err(|_| AuthError::InvalidToken)
    }

    pub fn authenticate_request_refresh_token<T>(
        &self,
        request: &Request<T>,
    ) -> Result<(Claims, String), AuthError> {
        let token = self.extract_bearer_token(request)?;
        let claims = self
            .jwt_config
            .validate_refresh_token(token)
            .map_err(|_| AuthError::InvalidToken)?;
        Ok((claims, token.to_string()))
    }

    fn extract_bearer_token<'a, T>(&self, request: &'a Request<T>) -> Result<&'a str, AuthError> {
        let auth_header = request
            .metadata()
            .get("authorization")
            .ok_or(AuthError::MissingAuthHeader)?;

        let auth_str = auth_header
            .to_str()
            .map_err(|_| AuthError::InvalidAuthHeader)?;

        auth_str
            .strip_prefix("Bearer ")
            .ok_or(AuthError::InvalidAuthHeader)
    }

    fn hash_password(&self, password: &str) -> Result<String, AuthError> {
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| AuthError::PasswordHash(e.to_string()))?;
        Ok(hash.to_string())
    }

    fn verify_password(&self, password: &str, hash: &str) -> Result<bool, AuthError> {
        let parsed_hash =
            PasswordHash::new(hash).map_err(|e| AuthError::PasswordHash(e.to_string()))?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }

    fn hash_refresh_token(&self, token: &str) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        hasher.finalize().to_vec()
    }

    fn is_approved_email(&self, email: &str) -> bool {
        let email = email.to_lowercase();
        self.jwt_config
            .approved_emails
            .iter()
            .any(|approved| approved == "*" || *approved == email)
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

    async fn ensure_plan_and_resolve_role(
        &self,
        user_id: Uuid,
        email: &str,
    ) -> Result<Role, AuthError> {
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

    async fn generate_tokens(
        &self,
        user_id: &str,
        username: &str,
        email: &str,
        role: Role,
    ) -> Result<(String, String), AuthError> {
        let now = Utc::now();
        let access_exp = now + Duration::hours(self.jwt_config.access_token_expiry_hours);
        let refresh_exp = now + Duration::days(self.jwt_config.refresh_token_expiry_days);

        let access_claims = Claims {
            sub: user_id.to_string(),
            username: username.to_string(),
            email: email.to_string(),
            exp: access_exp.timestamp(),
            iat: now.timestamp(),
            token_type: "access".to_string(),
            role: role.clone(),
        };

        let refresh_claims = Claims {
            sub: user_id.to_string(),
            username: username.to_string(),
            email: email.to_string(),
            exp: refresh_exp.timestamp(),
            iat: now.timestamp(),
            token_type: "refresh".to_string(),
            role,
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

        let user_uuid = Uuid::parse_str(user_id)
            .map_err(|e| AuthError::Internal(format!("Invalid user ID format: {e}")))?;

        let token_hash = self.hash_refresh_token(&refresh_token);

        self.db
            .create_refresh_token()
            .user_id(user_uuid)
            .token_hash(token_hash)
            .expires_at(refresh_exp)
            .call()
            .await?;

        Ok((access_token, refresh_token))
    }

    fn generate_random_string(&self, length: usize) -> Result<String, AuthError> {
        let byte_len = length.div_ceil(2);
        let mut bytes = vec![0u8; byte_len];
        rand::rngs::OsRng
            .try_fill_bytes(&mut bytes)
            .map_err(|e| AuthError::Internal(format!("Failed to generate random bytes: {e}")))?;

        let mut hex = hex::encode(bytes);
        hex.truncate(length);
        Ok(hex)
    }

    /// The `token` parameter is a code_challenge (already S256-transformed on the client),
    /// NOT a raw code_verifier. This matches `login_by_login_token` which converts
    /// code_verifier -> code_challenge before DB lookup.
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
            .expires_at(Utc::now() + Duration::minutes(20))
            .call()
            .await
        {
            Ok(_) => {
                tracing::info!(
                    "Successfully associated login token with user: {}",
                    user.username
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

    #[builder]
    async fn find_or_create_oauth_user(
        &self,
        provider: OAuthProvider,
        provider_user_id: &str,
        email: &str,
        email_verified: bool,
        name: &str,
        username: &str,
        encrypted_access_token: Vec<u8>,
        encrypted_refresh_token: Option<Vec<u8>>,
        token_expiry: Option<chrono::DateTime<Utc>>,
        scope: String,
    ) -> Result<be_remote_db::User, AuthError> {
        let existing = self
            .db
            .get_user_by_oauth_provider()
            .provider(provider)
            .provider_user_id(provider_user_id)
            .call()
            .await;

        match existing {
            Ok(user) => {
                if let Ok(oauth_creds) = self
                    .db
                    .get_oauth_credentials_by_provider_and_user()
                    .provider(provider)
                    .user_id(user.id)
                    .call()
                    .await
                    && let Err(e) = self
                        .db
                        .update_oauth_credentials()
                        .id(oauth_creds.id)
                        .access_token(encrypted_access_token)
                        .maybe_refresh_token(encrypted_refresh_token)
                        .maybe_access_token_expiry(token_expiry)
                        .scope(scope)
                        .call()
                        .await
                {
                    tracing::warn!("Failed to update OAuth credentials: {}", e);
                }
                Ok(user)
            }
            Err(_) => {
                if let Ok(existing_user) = self.db.get_user().email(email.to_string()).call().await
                {
                    tracing::info!(
                        "Linking {} provider to existing user {} via email match",
                        provider,
                        existing_user.username
                    );
                    self.db
                        .create_oauth_credentials()
                        .user_id(existing_user.id)
                        .provider(provider)
                        .provider_user_id(provider_user_id.to_string())
                        .access_token(encrypted_access_token)
                        .maybe_refresh_token(encrypted_refresh_token)
                        .maybe_access_token_expiry(token_expiry)
                        .scope(scope)
                        .call()
                        .await?;
                    return Ok(existing_user);
                }

                let base_username = username.to_string();
                let mut final_username = base_username.clone();
                let mut counter = 0u32;
                const MAX_RETRIES: u32 = 5;

                loop {
                    match self
                        .db
                        .create_user_with_oauth()
                        .username(final_username.clone())
                        .email(email.to_string())
                        .display_name(name.to_string())
                        .email_verified(email_verified)
                        .provider(provider)
                        .provider_user_id(provider_user_id.to_string())
                        .access_token(encrypted_access_token.clone())
                        .maybe_refresh_token(encrypted_refresh_token.clone())
                        .maybe_access_token_expiry(token_expiry)
                        .scope(scope.clone())
                        .call()
                        .await
                    {
                        Ok(user) => break Ok(user),
                        Err(be_remote_db::DbError::Duplicate { value, .. })
                            if value.contains("username") =>
                        {
                            counter += 1;
                            if counter >= MAX_RETRIES {
                                tracing::error!(
                                    "Failed to create unique username after {} attempts",
                                    MAX_RETRIES
                                );
                                return Err(AuthError::Internal(
                                    "Failed to create user account".into(),
                                ));
                            }
                            final_username = format!("{}_{}", base_username, counter);
                            tracing::info!(
                                "Username conflict ({}), retrying with '{}'",
                                value,
                                final_username
                            );
                        }
                        Err(e) => break Err(AuthError::Database(e)),
                    }
                }
            }
        }
    }

    pub async fn cleanup_expired_data(&self) -> Result<(), AuthError> {
        self.db.cleanup_expired_auth_data().call().await?;
        Ok(())
    }

    pub async fn register_user(
        &self,
        username: &str,
        email: &str,
        password: &str,
        display_name: Option<String>,
    ) -> Result<TokenResponse, AuthError> {
        if self
            .db
            .user_exists_by_username()
            .username(username)
            .call()
            .await?
        {
            return Err(AuthError::InvalidInput(
                "Username or email already taken".into(),
            ));
        }

        if self.db.user_exists_by_email().email(email).call().await? {
            return Err(AuthError::InvalidInput(
                "Username or email already taken".into(),
            ));
        }

        let password_hash = self.hash_password(password)?;

        let user = self
            .db
            .create_user()
            .username(username.to_string())
            .email(email.to_string())
            .maybe_display_name(display_name)
            .password_hash(password_hash)
            .call()
            .await?;

        let role = self
            .ensure_plan_and_resolve_role(user.id, &user.email)
            .await?;
        let (access_token, refresh_token) = self
            .generate_tokens(&user.id.to_string(), &user.username, &user.email, role)
            .await?;

        Ok(TokenResponse {
            access_token,
            refresh_token,
            expires_in: self.jwt_config.access_token_expiry_hours * 3600,
        })
    }

    pub async fn refresh_access_token(
        &self,
        refresh_token: &str,
    ) -> Result<TokenResponse, AuthError> {
        let token_hash = self.hash_refresh_token(refresh_token);

        let revoked_token = self
            .db
            .revoke_refresh_token()
            .token_hash(&token_hash)
            .call()
            .await
            .map_err(|_| AuthError::InvalidToken)?;

        let user = self.db.get_user().id(revoked_token.user_id).call().await?;

        let role = self
            .ensure_plan_and_resolve_role(user.id, &user.email)
            .await?;
        let (access_token, new_refresh_token) = self
            .generate_tokens(&user.id.to_string(), &user.username, &user.email, role)
            .await?;

        Ok(TokenResponse {
            access_token,
            refresh_token: new_refresh_token,
            expires_in: self.jwt_config.access_token_expiry_hours * 3600,
        })
    }

    async fn handle_google_login(
        &self,
        creds: ThirdPartyCredentials,
    ) -> Result<Response<TokenResponse>, AuthError> {
        let code = &creds.code;
        let state = &creds.state;

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
                tracing::warn!("Invalid or expired OAuth state: {}", state);
                AuthError::InvalidInput("Invalid or expired state parameter".into())
            })?;

        let pkce_verifier = decrypt_sensitive_string(&oauth_state.pkce_verifier)?;

        let nonce = match &oauth_state.nonce {
            Some(encrypted_nonce) => {
                let nonce_str = decrypt_sensitive_string(encrypted_nonce)?;
                Some(Nonce::new(nonce_str))
            }
            None => None,
        };

        let google_client = self.google_oauth_client().await?;
        let user_info = google_client
            .exchange_code(code, pkce_verifier, nonce.as_ref())
            .await?;

        if !user_info.verified_email {
            tracing::warn!(
                "Google login rejected: email {} not verified",
                user_info.email
            );
            return Err(AuthError::EmailNotVerified);
        }

        let oauth_access_token = encrypt_sensitive_string(&user_info.access_token)?;
        let oauth_refresh_token = user_info
            .refresh_token
            .as_ref()
            .map(|t| encrypt_sensitive_string(t))
            .transpose()?;
        let oauth_token_expiry = user_info.expires_in.map(|duration| {
            chrono::Utc::now() + chrono::Duration::seconds(duration.as_secs() as i64)
        });

        let username = user_info
            .email
            .split('@')
            .next()
            .filter(|s| !s.is_empty())
            .unwrap_or(&user_info.name)
            .to_string();

        if username.is_empty() {
            tracing::warn!("Empty username detected");
            return Err(AuthError::InvalidInput(
                "Unable to determine username from OAuth profile".into(),
            ));
        }

        let user = self
            .find_or_create_oauth_user()
            .provider(OAuthProvider::Google)
            .provider_user_id(&user_info.id)
            .email(&user_info.email)
            .email_verified(user_info.verified_email)
            .name(&user_info.name)
            .username(&username)
            .encrypted_access_token(oauth_access_token)
            .maybe_encrypted_refresh_token(oauth_refresh_token)
            .maybe_token_expiry(oauth_token_expiry)
            .scope("openid email profile".to_string())
            .call()
            .await?;

        if let Some(token) = creds.login_token {
            self.try_associate_login_token_with_user(&user, &token)
                .await;
        }

        let role = self
            .ensure_plan_and_resolve_role(user.id, &user.email)
            .await?;
        let (access_token, refresh_token) = self
            .generate_tokens(&user.id.to_string(), &user.username, &user.email, role)
            .await?;

        let response = TokenResponse {
            access_token,
            refresh_token,
            expires_in: self.jwt_config.access_token_expiry_hours * 3600,
        };

        Ok(Response::new(response))
    }

    async fn handle_github_login(
        &self,
        creds: ThirdPartyCredentials,
    ) -> Result<Response<TokenResponse>, AuthError> {
        let code = &creds.code;
        let state = &creds.state;

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
                tracing::warn!("Invalid or expired OAuth state: {}", state);
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

        let oauth_access_token = encrypt_sensitive_string(&user_info.access_token)?;

        let user = self
            .find_or_create_oauth_user()
            .provider(OAuthProvider::Github)
            .provider_user_id(&user_info.id)
            .email(&user_info.email)
            .email_verified(user_info.verified_email)
            .name(&user_info.name)
            .username(&user_info.username)
            .encrypted_access_token(oauth_access_token)
            .scope(user_info.scope)
            .call()
            .await?;

        if let Some(token) = creds.login_token {
            self.try_associate_login_token_with_user(&user, &token)
                .await;
        }

        let role = self
            .ensure_plan_and_resolve_role(user.id, &user.email)
            .await?;
        let (access_token, refresh_token) = self
            .generate_tokens(&user.id.to_string(), &user.username, &user.email, role)
            .await?;

        let response = TokenResponse {
            access_token,
            refresh_token,
            expires_in: self.jwt_config.access_token_expiry_hours * 3600,
        };

        Ok(Response::new(response))
    }
}

#[tonic::async_trait]
impl ProtoAuthService for AuthService {
    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<TokenResponse>, Status> {
        tracing::info!("Login request received");
        let req = request.into_inner();

        let credential = req.credential.ok_or_else(|| {
            tracing::warn!("Login request missing credentials");
            Status::invalid_argument("Missing credentials")
        })?;

        match credential {
            Credential::EmailPassword(creds) => self
                .handle_email_password_login(creds)
                .await
                .map_err(Into::into),
            Credential::ThirdParty(creds) => {
                let provider = Provider::try_from(creds.provider)
                    .map_err(|_| Status::invalid_argument("Invalid provider"))?;

                match provider {
                    Provider::Google => self.handle_google_login(creds).await.map_err(Into::into),
                    Provider::Github => self.handle_github_login(creds).await.map_err(Into::into),
                    Provider::Unspecified => {
                        tracing::warn!("Unspecified provider in OAuth request");
                        Err(Status::invalid_argument("Provider must be specified"))
                    }
                }
            }
        }
    }

    async fn register(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<Response<TokenResponse>, Status> {
        tracing::info!("Register request received");
        let req = request.into_inner();

        let response = self
            .register_user(&req.username, &req.email, &req.password, req.display_name)
            .await?;

        Ok(Response::new(response))
    }

    async fn refresh_token(
        &self,
        request: Request<RefreshTokenRequest>,
    ) -> Result<Response<TokenResponse>, Status> {
        tracing::info!("Refresh token request received");
        let (_, refresh_token) = self.authenticate_request_refresh_token(&request)?;
        let response = self.refresh_access_token(&refresh_token).await?;
        Ok(Response::new(response))
    }

    async fn get_third_party_auth_url(
        &self,
        request: Request<ThirdPartyAuthUrlRequest>,
    ) -> Result<Response<ThirdPartyAuthUrlResponse>, Status> {
        let req = request.into_inner();

        tracing::info!(
            "Third-party auth URL request received for provider: {:?}",
            req.provider
        );

        let provider = Provider::try_from(req.provider)
            .map_err(|_| Status::invalid_argument("Invalid provider"))?;

        let auth_url = match provider {
            Provider::Google => {
                tracing::info!("Generating Google OAuth URL");

                let google_client = self.google_oauth_client().await.map_err(Status::from)?;

                let state = self.generate_random_string(32).map_err(Status::from)?;
                let (_, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
                let pkce_verifier_secret = pkce_verifier.secret().to_string();
                let nonce = Nonce::new_random();
                let nonce_secret = nonce.secret().to_string();

                let expires_at = Utc::now() + Duration::minutes(10);

                let encrypted_pkce_verifier = encrypt_sensitive_string(&pkce_verifier_secret)
                    .map_err(|e| Status::from(AuthError::from(e)))?;

                let encrypted_nonce = encrypt_sensitive_string(&nonce_secret)
                    .map_err(|e| Status::from(AuthError::from(e)))?;

                self.db
                    .create_oauth_state()
                    .state(state.clone())
                    .pkce_verifier(encrypted_pkce_verifier)
                    .redirect_uri(google_client.redirect_uri().to_string())
                    .expires_at(expires_at)
                    .nonce(encrypted_nonce)
                    .call()
                    .await
                    .map_err(|e| Status::from(AuthError::from(e)))?;

                google_client.get_authorization_url_with_state_and_pkce(
                    &state,
                    &pkce_verifier_secret,
                    &nonce,
                )
            }
            Provider::Github => {
                tracing::info!("Generating GitHub OAuth URL");

                let github_client = self.github_oauth_client().map_err(Status::from)?;

                let state = self.generate_random_string(32).map_err(Status::from)?;
                let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
                let pkce_verifier_secret = pkce_verifier.secret().to_string();

                let expires_at = Utc::now() + Duration::minutes(10);

                let encrypted_pkce_verifier = encrypt_sensitive_string(&pkce_verifier_secret)
                    .map_err(|e| Status::from(AuthError::from(e)))?;

                self.db
                    .create_oauth_state()
                    .state(state.clone())
                    .pkce_verifier(encrypted_pkce_verifier)
                    .redirect_uri(github_client.redirect_uri().to_string())
                    .expires_at(expires_at)
                    .nonce(vec![])
                    .call()
                    .await
                    .map_err(|e| Status::from(AuthError::from(e)))?;

                github_client.get_authorization_url(&state, pkce_challenge.as_str())
            }
            Provider::Unspecified => {
                tracing::warn!("Unspecified provider in OAuth request");
                return Err(Status::invalid_argument("Provider must be specified"));
            }
        };

        let response = ThirdPartyAuthUrlResponse { url: auth_url };
        Ok(Response::new(response))
    }

    async fn login_by_login_token(
        &self,
        request: Request<LoginByLoginTokenRequest>,
    ) -> Result<Response<TokenResponse>, Status> {
        tracing::info!("Login by login token request received");

        let req = request.into_inner();
        let code_verifier = req.token;

        if code_verifier.is_empty() {
            tracing::warn!("Login by login token request received with empty token");
            return Err(Status::invalid_argument("Login token is required"));
        }

        let code_challenge = self.code_verifier_to_challenge(&code_verifier);
        let token_hash = self.hash_login_token(&code_challenge);

        let login_token = self
            .db
            .consume_login_token()
            .token_hash(&token_hash)
            .call()
            .await
            .map_err(|_| {
                tracing::warn!("Login token not found, expired, or already consumed");
                Status::from(AuthError::InvalidToken)
            })?;

        let user = self
            .db
            .get_user()
            .id(login_token.user_id)
            .call()
            .await
            .map_err(|e| {
                tracing::error!("User not found for login token: {}", e);
                Status::from(AuthError::Internal("User not found".into()))
            })?;

        let role = self
            .ensure_plan_and_resolve_role(user.id, &user.email)
            .await
            .map_err(Status::from)?;
        let (access_token, refresh_token) = self
            .generate_tokens(&user.id.to_string(), &user.username, &user.email, role)
            .await
            .map_err(Status::from)?;

        let response = TokenResponse {
            access_token,
            refresh_token,
            expires_in: self.jwt_config.access_token_expiry_hours * 3600,
        };

        Ok(Response::new(response))
    }
}

impl AuthService {
    async fn handle_email_password_login(
        &self,
        creds: EmailPasswordCredentials,
    ) -> Result<Response<TokenResponse>, AuthError> {
        let login = creds.login.trim();
        let password = creds.password;

        if login.is_empty() || password.is_empty() {
            tracing::warn!("Login attempt with empty credentials");
            return Err(AuthError::InvalidInput(
                "Login and password cannot be empty".into(),
            ));
        }

        let user = if login.contains('@') {
            self.db.get_user().email(login.to_string()).call().await
        } else {
            self.db.get_user().username(login.to_string()).call().await
        };

        let user = user.map_err(|_| {
            tracing::warn!("Login failed: user not found for {}", login);
            AuthError::InvalidCredentials
        })?;

        let password_creds = self
            .db
            .get_password_credentials()
            .user_id(user.id)
            .call()
            .await
            .map_err(|_| {
                tracing::warn!("Login failed: no password credentials for {}", login);
                AuthError::InvalidCredentials
            })?;

        let password_valid = self.verify_password(&password, &password_creds.password_hash)?;

        if !password_valid {
            tracing::warn!("Login failed: invalid password for {}", login);
            return Err(AuthError::InvalidCredentials);
        }

        let role = self
            .ensure_plan_and_resolve_role(user.id, &user.email)
            .await?;
        let (access_token, refresh_token) = self
            .generate_tokens(&user.id.to_string(), &user.username, &user.email, role)
            .await?;

        let response = TokenResponse {
            access_token,
            refresh_token,
            expires_in: self.jwt_config.access_token_expiry_hours * 3600,
        };

        Ok(Response::new(response))
    }
}
