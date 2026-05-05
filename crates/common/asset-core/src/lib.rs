//! Shared HTTP DTOs for the Eurora asset service.
//!
//! Used by `be-asset` (domain logic), `be-asset-service` (HTTP server) and the
//! desktop app's HTTP client. The same types feed Specta-generated TypeScript
//! bindings when the `specta` feature is enabled.

pub mod asset;

pub use asset::{Asset, CreateAssetRequest};
