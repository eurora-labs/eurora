use crate::AuthManager;
use crate::error::AuthError;
use euro_secret::ExposeSecret;
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
        let auth_manager = self.auth_manager.clone();

        Box::pin(async move {
            let token = auth_manager
                .get_or_refresh_access_token()
                .await
                .map_err(|e| {
                    tracing::error!("Failed to get access token: {}", e);
                    status_for_auth_error(&e, "Failed to retrieve access token")
                })?;

            let bearer_value = format!("Bearer {}", token.expose_secret());
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

pub fn build_authed_channel(channel: Channel, auth_manager: AuthManager) -> AuthedChannel {
    let interceptor = AuthInterceptor::new(auth_manager);

    ServiceBuilder::new()
        .layer(async_interceptor(interceptor))
        .service(channel)
}

fn status_for_auth_error(error: &AuthError, context: &str) -> Status {
    let message = format!("{context}: {error}");
    match error {
        AuthError::InvalidRefreshToken
        | AuthError::MissingRefreshToken
        | AuthError::MissingAccessToken => Status::unauthenticated(message),
        AuthError::Transient(_) => Status::unavailable(message),
        AuthError::Other(_) => Status::internal(message),
    }
}
