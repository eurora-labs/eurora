//! `POST /auth/email/check` — public endpoint reporting whether an
//! email is registered, and if so, what credential it authenticates
//! with. Used by the desktop / web client to pick the right next-step
//! UI (sign in vs. register vs. SSO).

use auth_core::{CheckEmailStatus, Provider};

use crate::error::{AuthError, AuthResult};
use crate::service::AuthService;

impl AuthService {
    pub async fn check_email(
        &self,
        email: &str,
    ) -> AuthResult<(CheckEmailStatus, Option<Provider>)> {
        let email = email.trim();
        if email.is_empty() {
            return Err(AuthError::InvalidInput("Email is required".into()));
        }

        let user = match self.db().get_user().email(email.to_string()).call().await {
            Ok(user) => user,
            Err(e) if e.is_not_found() => return Ok((CheckEmailStatus::NotFound, None)),
            Err(e) => return Err(AuthError::Database(e)),
        };

        Ok(
            match self
                .db()
                .get_oauth_provider_for_user()
                .user_id(user.id)
                .call()
                .await?
            {
                Some(provider) => (CheckEmailStatus::Oauth, Some(provider.into())),
                None => (CheckEmailStatus::Password, None),
            },
        )
    }
}
