use anyhow::{Result, anyhow};
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
pub use auth_core::{Claims, Role};
use be_auth_grpc::JwtConfig;
use be_remote_db::{
    CreateLoginToken, CreateOAuthCredentials, CreateOAuthState, CreateRefreshToken,
    DatabaseManager, NewUser, OAuthProvider,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, Header, encode};
use openidconnect::{Nonce, PkceCodeChallenge, PkceCodeVerifier};
use proto_gen::auth::{
    EmailPasswordCredentials, GetLoginTokenResponse, LoginByLoginTokenRequest, LoginRequest,
    Provider, RefreshTokenRequest, RegisterRequest, ThirdPartyAuthUrlRequest,
    ThirdPartyAuthUrlResponse, ThirdPartyCredentials, TokenResponse, login_request::Credential,
    proto_auth_service_server::ProtoAuthService,
};
use rand::TryRngCore;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::sync::OnceCell;
use tonic::{Request, Response, Status};
use tracing::{error, info, warn};
use uuid::Uuid;

pub mod crypto;
pub mod oauth;

use crypto::{decrypt_sensitive_string, encrypt_sensitive_string};
use oauth::google::GoogleOAuthClient;

pub struct AuthService {
    db: Arc<DatabaseManager>,
    jwt_config: JwtConfig,
    #[allow(dead_code)]
    desktop_login_url: String,
    google_oauth_client: OnceCell<GoogleOAuthClient>,
}

impl AuthService {
    pub fn new(db: Arc<DatabaseManager>, jwt_config: JwtConfig) -> Self {
        info!("Creating new AuthService instance");
        let desktop_login_url = std::env::var("DESKTOP_LOGIN_URL").unwrap_or_else(|e| {
            error!("DESKTOP_LOGIN_URL environment variable not set: {}", e);
            "http://localhost:5173/login".to_string()
        });
        Self {
            db,
            jwt_config,
            desktop_login_url,
            google_oauth_client: OnceCell::new(),
        }
    }

    async fn google_oauth_client(&self) -> Result<&GoogleOAuthClient, Status> {
        self.google_oauth_client
            .get_or_try_init(|| async {
                let config = oauth::google::GoogleOAuthConfig::from_env().map_err(|e| {
                    error!("Failed to load Google OAuth config: {}", e);
                    Status::internal("OAuth configuration error")
                })?;
                GoogleOAuthClient::discover(config).await.map_err(|e| {
                    error!("Failed to create Google OAuth client: {}", e);
                    Status::internal("Failed to initialize OAuth client")
                })
            })
            .await
    }

    pub fn authenticate_request_access_token<T>(&self, request: &Request<T>) -> Result<Claims> {
        let auth_header = request
            .metadata()
            .get("authorization")
            .ok_or_else(|| anyhow!("Missing authorization header"))?;

        let auth_str = auth_header
            .to_str()
            .map_err(|_| anyhow!("Invalid authorization header format"))?;

        if !auth_str.starts_with("Bearer ") {
            return Err(anyhow!("Authorization header must start with 'Bearer '"));
        }

        let token = &auth_str[7..];
        self.jwt_config.validate_access_token(token)
    }

    pub fn authenticate_request_refresh_token<T>(
        &self,
        request: &Request<T>,
    ) -> Result<(Claims, String)> {
        let auth_header = request
            .metadata()
            .get("authorization")
            .ok_or_else(|| anyhow!("Missing authorization header"))?;

        let auth_str = auth_header
            .to_str()
            .map_err(|_| anyhow!("Invalid authorization header format"))?;

        if !auth_str.starts_with("Bearer ") {
            return Err(anyhow!("Authorization header must start with 'Bearer '"));
        }

        let token = &auth_str[7..];

        let claims = self.jwt_config.validate_refresh_token(token)?;

        Ok((claims, token.to_string()))
    }

