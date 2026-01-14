//! The Eurora authentication service that provides gRPC endpoints for user authentication.

use anyhow::{Result, anyhow};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use bcrypt::{DEFAULT_COST, hash, verify};
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, Header, encode};
use std::sync::Arc;
// Re-export shared types for convenience
pub use auth_core::Claims;
use be_auth_grpc::JwtConfig;
use be_remote_db::{
    CreateLoginTokenRequest, CreateOAuthCredentialsRequest, CreateOAuthStateRequest,
    CreateRefreshTokenRequest, CreateUserRequest, DatabaseManager,
};
use oauth2::TokenResponse as OAuth2TokenResponse;
use proto_gen::auth::{
    EmailPasswordCredentials, GetLoginTokenResponse, LoginByLoginTokenRequest, LoginRequest,
    Provider, RefreshTokenRequest, RegisterRequest, ThirdPartyAuthUrlRequest,
    ThirdPartyAuthUrlResponse, ThirdPartyCredentials, TokenResponse, login_request::Credential,
    proto_auth_service_server::ProtoAuthService,
};
use rand::{TryRngCore, rngs::OsRng};
use sha2::{Digest, Sha256};
use tonic::{Request, Response, Status};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

pub mod oauth;

use oauth::google::create_google_oauth_client;

/// The main authentication service
pub struct AuthService {
    db: Arc<DatabaseManager>,
    jwt_config: JwtConfig,
    #[allow(dead_code)]
    desktop_login_url: String,
}

