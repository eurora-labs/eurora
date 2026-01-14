//! Euro Assets Service
//!
//! This crate provides a gRPC service for managing user file assets.
//! It serves as a cloud-based replacement for the asset-related
//! functionality in the local personal database.

pub mod error;
mod server;

pub use error::{AssetServiceError, Result};
pub use server::{AssetService, ProtoAssetService, ProtoAssetServiceServer};
