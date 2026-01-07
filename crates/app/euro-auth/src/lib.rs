use crate::proto::{
    EmailPasswordCredentials, GetLoginTokenResponse, LoginByLoginTokenRequest, LoginRequest,
    RefreshTokenRequest, RegisterRequest, TokenResponse, login_request::Credential,
    proto_auth_service_client::ProtoAuthServiceClient,
};
use anyhow::{Ok, Result, anyhow};
use tonic::transport::{Channel, ClientTlsConfig};
use tracing::{debug, error};

mod client;
mod interceptor;
mod manager;

pub mod proto {
    tonic::include_proto!("auth_service");
}

pub use auth_core::*;