    fn hash_password(&self, password: &str) -> Result<String> {
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow!("Failed to hash password: {}", e))?;
        Ok(hash.to_string())
    }

    fn verify_password(&self, password: &str, hash: &str) -> Result<bool> {
        let parsed_hash =
            PasswordHash::new(hash).map_err(|e| anyhow!("Invalid password hash format: {}", e))?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }

    fn hash_refresh_token(&self, token: &str) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        hasher.finalize().to_vec()
    }

    async fn resolve_role(&self, user_id: Uuid) -> Role {
        let local_mode = std::env::var("RUNNING_EURORA_FULLY_LOCAL")
            .map(|v| v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        if local_mode {
            return Role::Enterprise;
        }

        match self.db.get_billing_state_for_user(user_id).await {
            Ok(Some(state)) if matches!(state.status.as_deref(), Some("active" | "trialing")) => {
                match state.plan_id.as_deref() {
                    Some("enterprise") => Role::Enterprise,
                    _ => Role::Tier1,
                }
            }
            _ => Role::Free,
        }
    }

    async fn generate_tokens(
        &self,
        user_id: &str,
        username: &str,
        email: &str,
        role: Role,
    ) -> Result<(String, String)> {
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
        .map_err(|e| anyhow!("Failed to generate access token: {}", e))?;

        let refresh_token = encode(
            &header,
            &refresh_claims,
            &self.jwt_config.refresh_token_encoding_key,
        )
        .map_err(|e| anyhow!("Failed to generate refresh token: {}", e))?;

        let user_uuid =
            Uuid::parse_str(user_id).map_err(|e| anyhow!("Invalid user ID format: {}", e))?;

        let token_hash = self.hash_refresh_token(&refresh_token);
        let refresh_request = CreateRefreshToken {
            user_id: user_uuid,
            token_hash,
            expires_at: refresh_exp,
        };

        self.db
            .create_refresh_token(refresh_request)
            .await
            .map_err(|e| anyhow!("Failed to store refresh token: {}", e))?;

        Ok((access_token, refresh_token))
    }

    fn generate_random_string(&self, length: usize) -> Result<String> {
        let byte_len = length.div_ceil(2); // round up for odd lengths
        let mut bytes = vec![0u8; byte_len];
        rand::rngs::OsRng.try_fill_bytes(&mut bytes).map_err(|e| {
            error!("Failed to generate random bytes: {}", e);
            Status::internal("Failed to generate random bytes")
        })?;

        let mut hex = hex::encode(bytes);
        hex.truncate(length); // exact length
        Ok(hex)
    }

    /// Try to associate any pending login tokens with the user
    /// This looks for unused login tokens and associates them with the user
    ///
    /// IMPORTANT: The `token` parameter is expected to be a code_challenge (already transformed
    /// from code_verifier using PKCE S256 method on the client side), NOT a raw code_verifier.
    /// This matches the verification logic in `login_by_login_token` where the desktop client
    /// sends a code_verifier which gets converted to code_challenge before lookup.
    async fn try_associate_login_token_with_user(
        &self,
        user: &be_remote_db::User,
        code_challenge: &str,
    ) {
        let token_hash = self.hash_login_token(code_challenge);
        let create_request = CreateLoginToken {
            token_hash,
            expires_at: Utc::now() + Duration::minutes(20),
            user_id: user.id,
        };

        match self.db.create_login_token(create_request).await {
            Ok(_) => {
                info!(
                    "Successfully associated login token with user: {}",
                    user.username
                );
            }
            Err(e) => {
                error!("Failed to update login token with user_id: {}", e);
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

    pub async fn register_user(
        &self,
        username: &str,
        email: &str,
        password: &str,
        display_name: Option<String>,
    ) -> Result<TokenResponse> {
        if self
            .db
            .user_exists_by_username(username)
            .await
            .unwrap_or(false)
        {
            return Err(anyhow!("Username already exists"));
        }

        if self.db.user_exists_by_email(email).await? {
            return Err(anyhow!("Email already exists"));
        }

        let password_hash = self.hash_password(password)?;

        let create_request = NewUser {
            username: username.to_string(),
            email: email.to_string(),
            display_name,
            password_hash: Some(password_hash),
        };

        let user = self
            .db
            .create_user(create_request)
            .await
            .map_err(|e| anyhow!("Failed to create user: {}", e))?;

        let (access_token, refresh_token) = self
            .generate_tokens(
                &user.id.to_string(),
                &user.username,
                &user.email,
                Role::Free,
            )
            .await?;

        Ok(TokenResponse {
            access_token,
            refresh_token,
            expires_in: self.jwt_config.access_token_expiry_hours * 3600, // Convert to seconds
        })
    }

    pub async fn refresh_access_token(&self, refresh_token: &str) -> Result<TokenResponse> {
        let token_hash = self.hash_refresh_token(refresh_token);

        let stored_token = self
            .db
            .get_refresh_token_by_hash(&token_hash)
            .await
            .map_err(|e| anyhow!("Invalid or expired refresh token: {}", e))?;

        let user = self
            .db
            .get_user_by_id(stored_token.user_id)
            .await
            .map_err(|e| anyhow!("User not found: {}", e))?;

        self.db
            .revoke_refresh_token(&token_hash)
            .await
            .map_err(|e| anyhow!("Failed to revoke old refresh token: {}", e))?;

        let role = self.resolve_role(user.id).await;
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
    ) -> Result<Response<TokenResponse>, Status> {
        let code = &creds.code;
        let state = &creds.state;

        if code.is_empty() {
            warn!("Google login attempt with empty authorization code");
            return Err(Status::invalid_argument("Authorization code is required"));
        }

        if state.is_empty() {
            warn!("Google login attempt with empty state parameter");
            return Err(Status::invalid_argument("State parameter is required"));
        }

        let oauth_state = match self.db.get_oauth_state_by_state(state).await {
            Ok(oauth_state) => oauth_state,
            Err(_) => {
                warn!("Invalid or expired OAuth state: {}", state);
                return Err(Status::invalid_argument(
                    "Invalid or expired state parameter",
                ));
            }
        };

        let pkce_verifier = decrypt_sensitive_string(&oauth_state.pkce_verifier).map_err(|e| {
            error!("Failed to decrypt PKCE verifier: {}", e);
            Status::internal("Failed to process OAuth state")
        })?;

        let nonce = match &oauth_state.nonce {
            Some(encrypted_nonce) => {
                let nonce_str = decrypt_sensitive_string(encrypted_nonce).map_err(|e| {
                    error!("Failed to decrypt nonce: {}", e);
                    Status::internal("Failed to process OAuth state")
                })?;
                Some(Nonce::new(nonce_str))
            }
            None => None,
        };

        // Must succeed to prevent replay attacks
        if let Err(e) = self.db.consume_oauth_state(state).await {
            error!("Failed to consume OAuth state: {}", e);
            return Err(Status::internal("Failed to process OAuth state"));
        }

        let google_client = self.google_oauth_client().await?;
        let user_info = google_client
            .exchange_code(code, pkce_verifier, nonce.as_ref())
            .await
            .map_err(|e| {
                error!("Failed to exchange authorization code: {}", e);
                Status::unauthenticated("Invalid authorization code")
            })?;

        if !user_info.verified_email {
            warn!(
                "Google login rejected: email {} not verified",
                user_info.email
            );
            return Err(Status::unauthenticated("Email address is not verified"));
        }

        let oauth_access_token =
            encrypt_sensitive_string(&user_info.access_token).map_err(|e| {
                error!("Failed to encrypt OAuth access token: {}", e);
                Status::internal("Failed to secure OAuth credentials")
            })?;
        let oauth_refresh_token = user_info
            .refresh_token
            .as_ref()
            .map(|t| encrypt_sensitive_string(t))
            .transpose()
            .map_err(|e| {
                error!("Failed to encrypt OAuth refresh token: {}", e);
                Status::internal("Failed to secure OAuth credentials")
            })?;
        let oauth_token_expiry = user_info.expires_in.map(|duration| {
            chrono::Utc::now() + chrono::Duration::seconds(duration.as_secs() as i64)
        });

        let existing_user_by_oauth = self
            .db
            .get_user_by_oauth_provider(OAuthProvider::Google, &user_info.id)
            .await;

        let user = match existing_user_by_oauth {
            Ok(user) => {
                if let Ok(oauth_creds) = self
                    .db
                    .get_oauth_credentials_by_provider_and_user(OAuthProvider::Google, user.id)
                    .await
                {
                    let update_request = be_remote_db::UpdateOAuthCredentials {
                        access_token: Some(oauth_access_token.clone()),
                        refresh_token: oauth_refresh_token.clone(),
                        access_token_expiry: oauth_token_expiry,
                        scope: Some("openid email profile".to_string()),
                    };

                    if let Err(e) = self
                        .db
                        .update_oauth_credentials(oauth_creds.id, update_request)
                        .await
                    {
                        warn!("Failed to update OAuth credentials: {}", e);
                    }
                }

                user
            }
            Err(_) => {
                let existing_user_by_email = self.db.get_user_by_email(&user_info.email).await;

                match existing_user_by_email {
                    Ok(user) => {
                        let oauth_request = CreateOAuthCredentials {
                            user_id: user.id,
                            provider: OAuthProvider::Google,
                            provider_user_id: user_info.id.clone(),
                            access_token: Some(oauth_access_token.clone()),
                            refresh_token: oauth_refresh_token.clone(),
                            access_token_expiry: oauth_token_expiry,
                            scope: Some("openid email profile".to_string()),
                        };

                        if let Err(e) = self.db.create_oauth_credentials(oauth_request).await {
                            error!("Failed to create OAuth credentials: {}", e);
                            return Err(Status::internal("Failed to link OAuth account"));
                        }

                        user
                    }
                    Err(_) => {
                        let username = user_info
                            .email
                            .split('@')
                            .next()
                            .unwrap_or(&user_info.name)
                            .to_string();

                        // Retry with suffix on username conflicts from concurrent signups
                        let new_user = {
                            let mut final_username = username.clone();
                            let mut counter = 0u32;
                            const MAX_RETRIES: u32 = 5;

                            loop {
                                let create_request = be_remote_db::NewUser {
                                    username: final_username.clone(),
                                    email: user_info.email.clone(),
                                    display_name: Some(user_info.name.clone()),
                                    password_hash: None,
                                };

                                match self.db.create_user(create_request).await {
                                    Ok(user) => break user,
                                    Err(be_remote_db::DbError::Duplicate { field, value })
                                        if value.contains("username") =>
                                    {
                                        counter += 1;
                                        if counter >= MAX_RETRIES {
                                            error!(
                                                "Failed to create unique username after {} attempts",
                                                MAX_RETRIES
                                            );
                                            return Err(Status::internal(
                                                "Failed to create user account",
                                            ));
                                        }
                                        final_username = format!("{}_{}", username, counter);
                                        info!(
                                            "Username conflict on '{}' ({}), retrying with '{}'",
                                            field, value, final_username
                                        );
                                    }
                                    Err(e) => {
                                        error!("Failed to create user from Google OAuth: {}", e);
                                        return Err(Status::internal(
                                            "Failed to create user account",
                                        ));
                                    }
                                }
                            }
                        };

                        let oauth_request = CreateOAuthCredentials {
                            user_id: new_user.id,
                            provider: OAuthProvider::Google,
                            provider_user_id: user_info.id.clone(),
                            access_token: Some(oauth_access_token.clone()),
                            refresh_token: oauth_refresh_token.clone(),
                            access_token_expiry: oauth_token_expiry,
                            scope: Some("openid email profile".to_string()),
                        };

                        if let Err(e) = self.db.create_oauth_credentials(oauth_request).await {
                            error!("Failed to create OAuth credentials: {}", e);
                            return Err(Status::internal("Failed to create OAuth credentials"));
                        }

                        new_user
                    }
                }
            }
        };

        if let Some(token) = creds.login_token {
            self.try_associate_login_token_with_user(&user, &token)
                .await;
        }

        let role = self.resolve_role(user.id).await;
        let (access_token, refresh_token) = self
            .generate_tokens(&user.id.to_string(), &user.username, &user.email, role)
            .await
            .map_err(|e| {
                error!("Failed to generate tokens: {}", e);
                Status::internal("Token generation error")
            })?;

        let response = TokenResponse {
            access_token,
            refresh_token,
            expires_in: self.jwt_config.access_token_expiry_hours * 3600,
        };

        Ok(Response::new(response))
    }

    async fn handle_github_login(
        &self,
        _creds: ThirdPartyCredentials,
    ) -> Result<Response<TokenResponse>, Status> {
        info!("Handling GitHub login");
        todo!()
    }
}

#[tonic::async_trait]
impl ProtoAuthService for AuthService {
    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<TokenResponse>, Status> {
        info!("Login request received");
        let req = request.into_inner();

        let credential = req.credential.ok_or_else(|| {
            warn!("Login request missing credentials");
            Status::invalid_argument("Missing credentials")
        })?;

        match credential {
            Credential::EmailPassword(creds) => self.handle_email_password_login(creds).await,
            Credential::ThirdParty(creds) => {
                let provider = Provider::try_from(creds.provider)
                    .map_err(|_| Status::invalid_argument("Invalid provider"))?;

                match provider {
                    Provider::Google => self.handle_google_login(creds).await,
                    Provider::Github => self.handle_github_login(creds).await,
                    Provider::Unspecified => {
                        warn!("Unspecified provider in OAuth request");
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
        info!("Register request received");

        let req = request.into_inner();

        let response = match self
            .register_user(&req.username, &req.email, &req.password, req.display_name)
            .await
        {
            Ok(response) => response,
            Err(e) => {
                error!("Registration failed: {}", e);
                return Err(Status::invalid_argument(format!(
                    "Registration failed: {}",
                    e
                )));
            }
        };

        Ok(Response::new(response))
    }

    async fn refresh_token(
        &self,
        request: Request<RefreshTokenRequest>,
    ) -> Result<Response<TokenResponse>, Status> {
        info!("Refresh token request received");
        let (_, refresh_token) = self
            .authenticate_request_refresh_token(&request)
            .map_err(|e| Status::unauthenticated(e.to_string()))?;

        let response = match self.refresh_access_token(&refresh_token).await {
            Ok(response) => response,
            Err(e) => {
                error!("Token refresh failed: {}", e);
                return Err(Status::unauthenticated(format!(
                    "Token refresh failed: {}",
                    e
                )));
            }
        };

        Ok(Response::new(response))
    }

    async fn get_third_party_auth_url(
        &self,
        request: Request<ThirdPartyAuthUrlRequest>,
    ) -> Result<Response<ThirdPartyAuthUrlResponse>, Status> {
        let req = request.into_inner();

        info!(
            "Third-party auth URL request received for provider: {:?}",
            req.provider
        );

        let provider = Provider::try_from(req.provider)
            .map_err(|_| Status::invalid_argument("Invalid provider"))?;

        let auth_url = match provider {
            Provider::Google => {
                info!("Generating Google OAuth URL");

                let google_client = self.google_oauth_client().await?;

                let state = self.generate_random_string(32).map_err(|e| {
                    error!("Failed to generate OAuth state: {}", e);
                    Status::internal("Failed to generate OAuth state")
                })?;
                let (_, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
                let pkce_verifier_secret = pkce_verifier.secret().to_string();
                let nonce = Nonce::new_random();
                let nonce_secret = nonce.secret().to_string();

                let google_config = oauth::google::GoogleOAuthConfig::from_env().map_err(|e| {
                    error!("Failed to load Google OAuth config: {}", e);
                    Status::internal("OAuth configuration error")
                })?;

                let expires_at = Utc::now() + Duration::minutes(10);

                let encrypted_pkce_verifier = encrypt_sensitive_string(&pkce_verifier_secret)
                    .map_err(|e| {
                        error!("Failed to encrypt PKCE verifier: {}", e);
                        Status::internal("Failed to secure OAuth state")
                    })?;

                let encrypted_nonce = encrypt_sensitive_string(&nonce_secret).map_err(|e| {
                    error!("Failed to encrypt nonce: {}", e);
                    Status::internal("Failed to secure OAuth state")
                })?;

                let oauth_state_request = CreateOAuthState {
                    state: state.clone(),
                    pkce_verifier: encrypted_pkce_verifier,
                    redirect_uri: google_config.redirect_uri.clone(),
                    ip_address: None,
                    expires_at,
                    nonce: encrypted_nonce,
                };

                self.db
                    .create_oauth_state(oauth_state_request)
                    .await
                    .map_err(|e| {
                        error!("Failed to store OAuth state: {}", e);
                        Status::internal("Failed to store OAuth state")
                    })?;

                google_client
                    .get_authorization_url_with_state_and_pkce(
                        &state,
                        &pkce_verifier_secret,
                        &nonce,
                    )
                    .map_err(|e| {
                        error!("Failed to generate Google OAuth URL: {}", e);
                        Status::internal("Failed to generate OAuth URL")
                    })?
            }
            Provider::Github => {
                warn!("GitHub OAuth not implemented yet");
                return Err(Status::unimplemented("GitHub OAuth not implemented"));
            }
            Provider::Unspecified => {
                warn!("Unspecified provider in OAuth request");
                return Err(Status::invalid_argument("Provider must be specified"));
            }
        };

        let response = ThirdPartyAuthUrlResponse { url: auth_url };
        Ok(Response::new(response))
    }

    async fn get_login_token(
        &self,
        _request: Request<()>,
    ) -> Result<Response<GetLoginTokenResponse>, Status> {
        Err(Status::unimplemented("get_login_token not implemented"))
    }

    async fn login_by_login_token(
        &self,
        request: Request<LoginByLoginTokenRequest>,
    ) -> Result<Response<TokenResponse>, Status> {
        info!("Login by login token request received");

        let req = request.into_inner();
        let code_verifier = req.token;

        if code_verifier.is_empty() {
            warn!("Login by login token request received with empty token");
            return Err(Status::invalid_argument("Login token is required"));
        }

        // Convert code_verifier -> code_challenge (PKCE S256), then hash for DB lookup
        let code_challenge = self.code_verifier_to_challenge(&code_verifier);
        let token_hash = self.hash_login_token(&code_challenge);

        // Atomic consume avoids TOCTOU race between get + check + consume
        let login_token = match self.db.consume_login_token(&token_hash).await {
            Ok(login_token) => login_token,
            Err(_) => {
                // Re-issue tokens for already-consumed tokens (idempotent retry)
                // since the caller proved identity via PKCE code_verifier.
                match self.db.get_login_token_by_hash_any(&token_hash).await {
                    Ok(login_token) if login_token.consumed => {
                        info!(
                            "Login token already consumed, re-issuing tokens for idempotent retry"
                        );
                        login_token
                    }
                    _ => {
                        warn!("Login token not found or expired");
                        return Err(Status::unauthenticated("Invalid or expired login token"));
                    }
                }
            }
        };

        let user = match self.db.get_user_by_id(login_token.user_id).await {
            Ok(user) => user,
            Err(_) => {
                error!("User not found for login token");
                return Err(Status::internal("User not found"));
            }
        };

        let role = self.resolve_role(user.id).await;
        let (access_token, refresh_token) = match self
            .generate_tokens(&user.id.to_string(), &user.username, &user.email, role)
            .await
        {
            Ok(tokens) => tokens,
            Err(e) => {
                error!("Token generation error: {}", e);
                return Err(Status::internal("Authentication error"));
            }
        };

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
    ) -> Result<Response<TokenResponse>, Status> {
        let login = creds.login.trim();
        let password = creds.password;

        if login.is_empty() || password.is_empty() {
            warn!("Login attempt with empty credentials");
            return Err(Status::invalid_argument(
                "Login and password cannot be empty",
            ));
        }

        let user = if login.contains('@') {
            self.db.get_user_by_email(login).await
        } else {
            self.db.get_user_by_username(login).await
        };

        let user = match user {
            Ok(user) => user,
            Err(_) => {
                warn!("Login failed: user not found for {}", login);
                return Err(Status::unauthenticated("Invalid credentials"));
            }
        };

        let password_creds = match self.db.get_password_credentials(user.id).await {
            Ok(creds) => creds,
            Err(_) => {
                warn!("Login failed: no password credentials for {}", login);
                return Err(Status::unauthenticated("Invalid credentials"));
            }
        };

        let password_valid = match self.verify_password(&password, &password_creds.password_hash) {
            Ok(valid) => valid,
            Err(e) => {
                error!("Password verification error: {}", e);
                return Err(Status::internal("Authentication error"));
            }
        };

        if !password_valid {
            warn!("Login failed: invalid password for {}", login);
            return Err(Status::unauthenticated("Invalid credentials"));
        }

        let role = self.resolve_role(user.id).await;
        let (access_token, refresh_token) = match self
            .generate_tokens(&user.id.to_string(), &user.username, &user.email, role)
            .await
        {
            Ok(tokens) => tokens,
            Err(e) => {
                error!("Token generation error: {}", e);
                return Err(Status::internal("Authentication error"));
            }
        };

        let response = TokenResponse {
            access_token,
            refresh_token,
            expires_in: self.jwt_config.access_token_expiry_hours * 3600,
        };

        Ok(Response::new(response))
    }
}
