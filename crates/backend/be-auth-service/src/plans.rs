//! Per-user plan + role resolution.

use auth_core::Role;
use uuid::Uuid;

use crate::error::{AuthError, AuthResult};
use crate::service::AuthService;

const PLAN_TIER1: &str = "tier1";
const PLAN_FREE: &str = "free";

impl AuthService {
    /// Look up the caller's role.
    ///
    /// In dev mode (debug builds) every user is `Tier1` — no payment
    /// service exists, so plan checks are bypassed. Otherwise we hit
    /// the DB; an absent plan row means `Free`, and any other DB error
    /// propagates rather than silently downgrading the user (which
    /// would be a billing / feature-gate hazard).
    pub(crate) async fn resolve_role(&self, user_id: Uuid) -> AuthResult<Role> {
        if self.dev_mode() {
            return Ok(Role::Tier1);
        }

        let plan_id = self
            .db()
            .get_plan_id_for_user()
            .user_id(user_id)
            .call()
            .await
            .map_err(AuthError::Database)?;

        Ok(match plan_id.as_deref() {
            Some(PLAN_TIER1) => Role::Tier1,
            _ => Role::Free,
        })
    }

    /// Idempotently ensure a `user_plans` row exists for `user_id` and
    /// return the resulting role. The plan starts at `tier1` for
    /// approved emails (see `JwtConfig::is_approved_email`) and `free`
    /// otherwise.
    pub(crate) async fn ensure_plan_and_resolve_role(
        &self,
        user_id: Uuid,
        email: &str,
    ) -> AuthResult<Role> {
        let plan_id = if self.jwt_config().is_approved_email(email) {
            PLAN_TIER1
        } else {
            PLAN_FREE
        };

        self.db()
            .ensure_user_plan()
            .executor(&self.db().pool)
            .user_id(user_id)
            .plan_id(plan_id)
            .call()
            .await?;

        self.resolve_role(user_id).await
    }
}
