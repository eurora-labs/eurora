use auth_core::Claims;
use tonic::{Request, Status};
use uuid::Uuid;

pub fn extract_claims<T>(request: &Request<T>) -> Result<&Claims, Status> {
    request
        .extensions()
        .get::<Claims>()
        .ok_or_else(|| Status::unauthenticated("Missing claims"))
}

pub fn parse_user_id(claims: &Claims) -> Result<Uuid, Status> {
    Uuid::parse_str(&claims.sub).map_err(|_| Status::unauthenticated("Missing user ID"))
}
