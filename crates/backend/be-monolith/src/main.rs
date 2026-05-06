use axum::extract::DefaultBodyLimit;
use axum::http::{HeaderValue, Method, header};
use be_activity_service::init_activity_service;
use be_asset_service::init_asset_service;
use be_auth_core::JwtConfig;
use be_auth_service::{CookieConfig, init_auth_service};
use be_authz::{
    AuthzState, CasbinAuthz, HttpTokenGateState, OriginGuardConfig, TrustedProxies,
    authz_middleware, http_token_gate_middleware, new_auth_failure_rate_limiter,
    new_health_check_rate_limiter, origin_guard_middleware, web_origins_from_env,
};
use be_payment_service::{PaymentService, init_payment_service};
use be_remote_db::DatabaseManager;
use be_storage::StorageService;
use be_thread_service::init_thread_service;
use be_update_service::init_update_service;
use dotenv::dotenv;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

const HTTP_MAX_BODY_SIZE: usize = 2 * 1024 * 1024; // 2 MB

fn build_cors() -> CorsLayer {
    let allowed: Vec<HeaderValue> = web_origins_from_env()
        .into_iter()
        .filter_map(|s| s.parse::<HeaderValue>().ok())
        .collect();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(allowed))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE, header::ACCEPT])
        .allow_credentials(true)
        .max_age(Duration::from_secs(3600))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to install default CryptoProvider");

    dotenv().ok();

    let _sentry_guard = if cfg!(not(debug_assertions)) {
        std::env::var("SENTRY_MONOLITH_DSN")
            .ok()
            .filter(|s| !s.is_empty())
            .map(|sentry_dsn| {
                let send_pii = std::env::var("SENTRY_SEND_PII")
                    .map(|v| v.eq_ignore_ascii_case("true"))
                    .unwrap_or(false);
                let sentry_debug = std::env::var("SENTRY_DEBUG")
                    .map(|v| v.eq_ignore_ascii_case("true"))
                    .unwrap_or(false);

                sentry::init((
                    sentry_dsn,
                    sentry::ClientOptions {
                        release: sentry::release_name!(),
                        traces_sample_rate: 0.0,
                        send_default_pii: send_pii,
                        debug: sentry_debug,
                        ..Default::default()
                    },
                ))
            })
    } else {
        None
    };

    let app_level = if cfg!(debug_assertions) {
        LevelFilter::DEBUG
    } else {
        LevelFilter::INFO
    };
    let global_filter = Targets::new()
        .with_default(LevelFilter::WARN)
        .with_target("be_", app_level)
        .with_target("agent_", app_level)
        .with_target("hyper", LevelFilter::OFF)
        .with_target("tokio", LevelFilter::OFF);

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(sentry::integrations::tracing::layer())
        .with(global_filter)
        .try_init()
        .expect("failed to initialize tracing subscriber");

    if let Some(posthog_key) = std::env::var("POSTHOG_API_KEY")
        .ok()
        .filter(|s| !s.is_empty())
    {
        let posthog_options = posthog_rs::ClientOptionsBuilder::default()
            .api_key(posthog_key)
            .host("https://eu.i.posthog.com")
            .build()
            .expect("valid posthog client options");
        match posthog_rs::init_global(posthog_options).await {
            Ok(()) => tracing::info!("PostHog analytics initialized"),
            Err(e) => tracing::warn!("Failed to initialize PostHog: {}", e),
        }
    } else {
        tracing::info!("POSTHOG_API_KEY not set, analytics disabled");
    }

    let database_url = std::env::var("REMOTE_DATABASE_URL")
        .expect("REMOTE_DATABASE_URL environment variable must be set");
    let db_manager = Arc::new(DatabaseManager::new(&database_url).await?);

    let http_addr: SocketAddr = std::env::var("HTTP_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:3000".to_string())
        .parse()
        .expect("Invalid HTTP_ADDR format");

    let jwt_config = JwtConfig::default();

    let model_path =
        std::env::var("AUTHZ_MODEL_PATH").unwrap_or_else(|_| "config/authz/model.conf".to_string());
    let policy_path = std::env::var("AUTHZ_POLICY_PATH")
        .unwrap_or_else(|_| "config/authz/policy.csv".to_string());
    let authz = CasbinAuthz::new(&model_path, &policy_path)
        .await
        .expect("Failed to initialize casbin enforcer");

    let local_mode = std::env::var("RUNNING_EURORA_FULLY_LOCAL")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let email_service = if local_mode {
        tracing::info!("Email service disabled in local mode");
        None
    } else {
        match be_email_service::EmailService::from_env() {
            Ok(svc) => {
                tracing::info!("Email service initialized");
                Some(Arc::new(svc))
            }
            Err(e) => {
                tracing::error!("Failed to initialize email service: {}", e);
                return Err(e.into());
            }
        }
    };

    let cookie_config = CookieConfig::from_env();
    let origin_guard_config = Arc::new(OriginGuardConfig::from_env());

    let auth_router = init_auth_service(
        db_manager.clone(),
        jwt_config.clone(),
        email_service.clone(),
        cookie_config.clone(),
    )
    .await?;

    let payment_service = match init_payment_service(db_manager.clone()) {
        Ok(svc) => Some(svc),
        Err(e) if local_mode => {
            tracing::warn!("Payment service disabled in local mode: {}", e);
            None
        }
        Err(e) => {
            tracing::error!("Failed to initialize payment service: {}", e);
            return Err(e.into());
        }
    };

    let storage_config =
        be_storage::StorageConfig::from_env().expect("Failed to load storage config");
    let storage = Arc::new(
        StorageService::builder()
            .config(storage_config)
            .build()
            .expect("Failed to initialize storage service"),
    );

    let core_asset = Arc::new(be_asset::AssetService::new(
        db_manager.clone(),
        storage.clone(),
    ));
    let activity_router = init_activity_service(db_manager.clone(), core_asset.clone());
    let asset_router = init_asset_service(core_asset.clone());
    let thread_router = init_thread_service(db_manager.clone(), core_asset.clone());

    tracing::info!("Starting HTTP server at {}", http_addr);

    let bucket_name =
        std::env::var("S3_BUCKET_NAME").unwrap_or_else(|_| "eurora-releases".to_string());

    let update_router = match init_update_service(bucket_name).await {
        Ok(router) => router,
        Err(e) if local_mode => {
            tracing::warn!("Update service disabled in local mode: {}", e);
            axum::Router::new()
        }
        Err(e) => {
            tracing::error!("Failed to initialize update service: {}", e);
            return Err(e.into());
        }
    };

    let (payment_router, payment_drainer) = match payment_service {
        Some(PaymentService { router, drainer }) => (router, Some(drainer)),
        None => (axum::Router::new(), None),
    };

    let auth_rate_limiter = new_auth_failure_rate_limiter();
    let health_rate_limiter = new_health_check_rate_limiter();
    let trusted_proxies = TrustedProxies::from_env();

    let authz_state = Arc::new(AuthzState::new(
        authz,
        jwt_config,
        auth_rate_limiter,
        health_rate_limiter,
        trusted_proxies,
    ));

    let token_gate_state = Arc::new(HttpTokenGateState::new(db_manager.clone()));

    let health_route = axum::Router::new().route(
        "/health",
        axum::routing::get(|| async { axum::http::StatusCode::OK }),
    );

    // Layer order matters: the last `.layer()` call is the OUTERMOST wrapper.
    //
    // Inner → outer:
    //   1. http_token_gate    — runs *after* authz so claims are already in
    //      request extensions; only inspects token-gated routes.
    //   2. authz_middleware   — verifies JWT (Authorization header or
    //      eu_access cookie) and inserts Claims.
    //   3. origin_guard       — runs before authz so a forged cross-origin
    //      request with the session cookie attached is rejected before we
    //      even look at the JWT. Bearer-mode (desktop / mobile) and
    //      same-origin server-to-server callers bypass it.
    //   4. CORS               — must be outermost so 401/403/429 short-circuit
    //      responses still carry `Access-Control-*` headers; otherwise the
    //      browser surfaces the failure as a generic "Failed to fetch"
    //      instead of the real status.
    let http_router = update_router
        .merge(payment_router)
        .merge(activity_router)
        .merge(asset_router)
        .merge(thread_router)
        .merge(auth_router)
        .merge(health_route)
        .layer(DefaultBodyLimit::max(HTTP_MAX_BODY_SIZE))
        .layer(axum::middleware::from_fn_with_state(
            token_gate_state,
            http_token_gate_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            authz_state,
            authz_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            origin_guard_config,
            origin_guard_middleware,
        ))
        .layer(build_cors());

    let http_listener = tokio::net::TcpListener::bind(http_addr).await?;
    let outcome = axum::serve(
        http_listener,
        http_router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
        tracing::info!("Shutting down HTTP server...");
    })
    .await
    .map_err(|e| {
        tracing::error!("HTTP server error: {}", e);
        Box::<dyn std::error::Error>::from(e)
    });

    if let Some(drainer) = payment_drainer {
        drainer.shutdown().await;
    }

    outcome
}
