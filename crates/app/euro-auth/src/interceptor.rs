use crate::AuthManager;
use log::error;
use std::pin::Pin;
use tonic::{Request, Status, transport::Channel};
use tonic_async_interceptor::{AsyncInterceptor, async_interceptor};
use tower::ServiceBuilder;

pub type AuthedChannel = tonic_async_interceptor::AsyncInterceptedService<Channel, AuthInterceptor>;

#[derive(Debug, Clone)]
pub struct AuthInterceptor {
    auth_manager: AuthManager,
}

impl AuthInterceptor {
    pub fn new(auth_manager: AuthManager) -> Self {
        Self { auth_manager }
    }
}

impl AsyncInterceptor for AuthInterceptor {
    type Future =
        Pin<Box<dyn std::future::Future<Output = Result<Request<()>, Status>> + Send + 'static>>;

    fn call(&mut self, mut request: Request<()>) -> Self::Future {
        let mut auth_manager = self.auth_manager.clone();

        Box::pin(async move {
            let token = auth_manager
                .get_or_refresh_access_token()
                .await
                .map_err(|e| {
                    error!("Failed to get access token: {}", e);
                    Status::unauthenticated(format!("Failed to retrieve access token: {}", e))
                })?;

            let bearer_value = format!("Bearer {}", token.0);
            let metadata_value = bearer_value.parse().map_err(|e| {
                error!("Failed to parse authorization header: {}", e);
                Status::internal("Failed to create authorization header")
            })?;

            request
                .metadata_mut()
                .insert("authorization", metadata_value);

            Ok(request)
        })
    }
}

pub fn build_authed_channel(channel: Channel, auth_manager: AuthManager) -> AuthedChannel {
    let interceptor = AuthInterceptor::new(auth_manager);

    ServiceBuilder::new()
        .layer(async_interceptor(interceptor))
        .service(channel)
}
