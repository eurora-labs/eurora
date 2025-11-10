//! Browser strategy gRPC server configuration
//!
//! The gRPC server implementation has been moved to strategy.rs using a singleton pattern.
//! This module now only contains shared constants.

/// Port number for the persistent gRPC server
pub const PORT: &str = "1422";
