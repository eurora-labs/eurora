use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use auth_core::Claims;
use axum::extract::ConnectInfo;
use be_auth_core::JwtConfig;
use be_remote_db::DatabaseManager;
use http::Request;
use tonic::Status;
use tower::{Layer, Service};

use crate::CasbinAuthz;
use crate::bypass::is_grpc_bypass;
use crate::rate_limit::AuthFailureRateLimiter;
use crate::token_gate;

#[derive(Clone)]
pub struct GrpcAuthzLayer {
    authz: CasbinAuthz,
    jwt_config: Arc<JwtConfig>,
    rate_limiter: AuthFailureRateLimiter,
    db: Arc<DatabaseManager>,
}

impl GrpcAuthzLayer {
    pub fn new(
        authz: CasbinAuthz,
        jwt_config: JwtConfig,
        rate_limiter: AuthFailureRateLimiter,
        db: Arc<DatabaseManager>,
    ) -> Self {
        Self {
            authz,
            jwt_config: Arc::new(jwt_config),
            rate_limiter,
            db,
        }
    }
}

impl<S> Layer<S> for GrpcAuthzLayer {
    type Service = GrpcAuthzService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        GrpcAuthzService {
            inner,
            authz: self.authz.clone(),
            jwt_config: Arc::clone(&self.jwt_config),
            rate_limiter: Arc::clone(&self.rate_limiter),
            db: Arc::clone(&self.db),
        }
    }
}

#[derive(Clone)]
pub struct GrpcAuthzService<S> {
    inner: S,
    authz: CasbinAuthz,
    jwt_config: Arc<JwtConfig>,
    rate_limiter: AuthFailureRateLimiter,
    db: Arc<DatabaseManager>,
}

fn extract_client_ip<B>(req: &Request<B>) -> IpAddr {
    req.extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.ip())
        .unwrap_or(IpAddr::from([127, 0, 0, 1]))
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for GrpcAuthzService<S>
where
    S: Service<Request<ReqBody>, Response = http::Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Send + 'static,
    ReqBody: Send + 'static,
    ResBody: Default + Send + 'static,
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
        let jwt_config = Arc::clone(&self.jwt_config);
        let rate_limiter = Arc::clone(&self.rate_limiter);
        let db = Arc::clone(&self.db);
        let client_ip = extract_client_ip(&req);
        let inner = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, inner);

        Box::pin(async move {
            if req.method() == http::Method::OPTIONS {
                return inner.call(req).await;
            }

            let (service_full, method) = match parse_grpc_path(&path) {
                Some(parts) => parts,
                None => {
                    tracing::warn!(path = %path, "Rejecting request with unparseable gRPC path");
                    return Ok(Status::invalid_argument("Invalid gRPC path").into_http());
                }
            };

            if is_grpc_bypass(&service_full) {
                tracing::debug!(path = %path, "Bypassing authorization for public service");
                return inner.call(req).await;
            }

            if rate_limiter.check_key(&client_ip).is_err() {
                tracing::warn!(ip = %client_ip, "Rate limited — too many auth failures");
                return Ok(Status::resource_exhausted(
                    "Too many failed requests. Try again later.",
                )
                .into_http());
            }

            let claims = match extract_jwt_claims(&req, &jwt_config) {
                Ok(claims) => claims,
                Err(status) => {
                    let _ = rate_limiter.check_key(&client_ip);
                    return Ok(status.into_http());
                }
            };

            let service_name = extract_service_name(&service_full);
            let role = claims.role.to_string();

            match authz.enforce(&role, &service_name, &method) {
                Ok(true) => {
                    tracing::debug!(role = %role, service = %service_name, method = %method, "gRPC authorized");

                    if token_gate::is_token_gated(&service_full, &method) {
                        let user_id = match uuid::Uuid::parse_str(&claims.sub) {
                            Ok(id) => id,
                            Err(_) => {
                                return Ok(Status::internal("Invalid user ID in token").into_http());
                            }
                        };
                        if let Err(status) = token_gate::check_token_limit(&db, user_id).await {
                            return Ok(status.into_http());
                        }
                    }

                    req.extensions_mut().insert(claims);
                    inner.call(req).await
                }
                Ok(false) => {
                    let _ = rate_limiter.check_key(&client_ip);
                    tracing::warn!(role = %role, service = %service_name, method = %method, "gRPC authorization denied");
                    Ok(Status::permission_denied(
                        "Insufficient permissions. Please upgrade your plan.",
                    )
                    .into_http())
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Authorization enforcement error");
                    Ok(Status::internal("Authorization error").into_http())
                }
            }
        })
    }
}

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

    jwt_config.validate_access_token(token).map_err(|e| {
        tracing::warn!(error = %e, "JWT validation failed");
        Status::unauthenticated("Invalid or expired token")
    })
}

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

fn extract_service_name(full_service: &str) -> String {
    let name = full_service.rsplit('.').next().unwrap_or(full_service);
    name.strip_prefix("Proto").unwrap_or(name).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_grpc_path_valid() {
        let result = parse_grpc_path("/thread_service.ProtoThreadService/ChatStream");
        assert_eq!(
            result,
            Some((
                "thread_service.ProtoThreadService".to_string(),
                "ChatStream".to_string()
            ))
        );
    }

    #[test]
    fn parse_grpc_path_health() {
        let result = parse_grpc_path("/grpc.health.v1.Health/Check");
        assert_eq!(
            result,
            Some(("grpc.health.v1.Health".to_string(), "Check".to_string()))
        );
    }

    #[test]
    fn parse_grpc_path_no_leading_slash() {
        assert_eq!(parse_grpc_path("no_slash/Method"), None);
    }

    #[test]
    fn parse_grpc_path_empty_method() {
        assert_eq!(parse_grpc_path("/service.Name/"), None);
    }

    #[test]
    fn parse_grpc_path_no_method_slash() {
        assert_eq!(parse_grpc_path("/service.Name"), None);
    }

    #[test]
    fn parse_grpc_path_root() {
        assert_eq!(parse_grpc_path("/"), None);
    }

    #[test]
    fn extract_service_name_strips_proto_prefix() {
        assert_eq!(
            extract_service_name("thread_service.ProtoThreadService"),
            "ThreadService"
        );
    }

    #[test]
    fn extract_service_name_no_proto_prefix() {
        assert_eq!(extract_service_name("grpc.health.v1.Health"), "Health");
    }

    #[test]
    fn extract_service_name_no_package() {
        assert_eq!(extract_service_name("ProtoFoo"), "Foo");
    }

    #[test]
    fn extract_service_name_plain() {
        assert_eq!(extract_service_name("MyService"), "MyService");
    }
}
