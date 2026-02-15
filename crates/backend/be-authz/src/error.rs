#[derive(Debug, thiserror::Error)]
pub enum AuthzError {
    #[error("Failed to initialize casbin enforcer: {0}")]
    Init(String),

    #[error("Policy enforcement error: {0}")]
    Enforcement(String),
}
