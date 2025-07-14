use ferrous_llm_core::ConfigError;
use ferrous_llm_grpc::GrpcError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EuroraError {
    #[error("{0}")]
    Config(ConfigError),

    #[error("{0}")]
    Other(GrpcError),
}
