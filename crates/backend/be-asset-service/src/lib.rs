//! Euro Assets Service
//!
//! This crate provides a gRPC service for managing user file assets.
//! It serves as a cloud-based replacement for the asset-related
//! functionality in the local personal database.
//!
//! ## Features
//!
//! - `server` - Enables server-side functionality including the gRPC service
//!   implementation. This feature adds dependencies on `auth-core` and
//!   `euro-remote-db`. Without this feature, only the proto types and
//!   client are available.

use asset_models::proto;

pub mod error;
mod server;

pub use error::{AssetServiceError, Result};
pub use server::{AssetService, ProtoAssetService, ProtoAssetServiceServer};
