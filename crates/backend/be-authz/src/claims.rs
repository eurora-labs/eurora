use auth_core::Claims;
use tonic::{Request, Status};
use uuid::Uuid;

/// Extract validated JWT claims from a gRPC request's extensions.
///
/// The [`GrpcAuthzLayer`](crate::GrpcAuthzLayer) inserts [`Claims`] into the
/// request extensions after successful authentication. Service handlers call
/// this function to retrieve them.
pub fn extract_claims<T>(request: &Request<T>) -> Result<&Claims, Status> {
    request
        .extensions()
        .get::<Claims>()
        .ok_or_else(|| Status::unauthenticated("Missing claims"))
}

/// Parse the user ID (`sub` claim) from JWT claims into a [`Uuid`].
pub fn parse_user_id(claims: &Claims) -> Result<Uuid, Status> {
    Uuid::parse_str(&claims.sub).map_err(|_| Status::unauthenticated("Missing user ID"))
}
