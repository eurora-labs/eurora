//! Euro Activity Service
//!
//! This crate provides a gRPC service for managing user activities.
//! It serves as a cloud-based replacement for the activity-related
//! functionality in the local personal database.
//!
//! ## Error Handling
//!
//! The crate uses [`ActivityServiceError`] for all error conditions, which
//! automatically converts to appropriate gRPC [`tonic::Status`] codes.

mod error;
mod server;

pub use error::{ActivityResult, ActivityServiceError};
pub use server::{ActivityService, ProtoActivityService, ProtoActivityServiceServer};
