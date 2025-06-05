//! The Eurora authentication service that provides gRPC endpoints for user authentication.

use anyhow::{Result, anyhow};
use bcrypt::{DEFAULT_COST, hash, verify};
use chrono::{Duration, Utc};
use eur_proto::proto_auth_service::proto_auth_service_server::ProtoAuthService;
use eur_proto::proto_auth_service::{
    EmailPasswordCredentials, LoginRequest, Provider, RefreshTokenRequest, RegisterRequest,
    ThirdPartyAuthUrlRequest, ThirdPartyAuthUrlResponse, ThirdPartyCredentials, TokenResponse,
    login_request::Credential,
};
use eur_remote_db::{CreateUserRequest, DatabaseManager};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::{error, info, warn};
use uuid::Uuid;

// Re-export shared types for convenience
pub use eur_auth::{Claims, JwtConfig};
pub mod oauth;
use oauth::google::create_google_oauth_client;
/// The main authentication service
#[derive(Debug)]
pub struct AuthService {
    db: Arc<DatabaseManager>,
    jwt_config: JwtConfig,
}

impl AuthService {
    /// Create a new AuthService instance
    pub fn new(db: Arc<DatabaseManager>, jwt_config: Option<JwtConfig>) -> Self {
        Self {
            db,
            jwt_config: jwt_config.unwrap_or_default(),
        }
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

    /// Generate JWT tokens (access and refresh)
    fn generate_tokens(
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
            exp: access_exp.timestamp() as usize,
            iat: now.timestamp() as usize,
            token_type: "access".to_string(),
        };

        // Refresh token claims
        let refresh_claims = Claims {
            sub: user_id.to_string(),
            username: username.to_string(),
            email: email.to_string(),
            exp: refresh_exp.timestamp() as usize,
            iat: now.timestamp() as usize,
            token_type: "refresh".to_string(),
        };

        let header = Header::new(Algorithm::HS256);
        let encoding_key = EncodingKey::from_secret(self.jwt_config.secret.as_ref());

        let access_token = encode(&header, &access_claims, &encoding_key)
            .map_err(|e| anyhow!("Failed to generate access token: {}", e))?;

        let refresh_token = encode(&header, &refresh_claims, &encoding_key)
            .map_err(|e| anyhow!("Failed to generate refresh token: {}", e))?;

        Ok((access_token, refresh_token))
    }

    /// Validate and decode a JWT token
    pub fn validate_token(&self, token: &str) -> Result<Claims> {
        eur_auth::validate_token(token, &self.jwt_config)
    }

    /// Register a new user (not in proto yet, but implementing for completeness)
    pub async fn register_user(
        &self,
        username: &str,
        email: &str,
        password: &str,
        display_name: Option<String>,
    ) -> Result<TokenResponse> {
        info!("Attempting to register user: {}", username);

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

        info!("User registered successfully: {}", user.username);

        // Generate tokens
        let (access_token, refresh_token) =
            self.generate_tokens(&user.id.to_string(), &user.username, &user.email)?;

        Ok(TokenResponse {
            access_token,
            refresh_token,
            expires_in: self.jwt_config.access_token_expiry_hours * 3600, // Convert to seconds
        })
    }

    /// Refresh an access token using a refresh token
    pub async fn refresh_access_token(&self, refresh_token: &str) -> Result<TokenResponse> {
        info!("Attempting to refresh token");

        // Validate the refresh token
        let claims = self.validate_token(refresh_token)?;

        // Ensure it's a refresh token
        if claims.token_type != "refresh" {
            return Err(anyhow!("Invalid token type"));
        }

        // Get user from database to ensure they still exist
        let user_id =
            Uuid::parse_str(&claims.sub).map_err(|e| anyhow!("Invalid user ID in token: {}", e))?;

        let user = self
            .db
            .get_user_by_id(user_id)
            .await
            .map_err(|e| anyhow!("User not found: {}", e))?;

        // Generate new tokens
        let (access_token, new_refresh_token) =
            self.generate_tokens(&user.id.to_string(), &user.username, &user.email)?;

        info!("Token refreshed successfully for user: {}", user.username);

        Ok(TokenResponse {
            access_token,
            refresh_token: new_refresh_token,
            expires_in: self.jwt_config.access_token_expiry_hours * 3600,
        })
    }
}

