use thiserror::Error;

#[derive(Error, Debug)]
pub enum EndpointError {
    #[error("Invalid endpoint URL: {0}")]
    InvalidUrl(String),

    #[error("TLS configuration error: {0}")]
    Tls(#[source] tonic::transport::Error),

    #[error("No endpoint subscribers")]
    NoSubscribers,
}

pub type Result<T> = std::result::Result<T, EndpointError>;
