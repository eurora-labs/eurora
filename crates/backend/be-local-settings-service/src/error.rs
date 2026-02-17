use thiserror::Error;
use tonic::Status;

#[derive(Debug, Error)]
pub enum LocalSettingsError {
    #[error("invalid encryption key: {0}")]
    InvalidKey(#[from] be_encrypt::EncryptError),

    #[error("invalid provider settings: {0}")]
    InvalidProviderSettings(#[from] be_local_settings::Error),
}

impl From<LocalSettingsError> for Status {
    fn from(err: LocalSettingsError) -> Self {
        match &err {
            LocalSettingsError::InvalidKey(_) | LocalSettingsError::InvalidProviderSettings(_) => {
                Status::invalid_argument(err.to_string())
            }
        }
    }
}

pub type Result<T> = std::result::Result<T, LocalSettingsError>;
