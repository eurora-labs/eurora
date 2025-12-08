//! Common gRPC client functionality for Eurora services
//!
//! This crate provides the base functionality for building gRPC clients
//! to communicate with Eurora backend services.

mod connection;

pub use connection::ClientBuilder;

/// Create a new client builder with default settings
pub fn client_builder() -> ClientBuilder {
    ClientBuilder::new()
}
