use thiserror::Error;

#[derive(Error, Debug)]
pub enum NativeMessagingError {
    #[error("{0}")]
    Error(String),
}

impl NativeMessagingError {
    pub fn new<S: ToString>(err: S) -> Self {
        NativeMessagingError::Error(err.to_string())
    }
}

pub type NativeMessagingResult<T> = Result<T, NativeMessagingError>;
