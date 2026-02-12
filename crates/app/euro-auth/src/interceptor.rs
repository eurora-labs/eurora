use crate::{AuthManager, get_secure_channel};
use std::pin::Pin;
use tokio::sync::OnceCell;
use tonic::{Request, Status, transport::Channel};
use tonic_async_interceptor::{AsyncInterceptor, async_interceptor};
use tower::ServiceBuilder;

pub type AuthedChannel = tonic_async_interceptor::AsyncInterceptedService<Channel, AuthInterceptor>;

pub static AUTHED_CHANNEL: OnceCell<AuthedChannel> = OnceCell::const_new();

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
                    tracing::error!("Failed to get access token: {}", e);
                    Status::unauthenticated(format!("Failed to retrieve access token: {}", e))
                })?;

            let bearer_value = format!("Bearer {}", token.0);
            let metadata_value = bearer_value.parse().map_err(|e| {
                tracing::error!("Failed to parse authorization header: {}", e);
                Status::internal("Failed to create authorization header")
            })?;

            request
                .metadata_mut()
                .insert("authorization", metadata_value);

            Ok(request)
        })
    }
}

async fn build_authed_channel() -> AuthedChannel {
    let channel = get_secure_channel()
        .await
        .expect("Failed to build secure channel");

    let auth_manager = AuthManager::new().await;

    let interceptor = AuthInterceptor::new(auth_manager);

    ServiceBuilder::new()
        .layer(async_interceptor(interceptor))
        .service(channel)
}

pub async fn get_authed_channel() -> AuthedChannel {
    AUTHED_CHANNEL
        .get_or_init(|| async { build_authed_channel().await })
        .await
        .clone()
}
