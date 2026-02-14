use tonic::{Request, Status, service::Interceptor};
use uuid::Uuid;

pub use be_auth_core::*;

#[derive(Clone, Default)]
pub struct JwtInterceptor {
    config: JwtConfig,
}

impl Interceptor for JwtInterceptor {
    fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, Status> {
        let auth_header = request
            .metadata()
            .get("authorization")
            .ok_or_else(|| Status::unauthenticated("Missing authorization header"))?;

        let auth_str = auth_header
            .to_str()
            .map_err(|_| Status::unauthenticated("Invalid authorization header format"))?;

        if !auth_str.starts_with("Bearer ") {
            return Err(Status::unauthenticated(
                "Authorization header must start with 'Bearer '",
            ));
        }

        let token = &auth_str[7..];
        match self.config.validate_access_token(token) {
            Ok(claims) => {
                request.extensions_mut().insert(claims);
                Ok(request)
            }
            Err(err) => Err(Status::unauthenticated(err.to_string())),
        }
    }
}

impl JwtInterceptor {
    pub fn get_config(&self) -> &JwtConfig {
        &self.config
    }
}

pub fn extract_claims<T>(request: &Request<T>) -> Result<&Claims, Status> {
    request
        .extensions()
        .get::<Claims>()
        .ok_or_else(|| Status::unauthenticated("Missing claims"))
}

pub fn parse_user_id(claims: &Claims) -> Result<Uuid, Status> {
    Uuid::parse_str(&claims.sub).map_err(|_| Status::unauthenticated("Missing user ID"))
}

pub fn require_role(claims: &Claims, minimum: Role) -> Result<(), Status> {
    if claims.role.rank() >= minimum.rank() {
        Ok(())
    } else {
        Err(Status::permission_denied(
            "Active subscription required. Please upgrade to Pro.",
        ))
    }
}
