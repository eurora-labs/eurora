//! Generated Protocol Buffer code for Eurora services
//!
//! This crate contains auto-generated code from proto files
//! and makes them available to other Rust crates in the workspace.

// Include the generated code
pub mod generated {
    // Re-export all generated modules
    pub mod questions_service {
        include!("gen/questions_service.rs");
        pub use super::*;
    }

    pub mod shared {
        include!("gen/shared.rs");
        pub use super::*;
    }

    pub mod ipc {
        include!("gen/ipc.rs");
        pub use super::*;
    }

    pub mod native_messaging {
        include!("gen/native_messaging.rs");
        pub use super::*;
    }

    pub mod proto_ocr_service {
        include!("gen/ocr_service.rs");
        pub use super::*;
    }

    pub mod proto_auth_service {
        include!("gen/auth_service.rs");
        pub use super::*;
    }

    pub mod proto_prompt_service {
        include!("gen/prompt_service.rs");
        pub use super::*;
    }
}

// Convenience re-exports of the most commonly used types
pub use generated::*;
