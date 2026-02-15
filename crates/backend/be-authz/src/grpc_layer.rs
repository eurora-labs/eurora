use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;
use std::sync::LazyLock;
use std::task::{Context, Poll};

use auth_core::Claims;
use be_auth_core::JwtConfig;
use http::Request;
use tonic::Status;
use tonic::body::Body;
use tower::{Layer, Service};
use tracing::{debug, warn};

use crate::CasbinAuthz;

/// gRPC service paths that bypass authorization entirely (public/unauthenticated).
static BYPASS_SERVICES: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "auth_service.ProtoAuthService",
        "grpc.health.v1.Health",
        "local_config_service.ProtoLocalConfigService",
    ])
});

/// Tower layer that combines JWT authentication and casbin authorization for gRPC.
#[derive(Clone)]
pub struct GrpcAuthzLayer {
    authz: CasbinAuthz,
    jwt_config: JwtConfig,
}

impl GrpcAuthzLayer {
    pub fn new(authz: CasbinAuthz, jwt_config: JwtConfig) -> Self {
        Self { authz, jwt_config }
    }
}

impl<S> Layer<S> for GrpcAuthzLayer {
    type Service = GrpcAuthzService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        GrpcAuthzService {
            inner,
            authz: self.authz.clone(),
            jwt_config: self.jwt_config.clone(),
        }
    }
}

/// Tower service that enforces JWT + casbin policies on gRPC requests.
#[derive(Clone)]
pub struct GrpcAuthzService<S> {
    inner: S,
    authz: CasbinAuthz,
    jwt_config: JwtConfig,
}

impl<S, ReqBody> Service<Request<ReqBody>> for GrpcAuthzService<S>
where
    S: Service<Request<ReqBody>, Response = http::Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Send + 'static,
    ReqBody: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        let path = req.uri().path().to_string();
        let authz = self.authz.clone();
        let jwt_config = self.jwt_config.clone();
        let inner = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, inner);

        Box::pin(async move {
            let (service_full, method) = match parse_grpc_path(&path) {
                Some(parts) => parts,
                None => return inner.call(req).await,
            };

            if BYPASS_SERVICES.contains(service_full.as_str()) {
                debug!(path = %path, "Bypassing authorization for public service");
                return inner.call(req).await;
            }

            let claims = match extract_jwt_claims(&req, &jwt_config) {
                Ok(claims) => claims,
                Err(status) => return Ok(status.into_http()),
            };

            let service_name = extract_service_name(&service_full);
            let role = claims.role.to_string();

            match authz.enforce(&role, &service_name, &method).await {
                Ok(true) => {
                    debug!(role = %role, service = %service_name, method = %method, "gRPC authorized");
                    req.extensions_mut().insert(claims);
                    inner.call(req).await
                }
                Ok(false) => {
                    warn!(role = %role, service = %service_name, method = %method, "gRPC authorization denied");
                    Ok(Status::permission_denied(
                        "Insufficient permissions. Please upgrade your plan.",
                    )
                    .into_http())
                }
                Err(e) => {
                    warn!(error = %e, "Authorization enforcement error");
                    Ok(Status::internal("Authorization error").into_http())
                }
            }
        })
    }
}

/// Extract and validate JWT claims from the request's authorization header.
fn extract_jwt_claims<B>(req: &Request<B>, jwt_config: &JwtConfig) -> Result<Claims, Status> {
    let auth_header = req
        .headers()
        .get("authorization")
        .ok_or_else(|| Status::unauthenticated("Missing authorization header"))?
        .to_str()
        .map_err(|_| Status::unauthenticated("Invalid authorization header"))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| Status::unauthenticated("Authorization header must start with 'Bearer '"))?;

    jwt_config
        .validate_access_token(token)
        .map_err(|e| Status::unauthenticated(e.to_string()))
}

/// Parse a gRPC path `/package.ServiceName/MethodName` into `(full_service, method)`.
fn parse_grpc_path(path: &str) -> Option<(String, String)> {
    let path = path.strip_prefix('/')?;
    let slash_idx = path.find('/')?;
    let service = &path[..slash_idx];
    let method = &path[slash_idx + 1..];
    if method.is_empty() {
        return None;
    }
    Some((service.to_string(), method.to_string()))
}

/// Extract the short service name from the full gRPC service path.
/// `conversation_service.ProtoConversationService` -> `ConversationService`
fn extract_service_name(full_service: &str) -> String {
    let name = full_service.rsplit('.').next().unwrap_or(full_service);
    name.strip_prefix("Proto").unwrap_or(name).to_string()
}
