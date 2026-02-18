//! Euro Thread Service
//!
//! This crate provides a gRPC service for managing user threads.
//! It serves as a cloud-based replacement for the thread-related
//! functionality in the local personal database.
//!
//! ## Error Handling
//!
//! The crate uses [`ThreadServiceError`] for all error conditions, which
//! automatically converts to appropriate gRPC [`tonic::Status`] codes.

mod converters;
mod error;
mod server;

pub use error::{ThreadServiceError, ThreadServiceResult};
pub use server::{ProtoThreadService, ProtoThreadServiceServer, ThreadService};
