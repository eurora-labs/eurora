use thiserror::Error;
use tonic::Status;

#[derive(Debug, Error)]
pub enum LocalConfigError {
    #[error("invalid encryption key: {0}")]
    InvalidKey(#[from] be_encrypt::EncryptError),
}

impl From<LocalConfigError> for Status {
    fn from(err: LocalConfigError) -> Self {
        match &err {
            LocalConfigError::InvalidKey(_) => Status::invalid_argument(err.to_string()),
        }
    }
}

pub type Result<T> = std::result::Result<T, LocalConfigError>;
