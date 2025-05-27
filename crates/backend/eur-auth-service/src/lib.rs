//! The Eurora authentication service that provides gRPC endpoints for user authentication.

use anyhow::{Result, anyhow};
use bcrypt::{DEFAULT_COST, hash, verify};
use chrono::{Duration, Utc};
use eur_proto::proto_auth_service::proto_auth_service_server::ProtoAuthService;
use eur_proto::proto_auth_service::{
    EmailPasswordCredentials, LoginRequest, LoginResponse, login_request::Credential,
};
use eur_remote_db::{CreateUserRequest, DatabaseManager};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::{error, info, warn};
use uuid::Uuid;

/// JWT Claims structure
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    sub: String,        // Subject (user ID)
    username: String,   // Username
    email: String,      // Email
    exp: usize,         // Expiration time
    iat: usize,         // Issued at
    token_type: String, // "access" or "refresh"
}

/// Configuration for JWT tokens
#[derive(Debug, Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub access_token_expiry_hours: i64,
    pub refresh_token_expiry_days: i64,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: std::env::var("JWT_SECRET").unwrap_or_else(|_| "your-secret-key".to_string()),
            access_token_expiry_hours: 1,  // 1 hour
            refresh_token_expiry_days: 30, // 30 days
        }
    }
}

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
    fn hash_password(&self, password: &str) -> Result<(String, String)> {
        let salt = uuid::Uuid::new_v4().to_string();
        let salted_password = format!("{}{}", password, salt);
        let hashed = hash(salted_password, DEFAULT_COST)
            .map_err(|e| anyhow!("Failed to hash password: {}", e))?;
        Ok((hashed, salt))
    }

    /// Verify a password against a hash
    fn verify_password(&self, password: &str, salt: &str, hash: &str) -> Result<bool> {
        let salted_password = format!("{}{}", password, salt);
        verify(salted_password, hash).map_err(|e| anyhow!("Failed to verify password: {}", e))
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
        let decoding_key = DecodingKey::from_secret(self.jwt_config.secret.as_ref());
        let validation = Validation::new(Algorithm::HS256);

        let token_data = decode::<Claims>(token, &decoding_key, &validation)
            .map_err(|e| anyhow!("Invalid token: {}", e))?;

        Ok(token_data.claims)
    }

    /// Register a new user (not in proto yet, but implementing for completeness)
    pub async fn register_user(
        &self,
        username: &str,
        email: &str,
        password: &str,
        display_name: Option<String>,
    ) -> Result<LoginResponse> {
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

        if self.db.user_exists_by_email(email).await.unwrap_or(false) {
            return Err(anyhow!("Email already exists"));
        }

        // Hash the password
        let (password_hash, password_salt) = self.hash_password(password)?;

        // Create user request
        let create_request = CreateUserRequest {
            username: username.to_string(),
            email: email.to_string(),
            display_name,
            password_hash,
            password_salt,
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

        Ok(LoginResponse {
            access_token,
            refresh_token,
            expires_in: self.jwt_config.access_token_expiry_hours * 3600, // Convert to seconds
        })
    }

    /// Refresh an access token using a refresh token
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<LoginResponse> {
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

        Ok(LoginResponse {
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
    ) -> Result<Response<LoginResponse>, Status> {
        let req = request.into_inner();

        info!("Login request received");

        // Extract credentials from the request
        let credential = req.credential.ok_or_else(|| {
            warn!("Login request missing credentials");
            Status::invalid_argument("Missing credentials")
        })?;

        match credential {
            Credential::EmailPassword(creds) => self.handle_email_password_login(creds).await,
            Credential::ThirdParty(_) => {
                warn!("Third-party authentication not implemented");
                Err(Status::unimplemented(
                    "Third-party authentication not implemented",
                ))
            }
        }
    }
}

impl AuthService {
    /// Handle email/password login
    async fn handle_email_password_login(
        &self,
        creds: EmailPasswordCredentials,
    ) -> Result<Response<LoginResponse>, Status> {
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
        let password_valid = match self.verify_password(
            &password,
            &password_creds.password_salt,
            &password_creds.password_hash,
        ) {
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
                    error!("Token generation error: {}", e);
                    return Err(Status::internal("Authentication error"));
                }
            };

        info!("Login successful for user: {}", user.username);

        let response = LoginResponse {
            access_token,
            refresh_token,
            expires_in: self.jwt_config.access_token_expiry_hours * 3600,
        };

        Ok(Response::new(response))
    }
}
