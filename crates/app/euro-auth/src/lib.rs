mod client;
mod interceptor;
mod manager;

pub mod proto {
    tonic::include_proto!("auth_service");
}

pub use auth_core::*;
