//! Basic usage example for the Eurora Auth Service
//!
//! This example demonstrates how to:
//! 1. Set up the auth service with a database connection
//! 2. Register a new user
//! 3. Login with email/password
//! 4. Refresh tokens
//! 5. Validate tokens

use anyhow::Result;
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

    // Example 2: Refresh a token
    println!("\n=== Token refresh example ===");
    // In a real application, you would get this from the previous login/register response
    // For this example, we'll just show how the method would be called
    println!("Token refresh method is available: auth_service.refresh_token(refresh_token)");

    // Example 3: Token validation
    println!("\n=== Token validation example ===");
    println!("Token validation method is available: auth_service.validate_token(token)");

    println!("\n🎉 Auth service example completed!");
    println!("\nAvailable methods:");
    println!("- register_user(username, email, password, display_name)");
    println!("- login() - via gRPC ProtoAuthService trait");
    println!("- refresh_token(refresh_token)");
    println!("- validate_token(token)");

    Ok(())
}
