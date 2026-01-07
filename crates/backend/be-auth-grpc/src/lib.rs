use tonic::{Request, Status, service::Interceptor};

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

        // Remove "Bearer " prefix
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
