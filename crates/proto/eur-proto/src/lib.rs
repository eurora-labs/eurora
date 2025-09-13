//! Generated Protocol Buffer code for Eurora services
//!
//! This crate contains auto-generated code from proto files
//! and makes them available to other Rust crates in the workspace.
#![allow(clippy::all)]

// Include the generated code
pub mod generated {
    pub mod shared {
        tonic::include_proto!("shared");
        pub use super::*;
    }

    pub mod ipc {
        tonic::include_proto!("ipc");
        pub use super::*;
    }

    pub mod native_messaging {
        tonic::include_proto!("native_messaging");
        pub use super::*;
    }

    pub mod proto_ocr_service {
        tonic::include_proto!("ocr_service");
        pub use super::*;
    }

    pub mod proto_auth_service {
        tonic::include_proto!("auth_service");
        pub use super::*;
    }

    pub mod proto_prompt_service {
        tonic::include_proto!("prompt_service");
        pub use super::*;
    }
}

// Convenience re-exports of the most commonly used types
pub use generated::*;
