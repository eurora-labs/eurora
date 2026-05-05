#![allow(clippy::all)]

pub use agent_chain_core::proto as agent_chain;

mod proto {
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
