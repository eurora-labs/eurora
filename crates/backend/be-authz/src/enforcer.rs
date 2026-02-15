use std::sync::Arc;

use casbin::{CoreApi, DefaultModel, Enforcer, MgmtApi, prelude::FileAdapter};
use tokio::sync::RwLock;
use tracing::info;

use crate::AuthzError;

/// Shared casbin enforcer wrapped for concurrent access.
#[derive(Clone)]
pub struct CasbinAuthz {
    enforcer: Arc<RwLock<Enforcer>>,
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
            enforcer: Arc::new(RwLock::new(enforcer)),
        })
    }

    /// Check if a role is allowed to perform an action on a resource.
    pub async fn enforce(
        &self,
        role: &str,
        resource: &str,
        action: &str,
    ) -> Result<bool, AuthzError> {
        let enforcer = self.enforcer.read().await;
        enforcer
            .enforce(vec![
                role.to_string(),
                resource.to_string(),
                action.to_string(),
            ])
            .map_err(|e| AuthzError::Enforcement(e.to_string()))
    }
}
