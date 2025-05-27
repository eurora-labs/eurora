//! The Eurora monolith server that hosts the gRPC service for questions.

use anyhow::{Result, anyhow};
use dotenv::dotenv;
use eur_proto::proto_auth_service::proto_auth_service_server::{
    ProtoAuthService, ProtoAuthServiceServer,
};
use eur_proto::proto_auth_service::{LoginRequest, LoginResponse};
use futures::future;
use std::env;
use tonic::{Request, Response, Status, transport::Server};
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

#[derive(Default, Debug)]
pub struct AuthService {}

#[tonic::async_trait]
impl ProtoAuthService for AuthService {
    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        todo!()
    }
}
