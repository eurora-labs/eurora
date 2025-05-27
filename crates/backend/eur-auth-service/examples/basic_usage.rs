//! Basic usage example for the Eurora Auth Service
//!
//! This example demonstrates how to:
//! 1. Set up the auth service with a database connection
//! 2. Register a new user
//! 3. Login with email/password
//! 4. Refresh tokens
//! 5. Validate tokens

use anyhow::Result;
use chrono;
use eur_auth_service::{AuthService, JwtConfig};
use eur_remote_db::DatabaseManager;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Note: This example requires a PostgreSQL database to be running
    // You would typically get this from environment variables
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/eurora".to_string());

    println!("Connecting to database...");

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
    };

    // Create auth service
    let auth_service = AuthService::new(db_manager, Some(jwt_config));

    println!("Auth service initialized successfully!");

    // Example 1: Register a new user
    println!("\n=== Registering a new user ===");
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
            println!("✅ User registered successfully!");
            println!("Access token: {}...", &login_response.access_token[..20]);
            println!("Refresh token: {}...", &login_response.refresh_token[..20]);
            println!("Expires in: {} seconds", login_response.expires_in);
        }
        Err(e) => {
            println!("❌ Registration failed: {}", e);
            // User might already exist, which is fine for this example
        }
    }

    // Example 2: Login to get tokens for demonstration
    println!("\n=== Logging in to get tokens for demonstration ===");
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
            println!("✅ Demo user created/logged in successfully!");
            (response.access_token, response.refresh_token)
        }
        Err(_) => {
            // User might already exist, try to create a different one
            println!("Demo user exists, creating alternative user...");
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
                    println!("✅ Alternative demo user created successfully!");
                    (response.access_token, response.refresh_token)
                }
                Err(e) => {
                    println!("❌ Failed to create demo user: {}", e);
                    return Ok(());
                }
            }
        }
    };

    // Example 3: Token validation
    println!("\n=== Token validation example ===");
    match auth_service.validate_token(&access_token) {
        Ok(claims) => {
            println!("✅ Access token is valid!");
            println!("User ID: {}", claims.sub);
            println!("Username: {}", claims.username);
            println!("Email: {}", claims.email);
            println!("Token type: {}", claims.token_type);
            println!(
                "Expires at: {}",
                chrono::DateTime::from_timestamp(claims.exp as i64, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                    .unwrap_or_else(|| "Invalid timestamp".to_string())
            );
        }
        Err(e) => {
            println!("❌ Access token validation failed: {}", e);
        }
    }

    // Example 4: Refresh a token
    println!("\n=== Token refresh example ===");
    match auth_service.refresh_access_token(&refresh_token).await {
        Ok(new_response) => {
            println!("✅ Token refreshed successfully!");
            println!("New access token: {}...", &new_response.access_token[..20]);
            println!(
                "New refresh token: {}...",
                &new_response.refresh_token[..20]
            );
            println!("Expires in: {} seconds", new_response.expires_in);

            // Validate the new access token to show it works
            match auth_service.validate_token(&new_response.access_token) {
                Ok(claims) => {
                    println!("✅ New access token is valid!");
                    println!("Username: {}", claims.username);
                }
                Err(e) => {
                    println!("❌ New access token validation failed: {}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ Token refresh failed: {}", e);
        }
    }

    println!("\n🎉 Auth service example completed!");
    println!("\nAvailable methods:");
    println!("- register_user(username, email, password, display_name)");
    println!("- login() - via gRPC ProtoAuthService trait");
    println!("- refresh_token(refresh_token)");
    println!("- validate_token(token)");

    Ok(())
}
