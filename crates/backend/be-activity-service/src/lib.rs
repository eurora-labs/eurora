//! Euro Activity Service
//!
//! This crate provides a gRPC service for managing user activities.
//! It serves as a cloud-based replacement for the activity-related
//! functionality in the local personal database.
//!
//! ## Features
//!
//! - `server` - Enables server-side functionality including the gRPC service
//!   implementation. This feature adds dependencies on `auth-core` and
//!   `euro-remote-db`. Without this feature, only the proto types and
//!   client are available.

mod server;

pub use server::{ActivityService, ProtoActivityService, ProtoActivityServiceServer};
