use std::sync::Arc;

use casbin::{CoreApi, DefaultModel, Enforcer, MgmtApi, prelude::FileAdapter};
use tracing::info;

use crate::AuthzError;

/// Shared casbin enforcer wrapped for concurrent access.
///
/// The enforcer is read-only after initialization — no lock is needed.
#[derive(Clone)]
pub struct CasbinAuthz {
    enforcer: Arc<Enforcer>,
}

impl CasbinAuthz {
    /// Initialize from model and policy file paths.
    pub async fn new(model_path: &str, policy_path: &str) -> Result<Self, AuthzError> {
        let model = DefaultModel::from_file(model_path)
            .await
            .map_err(|e| AuthzError::Init(format!("Failed to load model: {e}")))?;
        let adapter = FileAdapter::new(policy_path.to_owned());
        let enforcer = Enforcer::new(model, adapter)
            .await
            .map_err(|e| AuthzError::Init(e.to_string()))?;

        info!(
            policies = enforcer.get_policy().len(),
            "Casbin enforcer initialized"
        );

        Ok(Self {
            enforcer: Arc::new(enforcer),
        })
    }

    /// Check if a role is allowed to perform an action on a resource.
    #[must_use = "authorization result must be checked"]
    pub fn enforce(&self, role: &str, resource: &str, action: &str) -> Result<bool, AuthzError> {
        self.enforcer
            .enforce((role, resource, action))
            .map_err(|e| AuthzError::Enforcement(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build an enforcer from the repo's config files.
    async fn test_authz() -> CasbinAuthz {
        let base = env!("CARGO_MANIFEST_DIR");
        let model = format!("{base}/../../../config/authz/model.conf");
        let policy = format!("{base}/../../../config/authz/policy.csv");
        CasbinAuthz::new(&model, &policy)
            .await
            .expect("failed to init enforcer")
    }

    // -- Role hierarchy --

    #[tokio::test]
    async fn free_can_list_conversations() {
        let authz = test_authz().await;
        assert!(
            authz
                .enforce("Free", "ConversationService", "ListConversations")
                .unwrap()
        );
    }

    #[tokio::test]
    async fn free_cannot_create_conversation() {
        let authz = test_authz().await;
        assert!(
            !authz
                .enforce("Free", "ConversationService", "CreateConversation")
                .unwrap()
        );
    }

    #[tokio::test]
    async fn tier1_inherits_free_permissions() {
        let authz = test_authz().await;
        assert!(
            authz
                .enforce("Tier1", "ConversationService", "ListConversations")
                .unwrap()
        );
    }

    #[tokio::test]
    async fn tier1_can_create_conversation() {
        let authz = test_authz().await;
        assert!(
            authz
                .enforce("Tier1", "ConversationService", "CreateConversation")
                .unwrap()
        );
    }

    #[tokio::test]
    async fn enterprise_inherits_all() {
        let authz = test_authz().await;
        // Inherited from Free via Tier1
        assert!(
            authz
                .enforce("Enterprise", "ConversationService", "ListConversations")
                .unwrap()
        );
        // Inherited from Tier1
        assert!(
            authz
                .enforce("Enterprise", "ConversationService", "ChatStream")
                .unwrap()
        );
    }

    // -- REST policies --

    #[tokio::test]
    async fn free_can_post_checkout() {
        let authz = test_authz().await;
        assert!(authz.enforce("Free", "/payment/checkout", "POST").unwrap());
    }

    #[tokio::test]
    async fn free_cannot_use_unknown_rest_path() {
        let authz = test_authz().await;
        assert!(!authz.enforce("Free", "/admin/users", "GET").unwrap());
    }

    // -- Unknown role --

    #[tokio::test]
    async fn unknown_role_denied() {
        let authz = test_authz().await;
        assert!(
            !authz
                .enforce("Unknown", "ConversationService", "ListConversations")
                .unwrap()
        );
    }
}
