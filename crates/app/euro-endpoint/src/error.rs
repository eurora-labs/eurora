use thiserror::Error;

#[derive(Error, Debug)]
pub enum EndpointError {
    #[error("Invalid endpoint URL: {0}")]
    InvalidUrl(#[source] url::ParseError),

    #[error("Failed to build HTTP client: {0}")]
    Build(#[source] reqwest::Error),
}

pub type Result<T> = std::result::Result<T, EndpointError>;
