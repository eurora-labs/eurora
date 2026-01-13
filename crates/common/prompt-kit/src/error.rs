use thiserror::Error;

#[derive(Error, Debug)]
pub enum PromptKitError {
    #[error("{0}")]
    AgentChainError(#[from] agent_chain_core::Error),

    #[error("{service} not initialized")]
    ServiceNotInitialized { service: String },
}
