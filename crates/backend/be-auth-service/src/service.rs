use std::sync::Arc;

use be_auth_core::JwtConfig;
use be_email_service::EmailService;
use be_remote_db::DatabaseManager;

use crate::AuthService;

/// Shared state injected into Axum handlers via `State<Arc<AppState>>`.
///
/// Wraps a single [`AuthService`] which owns the database handle, JWT
/// config, optional email service, and lazily-initialised OAuth clients.
/// Handlers reach into `state.auth` to invoke business-logic methods,
/// and into `state.jwt_config` to validate inbound bearer tokens.
pub struct AppState {
    pub auth: AuthService,
    pub jwt_config: JwtConfig,
}

impl AppState {
    pub fn new(
        db: Arc<DatabaseManager>,
        jwt_config: JwtConfig,
        email_service: Option<Arc<EmailService>>,
    ) -> Self {
        tracing::info!("Creating new auth-service AppState");
        let auth = AuthService::new(db, jwt_config.clone(), email_service);
        Self { auth, jwt_config }
    }
}