impl AuthService {
    /// Create a new AuthService instance
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
        }
    }

    /// Extract and validate JWT token from request metadata
    pub fn authenticate_request_access_token<T>(&self, request: &Request<T>) -> Result<Claims> {
        // Get authorization header
        let auth_header = request
            .metadata()
            .get("authorization")
            .ok_or_else(|| anyhow!("Missing authorization header"))?;

        // Convert to string
        let auth_str = auth_header
            .to_str()
            .map_err(|_| anyhow!("Invalid authorization header format"))?;

        // Extract Bearer token
        if !auth_str.starts_with("Bearer ") {
            return Err(anyhow!("Authorization header must start with 'Bearer '"));
        }

        let token = &auth_str[7..]; // Remove "Bearer " prefix

        // Validate access token using shared function
        self.jwt_config.validate_access_token(token)
    }

    /// Extract and validate JWT token from request metadata
    pub fn authenticate_request_refresh_token<T>(
        &self,
        request: &Request<T>,
    ) -> Result<(Claims, String)> {
        // Get authorization header
        let auth_header = request
            .metadata()
            .get("authorization")
            .ok_or_else(|| anyhow!("Missing authorization header"))?;

        // Convert to string
        let auth_str = auth_header
            .to_str()
            .map_err(|_| anyhow!("Invalid authorization header format"))?;

        // Extract Bearer token
        if !auth_str.starts_with("Bearer ") {
            return Err(anyhow!("Authorization header must start with 'Bearer '"));
        }

        let token = &auth_str[7..]; // Remove "Bearer " prefix

        // Validate refresh token using shared function
        let claims = self.jwt_config.validate_refresh_token(token)?;

        Ok((claims, token.to_string()))
    }

    /// Hash a password using bcrypt
    fn hash_password(&self, password: &str) -> Result<String> {
        let hashed =
            hash(password, DEFAULT_COST).map_err(|e| anyhow!("Failed to hash password: {}", e))?;
        Ok(hashed)
    }

    /// Verify a password against a hash
    fn verify_password(&self, password: &str, hash: &str) -> Result<bool> {
        verify(password, hash).map_err(|e| anyhow!("Failed to verify password: {}", e))
    }

    /// Hash a refresh token for secure storage
    fn hash_refresh_token(&self, token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Generate JWT tokens (access and refresh)
    async fn generate_tokens(
        &self,
        user_id: &str,
        username: &str,
        email: &str,
    ) -> Result<(String, String)> {
        let now = Utc::now();
        let access_exp = now + Duration::hours(self.jwt_config.access_token_expiry_hours);
        let refresh_exp = now + Duration::days(self.jwt_config.refresh_token_expiry_days);

        // Access token claims
        let access_claims = Claims {
            sub: user_id.to_string(),
            username: username.to_string(),
            email: email.to_string(),
            exp: access_exp.timestamp(),
            iat: now.timestamp(),
            token_type: "access".to_string(),
        };

        // Refresh token claims
        let refresh_claims = Claims {
            sub: user_id.to_string(),
            username: username.to_string(),
            email: email.to_string(),
            exp: refresh_exp.timestamp(),
            iat: now.timestamp(),
            token_type: "refresh".to_string(),
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

        // Store refresh token in database
        let user_uuid =
            Uuid::parse_str(user_id).map_err(|e| anyhow!("Invalid user ID format: {}", e))?;

        let token_hash = self.hash_refresh_token(&refresh_token);
        let refresh_request = CreateRefreshTokenRequest {
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
        OsRng.try_fill_bytes(&mut bytes).map_err(|e| {
            error!("Failed to generate random bytes: {}", e);
            Status::internal("Failed to generate random bytes")
        })?;

        let mut hex = hex::encode(bytes);
        hex.truncate(length); // exact length
        Ok(hex)
    }

    /// Try to associate any pending login tokens with the user
    /// This looks for unused login tokens and associates them with the user
    async fn try_associate_login_token_with_user(&self, user: &be_remote_db::User, token: &str) {
        debug!(
            "Attempting to associate login token with user: {}",
            user.username
        );
        let create_request = CreateLoginTokenRequest {
            token: token.to_string(),
            expires_at: Utc::now() + Duration::minutes(20),
            user_id: user.id,
        };

        match self.db.create_login_token(create_request).await {
            Ok(_) => {
                debug!(
                    "Successfully associated login token '{}' with user: {}",
                    token, user.username
                );
            }
            Err(e) => {
                error!("Failed to update login token with user_id: {}", e);
            }
        }
    }

    /// Register a new user (not in proto yet, but implementing for completeness)
    pub async fn register_user(
        &self,
        username: &str,
        email: &str,
        password: &str,
        display_name: Option<String>,
    ) -> Result<TokenResponse> {
        debug!("Attempting to register user: {}", username);

        // Check if user already exists
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

        // Hash the password
        let password_hash = self.hash_password(password)?;

        // Create user request
        let create_request = CreateUserRequest {
            username: username.to_string(),
            email: email.to_string(),
            display_name,
            password_hash,
        };

        // Create user in database
        let user = self
            .db
            .create_user(create_request)
            .await
            .map_err(|e| anyhow!("Failed to create user: {}", e))?;

        debug!("User registered successfully: {}", user.username);

        // Generate tokens
        let (access_token, refresh_token) = self
            .generate_tokens(&user.id.to_string(), &user.username, &user.email)
            .await?;

        Ok(TokenResponse {
            access_token,
            refresh_token,
            expires_in: self.jwt_config.access_token_expiry_hours * 3600, // Convert to seconds
        })
    }

    /// Refresh an access token using a refresh token
    pub async fn refresh_access_token(&self, refresh_token: &str) -> Result<TokenResponse> {
        debug!("Attempting to refresh token");

        // Hash the provided refresh token to look it up in the database
        let token_hash = self.hash_refresh_token(refresh_token);

        // Get the refresh token from database (this also validates it's not expired/revoked)
        let stored_token = self
            .db
            .get_refresh_token_by_hash(&token_hash)
            .await
            .map_err(|e| anyhow!("Invalid or expired refresh token: {}", e))?;

        // Get user from database to ensure they still exist
        let user = self
            .db
            .get_user_by_id(stored_token.user_id)
            .await
            .map_err(|e| anyhow!("User not found: {}", e))?;

        // Revoke the old refresh token
        self.db
            .revoke_refresh_token(&token_hash)
            .await
            .map_err(|e| anyhow!("Failed to revoke old refresh token: {}", e))?;

        // Generate new tokens
        let (access_token, new_refresh_token) = self
            .generate_tokens(&user.id.to_string(), &user.username, &user.email)
            .await?;

        debug!("Token refreshed successfully for user: {}", user.username);

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
        debug!("Handling Google login");
        // Extract code and state from credentials
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

        // Validate and consume the OAuth state
        let _oauth_state = match self.db.get_oauth_state_by_state(state).await {
            Ok(oauth_state) => oauth_state,
            Err(_) => {
                warn!("Invalid or expired OAuth state: {}", state);
                return Err(Status::invalid_argument(
                    "Invalid or expired state parameter",
                ));
            }
        };

        // Consume the state to prevent replay attacks
        if let Err(e) = self.db.consume_oauth_state(state).await {
            error!("Failed to consume OAuth state: {}", e);
            // Continue anyway, as the state was valid
        }

        debug!("OAuth state validated successfully for state: {}", state);

        // Extract PKCE verifier for token exchange
        // let pkce_verifier = oauth_state.pkce_verifier.clone();

        debug!("Exchanging authorization code for access token");

        // Create OAuth client for token exchange
        let google_config = oauth::google::GoogleOAuthConfig::from_env().map_err(|e| {
            error!("Failed to load Google OAuth config: {}", e);
            Status::internal("OAuth configuration error")
        })?;

        let google_client_id = oauth2::ClientId::new(google_config.client_id.clone());
        let google_client_secret = oauth2::ClientSecret::new(google_config.client_secret.clone());

        let auth_url =
            oauth2::AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
                .map_err(|e| {
                    error!("Invalid authorization endpoint URL: {}", e);
                    Status::internal("OAuth configuration error")
                })?;

        let token_url =
            oauth2::TokenUrl::new("https://www.googleapis.com/oauth2/v3/token".to_string())
                .map_err(|e| {
                    error!("Invalid token endpoint URL: {}", e);
                    Status::internal("OAuth configuration error")
                })?;

        let redirect_url =
            oauth2::RedirectUrl::new(google_config.redirect_uri.clone()).map_err(|e| {
                error!("Invalid redirect URL: {}", e);
                Status::internal("OAuth configuration error")
            })?;

        let client = oauth2::basic::BasicClient::new(google_client_id)
            .set_client_secret(google_client_secret)
            .set_auth_uri(auth_url)
            .set_token_uri(token_url)
            .set_redirect_uri(redirect_url);

        // Exchange authorization code for access token
        let http_client = reqwest::ClientBuilder::new()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| {
                error!("Failed to build HTTP client: {}", e);
                Status::internal("HTTP client error")
            })?;

        let token_result = client
            .exchange_code(oauth2::AuthorizationCode::new(code.to_string()))
            // .set_pkce_verifier(oauth2::PkceCodeVerifier::new(pkce_verifier))
            .request_async(&http_client)
            .await
            .map_err(|e| {
                error!("Failed to exchange authorization code: {}", e);
                Status::unauthenticated("Invalid authorization code")
            })?;

        let access_token = token_result.access_token().secret();
        debug!("Successfully obtained access token from Google");

        // Get user info from Google
        let user_info = self.get_google_user_info(access_token).await.map_err(|e| {
            error!("Failed to get user info from Google: {}", e);
            Status::internal("Failed to retrieve user information")
        })?;

        debug!("Retrieved user info for: {}", user_info.email);

        // Check if user exists by OAuth provider first
        let existing_user_by_oauth = self
            .db
            .get_user_by_oauth_provider("google", &user_info.id)
            .await;

        let user = match existing_user_by_oauth {
            Ok(user) => {
                debug!("Found existing user by OAuth: {}", user.username);

                // Update OAuth credentials with new tokens
                if let Ok(oauth_creds) = self
                    .db
                    .get_oauth_credentials_by_provider_and_user("google", user.id)
                    .await
                {
                    let update_request = be_remote_db::UpdateOAuthCredentialsRequest {
                        access_token: Some(access_token.as_bytes().to_vec()),
                        refresh_token: token_result
                            .refresh_token()
                            .map(|t| t.secret().as_bytes().to_vec()),
                        access_token_expiry: token_result.expires_in().map(|duration| {
                            chrono::Utc::now()
                                + chrono::Duration::seconds(duration.as_secs() as i64)
                        }),
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
                // Check if user exists by email
                let existing_user_by_email = self.db.get_user_by_email(&user_info.email).await;

                match existing_user_by_email {
                    Ok(user) => {
                        debug!(
                            "Found existing user by email, linking OAuth: {}",
                            user.username
                        );

                        // Link OAuth account to existing user
                        let oauth_request = CreateOAuthCredentialsRequest {
                            user_id: user.id,
                            provider: "google".to_string(),
                            provider_user_id: user_info.id.clone(),
                            access_token: Some(access_token.as_bytes().to_vec()),
                            refresh_token: token_result
                                .refresh_token()
                                .map(|t| t.secret().as_bytes().to_vec()),
                            access_token_expiry: token_result.expires_in().map(|duration| {
                                chrono::Utc::now()
                                    + chrono::Duration::seconds(duration.as_secs() as i64)
                            }),
                            scope: Some("openid email profile".to_string()),
                        };

                        if let Err(e) = self.db.create_oauth_credentials(oauth_request).await {
                            error!("Failed to create OAuth credentials: {}", e);
                            return Err(Status::internal("Failed to link OAuth account"));
                        }

                        user
                    }
                    Err(_) => {
                        // If token is not present throw error
                        let login_token = creds.login_token.clone();
                        let challenge_method = creds.challenge_method.clone();

                        if login_token.is_none() {
                            error!("Login token is missing");
                            return Err(Status::unauthenticated("Login token is missing"));
                        }
                        if challenge_method.is_none() {
                            error!("Challenge method is missing");
                            return Err(Status::unauthenticated("Challenge method is missing"));
                        }

                        // Create new user from Google info
                        debug!("Creating new user from Google OAuth: {}", user_info.email);

                        // Generate username from email (before @ symbol) or use name
                        let username = user_info
                            .email
                            .split('@')
                            .next()
                            .unwrap_or(&user_info.name)
                            .to_string();

                        // Ensure username is unique by appending numbers if needed
                        let mut final_username = username.clone();
                        let mut counter = 1;
                        while self
                            .db
                            .user_exists_by_username(&final_username)
                            .await
                            .unwrap_or(false)
                        {
                            final_username = format!("{}_{}", username, counter);
                            counter += 1;
                        }

                        let create_request = be_remote_db::CreateUserRequest {
                            username: final_username,
                            email: user_info.email.clone(),
                            display_name: Some(user_info.name.clone()),
                            password_hash: String::new(), // No password for OAuth users
                        };

                        let new_user = self.db.create_user(create_request).await.map_err(|e| {
                            error!("Failed to create user from Google OAuth: {}", e);
                            Status::internal("Failed to create user account")
                        })?;

                        // Create OAuth credentials for the new user
                        let oauth_request = CreateOAuthCredentialsRequest {
                            user_id: new_user.id,
                            provider: "google".to_string(),
                            provider_user_id: user_info.id.clone(),
                            access_token: Some(access_token.as_bytes().to_vec()),
                            refresh_token: token_result
                                .refresh_token()
                                .map(|t| t.secret().as_bytes().to_vec()),
                            access_token_expiry: token_result.expires_in().map(|duration| {
                                chrono::Utc::now()
                                    + chrono::Duration::seconds(duration.as_secs() as i64)
                            }),
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

        // Generate JWT tokens
        let (access_token, refresh_token) = self
            .generate_tokens(&user.id.to_string(), &user.username, &user.email)
            .await
            .map_err(|e| {
                error!("Failed to generate tokens: {}", e);
                Status::internal("Token generation error")
            })?;

        debug!("Google OAuth login successful for user: {}", user.username);

        let response = TokenResponse {
            access_token,
            refresh_token,
            expires_in: self.jwt_config.access_token_expiry_hours * 3600,
        };

        Ok(Response::new(response))
    }

    /// Get user info from Google using access token
    async fn get_google_user_info(
        &self,
        access_token: &str,
    ) -> Result<oauth::google::GoogleUserInfo> {
        debug!("Fetching user info from Google");

        let client = reqwest::Client::new();
        let response = client
            .get("https://www.googleapis.com/oauth2/v2/userinfo")
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to fetch user info: {}", e))?;

        if !response.status().is_success() {
            error!("Google API returned error: {}", response.status());
            return Err(anyhow!("Failed to fetch user info from Google"));
        }

        let user_info: oauth::google::GoogleUserInfo = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse user info response: {}", e))?;

        debug!("Successfully fetched user info for: {}", user_info.email);

        Ok(user_info)
    }

    async fn handle_github_login(
        &self,
        _creds: ThirdPartyCredentials,
    ) -> Result<Response<TokenResponse>, Status> {
        debug!("Handling GitHub login");
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

        // Extract credentials from the request
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

        // Call the existing register_user method
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

        // Call the existing refresh_access_token method
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
                debug!("Generating Google OAuth URL");

                let google_client = create_google_oauth_client().map_err(|e| {
                    error!("Failed to create Google OAuth client: {}", e);
                    Status::internal("Failed to initialize OAuth client")
                })?;

                // Generate random state and PKCE verifier
                let state = self.generate_random_string(32).unwrap();
                let pkce_verifier = self.generate_random_string(64).unwrap();

                // Get redirect URI from Google client config
                let google_config = oauth::google::GoogleOAuthConfig::from_env().map_err(|e| {
                    error!("Failed to load Google OAuth config: {}", e);
                    Status::internal("OAuth configuration error")
                })?;

                // Store OAuth state in database
                let expires_at = Utc::now() + Duration::minutes(10); // 10 minute expiration
                let oauth_state_request = CreateOAuthStateRequest {
                    state: state.clone(),
                    pkce_verifier: pkce_verifier.clone(),
                    redirect_uri: google_config.redirect_uri.clone(),
                    ip_address: None, // Could be extracted from request metadata if needed
                    expires_at,
                };

                self.db
                    .create_oauth_state(oauth_state_request)
                    .await
                    .map_err(|e| {
                        error!("Failed to store OAuth state: {}", e);
                        Status::internal("Failed to store OAuth state")
                    })?;

                let url = google_client
                    .get_authorization_url_with_state(&state)
                    .map_err(|e| {
                        error!("Failed to generate Google OAuth URL: {}", e);
                        Status::internal("Failed to generate OAuth URL")
                    })?;

                debug!(
                    "Generated Google OAuth URL successfully with state: {}",
                    state
                );
                url
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
        let token = req.token;

        if token.is_empty() {
            warn!("Login by login token request received with empty token");
            return Err(Status::invalid_argument("Login token is required"));
        }

        // Hash the token
        let mut hasher = Sha256::new();
        hasher.update(token);
        let code_challenge = hasher.finalize();
        let token = URL_SAFE_NO_PAD.encode(code_challenge);

        // Get the login token from database
        let login_token = match self.db.get_login_token_by_token(&token).await {
            Ok(login_token) => login_token,
            Err(_) => {
                warn!("Login token not found or expired: {}", token);
                return Err(Status::unauthenticated("Invalid or expired login token"));
            }
        };

        // Check if user_id is empty (token not associated with user)
        if login_token.user_id.is_none() {
            debug!(
                "Login token not yet associated with user, client should keep polling: {}",
                token
            );
            return Err(Status::unavailable(
                "Login token pending user authentication",
            ));
        }

        // Check if token is already consumed
        if login_token.consumed {
            warn!("Login token already consumed: {}", token);
            return Err(Status::unauthenticated("Invalid login token"));
        }

        // Get the user associated with the token
        let user_id = login_token.user_id.unwrap();
        let user = match self.db.get_user_by_id(user_id).await {
            Ok(user) => user,
            Err(_) => {
                error!("User not found for login token: {}", token);
                return Err(Status::internal("User not found"));
            }
        };

        // Mark the token as consumed
        if let Err(e) = self.db.consume_login_token(&token).await {
            error!("Failed to consume login token: {}", e);
            return Err(Status::internal("Failed to process login token"));
        }

        // Generate JWT tokens for the user
        let (access_token, refresh_token) = match self
            .generate_tokens(&user.id.to_string(), &user.username, &user.email)
            .await
        {
            Ok(tokens) => tokens,
            Err(e) => {
                error!("Token generation error: {}", e);
                return Err(Status::internal("Authentication error"));
            }
        };

        debug!(
            "Login by login token successful for user: {}",
            user.username
        );

        let response = TokenResponse {
            access_token,
            refresh_token,
            expires_in: self.jwt_config.access_token_expiry_hours * 3600,
        };

        Ok(Response::new(response))
    }
}

impl AuthService {
    /// Handle email/password login
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

        debug!("Attempting login for: {}", login);

        // Try to find user by username or email
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

        // Get password credentials
        let password_creds = match self.db.get_password_credentials(user.id).await {
            Ok(creds) => creds,
            Err(e) => {
                error!("Failed to get password credentials: {}", e);
                return Err(Status::internal("Authentication error"));
            }
        };

        // Verify password
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

        // Generate tokens
        let (access_token, refresh_token) = match self
            .generate_tokens(&user.id.to_string(), &user.username, &user.email)
            .await
        {
            Ok(tokens) => tokens,
            Err(e) => {
                error!("Token generation error: {}", e);
                return Err(Status::internal("Authentication error"));
            }
        };

        debug!("Login successful for user: {}", user.username);

        let response = TokenResponse {
            access_token,
            refresh_token,
            expires_in: self.jwt_config.access_token_expiry_hours * 3600,
        };

        Ok(Response::new(response))
    }
}
