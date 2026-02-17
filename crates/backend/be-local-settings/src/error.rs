use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    #[error("{0} must not be empty")]
    EmptyField(&'static str),
}

pub type Result<T> = std::result::Result<T, Error>;
