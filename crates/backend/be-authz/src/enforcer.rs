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
    async fn free_can_list_threads() {
        let authz = test_authz().await;
        assert!(
            authz
                .enforce("Free", "ThreadService", "ListThreads")
                .unwrap()
        );
    }

    #[tokio::test]
    async fn free_cannot_create_thread() {
        let authz = test_authz().await;
        assert!(
            !authz
                .enforce("Free", "ThreadService", "CreateThread")
                .unwrap()
        );
    }

    #[tokio::test]
    async fn tier1_inherits_free_permissions() {
        let authz = test_authz().await;
        assert!(
            authz
                .enforce("Tier1", "ThreadService", "ListThreads")
                .unwrap()
        );
    }

    #[tokio::test]
    async fn tier1_can_create_thread() {
        let authz = test_authz().await;
        assert!(
            authz
                .enforce("Tier1", "ThreadService", "CreateThread")
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
                .enforce("Unknown", "ThreadService", "ListThreads")
                .unwrap()
        );
    }

    // -- Policy ↔ Proto validation --

    /// Exhaustive list of RPC methods defined in proto service definitions.
    /// If a proto file gains or removes a method, update this list — the test
    /// below will catch the drift.
    const PROTO_METHODS: &[(&str, &str)] = &[
        // thread_service.proto — ProtoThreadService
        ("ThreadService", "CreateThread"),
        ("ThreadService", "ListThreads"),
        ("ThreadService", "GetThread"),
        ("ThreadService", "GetMessages"),
        ("ThreadService", "AddHiddenHumanMessage"),
        ("ThreadService", "AddHumanMessage"),
        ("ThreadService", "AddSystemMessage"),
        ("ThreadService", "ChatStream"),
        ("ThreadService", "GenerateThreadTitle"),
        // activity_service.proto — ProtoActivityService
        ("ActivityService", "ListActivities"),
        ("ActivityService", "InsertActivity"),
        // asset_service.proto — ProtoAssetService
        ("AssetService", "CreateAsset"),
    ];

    /// Every gRPC policy entry in policy.csv must reference a method that
    /// actually exists in the proto definitions. This catches typos and stale
    /// policies after proto renames.
    #[tokio::test]
    async fn policy_grpc_actions_match_proto_methods() {
        let base = env!("CARGO_MANIFEST_DIR");
        let policy_path = format!("{base}/../../../config/authz/policy.csv");
        let policy = std::fs::read_to_string(&policy_path).expect("failed to read policy.csv");

        let proto_set: std::collections::HashSet<(&str, &str)> =
            PROTO_METHODS.iter().copied().collect();

        let mut checked = 0;
        for line in policy.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let parts: Vec<&str> = line.splitn(4, ',').map(str::trim).collect();
            if parts.len() < 4 || parts[0] != "p" {
                continue;
            }
            let resource = parts[2];
            let action = parts[3];

            // Skip REST policies (paths start with '/')
            if resource.starts_with('/') {
                continue;
            }

            assert!(
                proto_set.contains(&(resource, action)),
                "Policy entry ({resource}, {action}) does not match any proto method. \
                 Did a proto RPC get renamed or removed?"
            );
            checked += 1;
        }

        assert!(
            checked > 0,
            "No gRPC policy entries were checked — is the policy file empty?"
        );
    }
}
