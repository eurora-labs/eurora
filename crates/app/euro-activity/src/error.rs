use thiserror::Error;

#[derive(Error, Debug)]
pub enum ActivityError {
    #[error("Protocol buffer error: {0}")]
    ProtocolBuffer(String),

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Image processing error: {0}")]
    Image(#[from] image::ImageError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Strategy error: {0}")]
    Strategy(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Encryption error: {0}")]
    Encryption(#[from] euro_encrypt::EncryptError),

    #[error("Invalid asset type: {0}")]
    InvalidAssetType(String),

    #[error("Invalid snapshot type: {0}")]
    InvalidSnapshotType(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl ActivityError {
    pub fn protocol_buffer(msg: impl Into<String>) -> Self {
        Self::ProtocolBuffer(msg.into())
    }

    pub fn invalid_data(msg: impl Into<String>) -> Self {
        Self::InvalidData(msg.into())
    }

    pub fn network(msg: impl Into<String>) -> Self {
        Self::Network(msg.into())
    }

    pub fn strategy(msg: impl Into<String>) -> Self {
        Self::Strategy(msg.into())
    }

    pub fn configuration(msg: impl Into<String>) -> Self {
        Self::Configuration(msg.into())
    }

    pub fn unknown(msg: impl Into<String>) -> Self {
        Self::Unknown(msg.into())
    }
}

pub type ActivityResult<T> = std::result::Result<T, ActivityError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let pb_error = ActivityError::protocol_buffer("Invalid format");
        assert!(matches!(pb_error, ActivityError::ProtocolBuffer(_)));
        assert_eq!(
            pb_error.to_string(),
            "Protocol buffer error: Invalid format"
        );

        let data_error = ActivityError::invalid_data("Missing field");
        assert!(matches!(data_error, ActivityError::InvalidData(_)));
        assert_eq!(data_error.to_string(), "Invalid data: Missing field");

        let network_error = ActivityError::network("Connection failed");
        assert!(matches!(network_error, ActivityError::Network(_)));
        assert_eq!(
            network_error.to_string(),
            "Network error: Connection failed"
        );
    }

    #[test]
    fn test_error_from_conversions() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let activity_error: ActivityError = io_error.into();
        assert!(matches!(activity_error, ActivityError::Io(_)));

        let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let activity_error: ActivityError = json_error.into();
        assert!(matches!(activity_error, ActivityError::Serialization(_)));
    }
}
