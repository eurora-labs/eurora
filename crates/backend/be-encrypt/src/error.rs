use thiserror::Error;

#[derive(Error, Debug)]
pub enum EncryptError {
    #[error("base64 decode error")]
    Base64Decode(#[from] base64::DecodeError),

    #[error("invalid key length")]
    InvalidKeyLength,

    #[error("key error: {0}")]
    Key(String),

    #[error("format error: {0}")]
    Format(String),

    #[error("encryption error: {0}")]
    Encryption(#[from] chacha20poly1305::Error),
}

pub type EncryptResult<T> = std::result::Result<T, EncryptError>;
