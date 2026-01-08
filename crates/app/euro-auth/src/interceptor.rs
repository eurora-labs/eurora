use crate::{AuthManager, get_secure_channel};
use http::header::{AUTHORIZATION, HeaderValue};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::OnceCell;
use tonic::{body::Body, transport::Channel};
use tower::{Layer, Service, ServiceBuilder};

pub type AuthedChannel = AuthService<Channel>;
pub static AUTHED_CHANNEL: OnceCell<AuthedChannel> = OnceCell::const_new();

#[derive(Debug, Clone)]
pub struct AuthService<S> {
    inner: S,
    auth_manager: AuthManager,
}

impl AuthService<Channel> {
    pub async fn new(channel: Channel, auth_manager: AuthManager) -> Self {
        Self {
            inner: channel,
            auth_manager,
        }
    }
}

#[derive(Clone)]
pub struct AuthLayer {
    auth_manager: AuthManager,
}

impl AuthLayer {
    pub fn new(auth_manager: AuthManager) -> Self {
        Self { auth_manager }
    }
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthService {
            inner,
            auth_manager: self.auth_manager.clone(),
        }
    }
}

// This is the critical part: implement Service<Request<BoxBody>>
// so that AuthService<S> is a valid tonic transport.
impl<S> Service<http::Request<Body>> for AuthService<S>
where
    S: Service<http::Request<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: http::Request<Body>) -> Self::Future {
        let mut inner = self.inner.clone();
        let auth_manager = self.auth_manager.clone();

        Box::pin(async move {
            let token = auth_manager
                .get_or_refresh_access_token()
                .await
                .map_err(|status| {
                    // Map tonic::Status into your transport error as needed.
                    // For a real app, don't panic. Convert into S::Error properly.
                    panic!("token refresh failed: {status}");
                })?;

            // let token: MetadataValue<_> = format!("Bearer {}", token.0).parse().unwrap();
            // req.metadata_mut().insert("authorization", token);
            req.headers_mut().insert(
                AUTHORIZATION,
                HeaderValue::from_str(&token.0).expect("valid header value"),
            );

            inner.call(req).await
        })
    }
}

async fn build_authed_channel() -> AuthedChannel {
    let channel = get_secure_channel()
        .await
        .expect("Failed to build secure channel");

    let auth_manager = AuthManager::new()
        .await
        .expect("Failed to create AuthManager");

    ServiceBuilder::new()
        .layer(AuthLayer::new(auth_manager))
        .service(channel)
}

pub async fn get_authed_channel() -> AuthedChannel {
    AUTHED_CHANNEL
        .get_or_init(|| async { build_authed_channel().await })
        .await
        .clone()
}
