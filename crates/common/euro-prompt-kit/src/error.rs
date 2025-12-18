use euro_llm_eurora::EuroraError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PromptKitError {
    #[error("{0}")]
    AgentChainError(#[from] agent_chain::Error),

    #[error("{0}")]
    EuroraError(EuroraError),

    #[error("{service} not initialized")]
    ServiceNotInitialized { service: String },
}
