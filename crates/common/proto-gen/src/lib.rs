#![allow(clippy::all)]

pub use agent_chain_core::proto as agent_chain;

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

    pub mod thread {
        tonic::include_proto!("thread_service");
        pub use super::*;
    }

    pub mod shared {
        tonic::include_proto!("shared");
        pub use super::*;
    }
}

pub use proto::*;
