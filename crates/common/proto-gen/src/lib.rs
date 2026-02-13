#![allow(clippy::all)]

#[cfg(feature = "agent-chain")]
mod agent_chain_conversions;

mod proto {
    pub mod activity {
        tonic::include_proto!("activity_service");
        pub use super::*;
    }

    pub mod agent_chain {
        tonic::include_proto!("agent_chain");
        pub use super::*;
        #[cfg(feature = "agent-chain")]
        pub use crate::agent_chain_conversions::*;
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

    pub mod local_config {
        tonic::include_proto!("local_config_service");
        pub use super::*;
    }

    pub mod shared {
        tonic::include_proto!("shared");
        pub use super::*;
    }
}

pub use proto::*;