#[tonic::async_trait]
impl ProtoAuthService for AuthService {
    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<TokenResponse>, Status> {
        let req = request.into_inner();

        info!("Login request received");
        // eprintln!("Request: {:#?}", request);

        // return Err(Status::unimplemented("Login not implemented"));

        // Extract credentials from the request
        let credential = req.credential.ok_or_else(|| {
            warn!("Login request missing credentials");
            Status::invalid_argument("Missing credentials")
        })?;

        match credential {
            Credential::EmailPassword(creds) => self.handle_email_password_login(creds).await,
            Credential::ThirdParty(creds) => self.handle_third_party_login(creds).await,
        }
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

                let google_client = create_google_oauth_client().map_err(|e| {
                    error!("Failed to create Google OAuth client: {}", e);
                    Status::internal("Failed to initialize OAuth client")
                })?;

                let (url, _csrf_token) = google_client.get_authorization_url().map_err(|e| {
                    error!("Failed to generate Google OAuth URL: {}", e);
                    Status::internal("Failed to generate OAuth URL")
                })?;

                // TODO: Store CSRF token for validation during callback
                // This should be stored in a cache/database with expiration
                info!("Generated Google OAuth URL successfully");
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

    async fn register(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<Response<TokenResponse>, Status> {
        let req = request.into_inner();

        info!("Register request received for user: {}", req.username);

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
        let req = request.into_inner();

        info!("Refresh token request received");

        // Call the existing refresh_access_token method
        let response = match self.refresh_access_token(&req.refresh_token).await {
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

        info!("Attempting login for: {}", login);

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
        let (access_token, refresh_token) =
            match self.generate_tokens(&user.id.to_string(), &user.username, &user.email) {
                Ok(tokens) => tokens,
                Err(e) => {
                    /// Handle third-party login (Google OAuth)
                    async fn handle_third_party_login(
                        &self,
                        creds: ThirdPartyCredentials,
                    ) -> Result<Response<TokenResponse>, Status> {
                        info!(
                            "Third-party login request received for provider: {:?}",
                            creds.provider
                        );

                        let provider = Provider::try_from(creds.provider)
                            .map_err(|_| Status::invalid_argument("Invalid provider"))?;

                        match provider {
                            Provider::Google => {
                                info!("Processing Google OAuth login");

                                // Validate the ID token and get user info
                                let user_info =
                                    match self.validate_google_id_token(&creds.id_token).await {
                                        Ok(info) => info,
                                        Err(e) => {
                                            error!("Failed to validate Google ID token: {}", e);
                                            return Err(Status::unauthenticated(
                                                "Invalid Google credentials",
                                            ));
                                        }
                                    };

                                info!("Google user info validated for: {}", user_info.email);

                                // Check if user exists in our database
                                let user = match self.db.get_user_by_email(&user_info.email).await {
                                    Ok(user) => {
                                        info!("Existing user found: {}", user.username);
                                        user
                                    }
                                    Err(_) => {
                                        // User doesn't exist, create a new one
                                        info!(
                                            "Creating new user from Google OAuth: {}",
                                            user_info.email
                                        );

                                        // Generate a username from email (before @)
                                        let username = user_info
                                            .email
                                            .split('@')
                                            .next()
                                            .unwrap_or("user")
                                            .to_string();

                                        // Create user request (no password for OAuth users)
                                        let create_request = CreateUserRequest {
                                            username: username.clone(),
                                            email: user_info.email.clone(),
                                            display_name: Some(user_info.name.clone()),
                                            password_hash: String::new(), // Empty password hash for OAuth users
                                        };

                                        match self.db.create_user(create_request).await {
                                            Ok(user) => {
                                                info!(
                                                    "Successfully created new user: {}",
                                                    user.username
                                                );
                                                user
                                            }
                                            Err(e) => {
                                                error!("Failed to create user: {}", e);
                                                return Err(Status::internal(
                                                    "Failed to create user account",
                                                ));
                                            }
                                        }
                                    }
                                };

                                // Generate tokens for the user
                                let (access_token, refresh_token) = match self.generate_tokens(
                                    &user.id.to_string(),
                                    &user.username,
                                    &user.email,
                                ) {
                                    Ok(tokens) => tokens,
                                    Err(e) => {
                                        error!("Token generation error: {}", e);
                                        return Err(Status::internal("Authentication error"));
                                    }
                                };

                                info!("Third-party login successful for user: {}", user.username);

                                let response = TokenResponse {
                                    access_token,
                                    refresh_token,
                                    expires_in: self.jwt_config.access_token_expiry_hours * 3600,
                                };

                                Ok(Response::new(response))
                            }
                            Provider::Github => {
                                warn!("GitHub OAuth not implemented yet");
                                Err(Status::unimplemented("GitHub OAuth not implemented"))
                            }
                            Provider::ProviderUnspecified => {
                                warn!("Unspecified provider in OAuth request");
                                Err(Status::invalid_argument("Provider must be specified"))
                            }
                        }
                    }

                    /// Validate Google ID token and extract user info
                    async fn validate_google_id_token(
                        &self,
                        id_token: &str,
                    ) -> Result<oauth::google::GoogleUserInfo> {
                        // In this implementation, we're treating the access token as the ID token
                        // In a real implementation, you would validate a proper JWT ID token

                        // For now, we'll use the token to fetch user info from Google
                        let client = reqwest::Client::new();
                        let response = client
                            .get("https://www.googleapis.com/oauth2/v2/userinfo")
                            .bearer_auth(id_token)
                            .send()
                            .await
                            .map_err(|e| anyhow!("Failed to fetch user info: {}", e))?;

                        if !response.status().is_success() {
                            error!("Google API returned error: {}", response.status());
                            return Err(anyhow!("Failed to validate Google credentials"));
                        }

                        let user_info: oauth::google::GoogleUserInfo = response
                            .json()
                            .await
                            .map_err(|e| anyhow!("Failed to parse user info response: {}", e))?;

                        if !user_info.verified_email {
                            return Err(anyhow!("Google email not verified"));
                        }

                        Ok(user_info)
                    }
                    error!("Token generation error: {}", e);
                    return Err(Status::internal("Authentication error"));
                }
            };

        info!("Login successful for user: {}", user.username);

        let response = TokenResponse {
            access_token,
            refresh_token,
            expires_in: self.jwt_config.access_token_expiry_hours * 3600,
        };

        Ok(Response::new(response))
    }
}
