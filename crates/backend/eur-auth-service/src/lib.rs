//! The Eurora monolith server that hosts the gRPC service for questions.

use anyhow::Result;
use eur_proto::proto_auth_service::proto_auth_service_server::ProtoAuthService;
use eur_proto::proto_auth_service::{LoginRequest, LoginResponse};
use tonic::{Request, Response, Status};

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
