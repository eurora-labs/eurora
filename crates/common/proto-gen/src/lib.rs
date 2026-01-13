//! Generated Protocol Buffer code for Eurora services
//!
//! This crate contains auto-generated code from proto files
//! and makes them available to other Rust crates in the workspace.
#![allow(clippy::all)]

// Include the generated code
mod proto {
    pub mod activity {
        tonic::include_proto!("activity_service");
        pub use super::*;
    }

    pub mod asset {
        tonic::include_proto!("asset_service");
        pub use super::*;
    }

    pub mod auth {
        tonic::include_proto!("auth_service");
        pub use super::*;
    }

    pub mod conversation {
        tonic::include_proto!("conversation_service");
        pub use super::*;
    }

    pub mod shared {
        tonic::include_proto!("shared");
        pub use super::*;
    }
}

// Convenience re-exports of the most commonly used types
pub use proto::*;
