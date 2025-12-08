//! Basic usage example for the Eurora Auth Service
//!
//! This example demonstrates how to:
//! 1. Set up the auth service with a database connection
//! 2. Register a new user
//! 3. Login with email/password
//! 4. Refresh tokens
//! 5. Validate tokens

use std::sync::Arc;

use anyhow::Result;
use euro_auth_service::{AuthService, JwtConfig};
use euro_remote_db::DatabaseManager;

#[tokio::main]
async fn main() -> Result<()> {
    // Note: This example requires a PostgreSQL database to be running
    // You would typically get this from environment variables
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/eurora".to_string());

    eprintln!("Connecting to database...");

    // Initialize database manager
    let db_manager = match DatabaseManager::new(&database_url).await {
        Ok(db) => Arc::new(db),
        Err(e) => {
            eprintln!("Failed to connect to database: {}", e);
            eprintln!("Make sure PostgreSQL is running and DATABASE_URL is set correctly");
            return Ok(());
        }
    };

    // Configure JWT settings
    let jwt_config = JwtConfig {
        secret: "your-super-secret-jwt-key".to_string(),
        access_token_expiry_hours: 1,
        refresh_token_expiry_days: 30,
        approved_emails: vec![],
    };

    // Create auth service
    let auth_service = AuthService::new(db_manager, Some(jwt_config));

    eprintln!("Auth service initialized successfully!");

    // Example 1: Register a new user
    eprintln!("\n=== Registering a new user ===");
    let register_result = auth_service
        .register_user(
            "john_doe",
            "john@example.com",
            "secure_password123",
            Some("John Doe".to_string()),
        )
        .await;

    match register_result {
        Ok(login_response) => {
            eprintln!("✅ User registered successfully!");
            eprintln!("Access token: {}...", &login_response.access_token[..20]);
            eprintln!("Refresh token: {}...", &login_response.refresh_token[..20]);
            eprintln!("Expires in: {} seconds", login_response.expires_in);
        }
        Err(e) => {
            eprintln!("❌ Registration failed: {}", e);
            // User might already exist, which is fine for this example
        }
    }

    // Example 2: Login to get tokens for demonstration
    eprintln!("\n=== Logging in to get tokens for demonstration ===");
    let login_result = auth_service
        .register_user(
            "demo_user",
            "demo@example.com",
            "demo_password123",
            Some("Demo User".to_string()),
        )
        .await;

    let (access_token, refresh_token) = match login_result {
        Ok(response) => {
            eprintln!("✅ Demo user created/logged in successfully!");
            (response.access_token, response.refresh_token)
        }
        Err(_) => {
            // User might already exist, try to create a different one
            eprintln!("Demo user exists, creating alternative user...");
            let alt_result = auth_service
                .register_user(
                    &format!("demo_user_{}", chrono::Utc::now().timestamp()),
                    &format!("demo_{}@example.com", chrono::Utc::now().timestamp()),
                    "demo_password123",
                    Some("Demo User".to_string()),
                )
                .await;

            match alt_result {
                Ok(response) => {
                    eprintln!("✅ Alternative demo user created successfully!");
                    (response.access_token, response.refresh_token)
                }
                Err(e) => {
                    eprintln!("❌ Failed to create demo user: {}", e);
                    return Ok(());
                }
            }
        }
    };

    // Example 3: Token validation
    eprintln!("\n=== Token validation example ===");
    match auth_service.validate_token(&access_token) {
        Ok(claims) => {
            eprintln!("✅ Access token is valid!");
            eprintln!("User ID: {}", claims.sub);
            eprintln!("Username: {}", claims.username);
            eprintln!("Email: {}", claims.email);
            eprintln!("Token type: {}", claims.token_type);
            eprintln!(
                "Expires at: {}",
                chrono::DateTime::from_timestamp(claims.exp as i64, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                    .unwrap_or_else(|| "Invalid timestamp".to_string())
            );
        }
        Err(e) => {
            eprintln!("❌ Access token validation failed: {}", e);
        }
    }

    // Example 4: Refresh a token
    eprintln!("\n=== Token refresh example ===");
    match auth_service.refresh_access_token(&refresh_token).await {
        Ok(new_response) => {
            eprintln!("✅ Token refreshed successfully!");
            eprintln!("New access token: {}...", &new_response.access_token[..20]);
            eprintln!(
                "New refresh token: {}...",
                &new_response.refresh_token[..20]
            );
            eprintln!("Expires in: {} seconds", new_response.expires_in);

            // Validate the new access token to show it works
            match auth_service.validate_token(&new_response.access_token) {
                Ok(claims) => {
                    eprintln!("✅ New access token is valid!");
                    eprintln!("Username: {}", claims.username);
                }
                Err(e) => {
                    eprintln!("❌ New access token validation failed: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Token refresh failed: {}", e);
        }
    }

    eprintln!("\n🎉 Auth service example completed!");
    eprintln!("\nAvailable methods:");
    eprintln!("- register_user(username, email, password, display_name)");
    eprintln!("- login() - via gRPC ProtoAuthService trait");
    eprintln!("- refresh_token(refresh_token)");
    eprintln!("- validate_token(token)");

    Ok(())
}
