//! Error types for the eur-activity crate

use std::fmt;

/// Custom error type for activity-related operations
#[derive(Debug)]
pub enum ActivityError {
    /// Error occurred during image processing
    ImageProcessing(String),
    /// Error occurred with protocol buffer data
    ProtocolBuffer(String),
    /// Network timeout or connection error
    Network(String),
    /// Invalid or missing data
    InvalidData(String),
    /// Serialization/deserialization error
    Serialization(String),
}

impl fmt::Display for ActivityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActivityError::ImageProcessing(msg) => write!(f, "Image processing error: {}", msg),
            ActivityError::ProtocolBuffer(msg) => write!(f, "Protocol buffer error: {}", msg),
            ActivityError::Network(msg) => write!(f, "Network error: {}", msg),
            ActivityError::InvalidData(msg) => write!(f, "Invalid data error: {}", msg),
            ActivityError::Serialization(msg) => write!(f, "Serialization error: {}", msg),
        }
    }
}

impl std::error::Error for ActivityError {}

impl From<image::ImageError> for ActivityError {
    fn from(err: image::ImageError) -> Self {
        ActivityError::ImageProcessing(err.to_string())
    }
}

impl From<tonic::Status> for ActivityError {
    fn from(err: tonic::Status) -> Self {
        ActivityError::Network(err.to_string())
    }
}

impl From<serde_json::Error> for ActivityError {
    fn from(err: serde_json::Error) -> Self {
        ActivityError::Serialization(err.to_string())
    }
}

impl From<anyhow::Error> for ActivityError {
    fn from(err: anyhow::Error) -> Self {
        ActivityError::Network(err.to_string())
    }
}
