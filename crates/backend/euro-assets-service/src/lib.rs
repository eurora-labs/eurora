//! Euro Assets Service
//!
//! This crate provides a gRPC service for managing user file assets.
//! It serves as a cloud-based replacement for the asset-related
//! functionality in the local personal database.
//!
//! ## Features
//!
//! - `server` - Enables server-side functionality including the gRPC service
//!   implementation. This feature adds dependencies on `euro-auth` and
//!   `euro-remote-db`. Without this feature, only the proto types and
//!   client are available.

// Include the generated proto code
pub mod proto {
    tonic::include_proto!("assets_service");
}

// Server module is only available with the "server" feature
#[cfg(feature = "server")]
mod server;

// Storage module is only available with the "server" feature
#[cfg(feature = "server")]
mod storage;

// Re-export proto types (always available)
pub use proto::*;

// Re-export server types when the feature is enabled
#[cfg(feature = "server")]
pub use server::{
    AssetsService, ProtoAssetsService, ProtoAssetsServiceServer, authenticate_request,
};

// Re-export storage types when the feature is enabled
#[cfg(feature = "server")]
pub use storage::{StorageConfig, StorageService};
