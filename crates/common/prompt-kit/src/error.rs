use thiserror::Error;

#[derive(Error, Debug)]
pub enum PromptKitError {
    #[error("{0}")]
    AgentChainError(#[from] agent_chain::Error),

    #[error("{service} not initialized")]
    ServiceNotInitialized { service: String },
}
