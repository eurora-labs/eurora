use thiserror::Error;
use tonic::Status;

#[derive(Debug, Error)]
pub enum AssetServiceError {
    #[error("missing authentication claims")]
    MissingClaims,

    #[error("Asset: {0}")]
    Asset(#[source] be_asset::AssetError),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<AssetServiceError> for Status {
    fn from(err: AssetServiceError) -> Self {
        use AssetServiceError::*;

        match &err {
            MissingClaims => Status::unauthenticated(err.to_string()),
            Asset(err) => Status::internal(err.to_string()),
            Internal(err) => Status::internal(err.to_string()),
        }
    }
}

pub type Result<T> = std::result::Result<T, AssetServiceError>;
