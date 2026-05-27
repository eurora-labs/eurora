//! Backend startup, factored out of `main` so a single `?` can return any
//! [`BootstrapError`] variant and let `main` print it nicely.
//!
//! `run` is responsible for everything between "process started" and "tokio
//! runtime is serving HTTP". `main` only owns the runtime lifecycle and the
//! pretty-printer for failures.
use std::{collections::HashSet, net::SocketAddr, sync::Arc, time::Duration};

use axum::extract::DefaultBodyLimit;
use axum::http::{HeaderValue, Method, header};
use be_activity_service::init_activity_service;
use be_asset_service::init_asset_service;
use be_auth_core::JwtConfig;
use be_auth_service::{CookieConfig, init_auth_service};
use be_authz::{
    AuthzState, CasbinAuthz, HttpTokenGateState, OriginGuardConfig, TrustedProxies,
    authz_middleware, http_token_gate_middleware, new_auth_failure_rate_limiter,
    new_health_check_rate_limiter, origin_guard_middleware,
};
use be_payment_service::{PaymentService, init_payment_service};
use be_remote_db::DatabaseManager;
use be_settings_service::init_settings_service;
use be_storage::StorageService;
use be_thread_service::init_thread_service;
use be_update_service::init_update_service;
use llm_core::LlmConfig;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use url::Url;

use crate::errors::BootstrapError;

/// The webview origin Tauri serves the desktop SPA from. Hard-coded
/// because it's a property of the Tauri runtime, not something an
/// operator configures — any other value would mean the desktop
/// build is misconfigured.
const TAURI_WEB_ORIGIN: &str = "tauri://localhost";

/// Dev mode is keyed off the build profile rather than an env var: debug
/// builds skip wiring of services that need real secrets (email, payment,
/// update) so `cargo run -p be-monolith` works against a clean Postgres
/// without any external setup. Release builds require the full stack.
pub(crate) const DEV_MODE: bool = cfg!(debug_assertions);

const HTTP_MAX_BODY_SIZE: usize = 50 * 1024 * 1024; // 50 MiB

/// Boot the backend. Owns every fallible step between "process started" and
/// "axum is serving HTTP".
pub async fn run() -> Result<(), BootstrapError> {
    install_crypto_provider();

    let _sentry_guard = init_sentry();
    init_tracing();

    // `--migrate-only` short-circuits everything below: connect to Postgres
    // (which runs `sqlx::migrate!` as part of `DatabaseManager::new`) and
    // exit. The justfile's `dev-migrate` recipe uses this to apply schema
    // before the seed step on a fresh checkout. Skipped here:
    //   - posthog / dev banner / .env warning (not relevant for a CLI tool)
    //   - LLM, Casbin, JWT, email, payment, storage, update, HTTP listener
    //     (none of those touch the schema; pulling them in would mean a
    //      working `OPENAI_API_KEY` is required just to run migrations)
    if has_flag("--migrate-only") {
        return run_migrations_only().await;
    }

    init_posthog().await?;

    if DEV_MODE {
        log_dev_banner();
    }

    let (llm_config, llm_source) = LlmConfig::from_env()?;
    let llm_config = Arc::new(llm_config);
    tracing::info!(
        source = ?llm_source,
        providers = ?llm_config.providers.keys().cloned().collect::<Vec<_>>(),
        chat_model = %llm_config.roles.chat.model,
        title_model = %llm_config.roles.title.model,
        has_vision = %llm_config.roles.vision.is_some(),
        "LLM configuration loaded"
    );

    let database_url = require_env("REMOTE_DATABASE_URL")?;
    let db_manager = Arc::new(
        DatabaseManager::new(&database_url)
            .await
            .map_err(|e| BootstrapError::Database { source: e.into() })?,
    );

    let backend_url = require_url("BACKEND_URL")?;
    let web_url = require_url("WEB_URL")?;
    let http_addr = backend_bind_addr(&backend_url)?;
    let web_origins = compose_web_origins(&backend_url, &web_url);

    let jwt_config = JwtConfig::try_from_env()?;

    let model_path = require_env("AUTHZ_MODEL_PATH")?;
    let policy_path = require_env("AUTHZ_POLICY_PATH")?;
    let authz = CasbinAuthz::new(&model_path, &policy_path)
        .await
        .map_err(|source| BootstrapError::Authz {
            model_path: model_path.clone(),
            policy_path: policy_path.clone(),
            source: source.into(),
        })?;

    let email_service = if DEV_MODE {
        tracing::info!("Email service disabled in dev mode");
        None
    } else {
        let svc = be_email_service::EmailService::from_env().map_err(|source| {
            BootstrapError::EmailService {
                source: source.into(),
            }
        })?;
        tracing::info!("Email service initialized");
        Some(Arc::new(svc))
    };

    let cookie_config = CookieConfig::from_env(web_origins.clone())?;
    let origin_guard_config = Arc::new(OriginGuardConfig::new(web_origins.clone()));

    let auth_router = init_auth_service(
        db_manager.clone(),
        jwt_config.clone(),
        email_service.clone(),
        cookie_config.clone(),
    )
    .await
    .map_err(|source| BootstrapError::Database { source })?;

    let payment_service = if DEV_MODE {
        tracing::info!("Payment service disabled in dev mode");
        None
    } else {
        let svc = init_payment_service(db_manager.clone())
            .map_err(|source| BootstrapError::PaymentService { source })?;
        Some(svc)
    };

    let storage_config =
        be_storage::StorageConfig::from_env().map_err(|source| BootstrapError::StorageConfig {
            source: source.into(),
        })?;
    let storage = Arc::new(
        StorageService::builder()
            .config(storage_config)
            .build()
            .map_err(|source| BootstrapError::StorageService {
                source: source.into(),
            })?,
    );

    let core_asset = Arc::new(be_asset::AssetService::new(
        db_manager.clone(),
        storage.clone(),
    ));
    let activity_router = init_activity_service(db_manager.clone(), core_asset.clone());
    let asset_router = init_asset_service(core_asset.clone());
    let settings_router = init_settings_service(db_manager.clone());
    let thread_router =
        init_thread_service(db_manager.clone(), core_asset.clone(), llm_config.clone())?;

    let update_router = if DEV_MODE {
        tracing::info!("Update service disabled in dev mode");
        axum::Router::new()
    } else {
        let bucket_name = require_env("S3_BUCKET_NAME")?;
        init_update_service(bucket_name)
            .await
            .map_err(|source| BootstrapError::UpdateService { source })?
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

    // `/llm/info` exposes the redacted LLM configuration (provider names,
    // models, base URLs — never API keys). The desktop app's connection
    // panel calls this *before* login to confirm a backend is reachable
    // and to surface "you're talking to: openai / gpt-4o-mini" in the UI.
    let llm_info_state = llm_config.clone();
    let llm_info_route = axum::Router::new().route(
        "/llm/info",
        axum::routing::get(move || {
            let cfg = llm_info_state.clone();
            async move { axum::Json(cfg.redacted()) }
        }),
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
        .merge(settings_router)
        .merge(thread_router)
        .merge(auth_router)
        .merge(health_route)
        .merge(llm_info_route)
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
        .layer(build_cors(&web_origins));

    tracing::info!("Starting HTTP server at {}", http_addr);
    let http_listener = tokio::net::TcpListener::bind(http_addr)
        .await
        .map_err(|source| BootstrapError::BindFailed {
            addr: http_addr,
            source,
        })?;

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
    .map_err(|source| BootstrapError::ServerRuntime { source });

    if let Some(drainer) = payment_drainer {
        drainer.shutdown().await;
    }

    outcome
}

fn install_crypto_provider() {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to install default CryptoProvider");
}

fn init_sentry() -> Option<sentry::ClientInitGuard> {
    if cfg!(debug_assertions) {
        return None;
    }
    let sentry_dsn = std::env::var("SENTRY_MONOLITH_DSN")
        .ok()
        .filter(|s| !s.is_empty())?;
    let send_pii = std::env::var("SENTRY_SEND_PII")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let sentry_debug = std::env::var("SENTRY_DEBUG")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    Some(sentry::init((
        sentry_dsn,
        sentry::ClientOptions {
            release: sentry::release_name!(),
            traces_sample_rate: 0.0,
            send_default_pii: send_pii,
            debug: sentry_debug,
            ..Default::default()
        },
    )))
}

/// True if `flag` appears anywhere in `argv`. Tiny by-design — adding clap
/// for one boolean is overkill, and the binary deliberately doesn't grow
/// other flags.
fn has_flag(flag: &str) -> bool {
    std::env::args().any(|a| a == flag)
}

/// Apply database migrations and exit. Used by `cargo run -p be-monolith
/// -- --migrate-only` from the justfile, so a fresh `just dev` can
/// migrate before seed runs.
///
/// Reuses [`DatabaseManager::new`], which invokes the same
/// `sqlx::migrate!` pass the running backend uses on every startup —
/// keeping a single source of truth for schema application.
async fn run_migrations_only() -> Result<(), BootstrapError> {
    let database_url = require_env("REMOTE_DATABASE_URL")?;
    tracing::info!("Applying database migrations…");
    let _ = DatabaseManager::new(&database_url)
        .await
        .map_err(|e| BootstrapError::Migration { source: e.into() })?;
    tracing::info!("Migrations applied.");
    Ok(())
}

fn init_tracing() {
    let app_level = if cfg!(debug_assertions) {
        LevelFilter::DEBUG
    } else {
        LevelFilter::INFO
    };
    // `Targets` uses longest-prefix match. The OpenAI SSE dump in
    // `agent_chain::providers::openai::base` is emitted on the dedicated
    // target `agent_chain::providers::openai::sse` so it can be cranked
    // up to TRACE without dragging the rest of `agent_chain` along —
    // useful when diagnosing GLM-family tool-call failures where the
    // raw wire format is the only ground truth.
    let global_filter = Targets::new()
        .with_default(LevelFilter::WARN)
        .with_target("be_", app_level)
        .with_target("agent_", app_level)
        .with_target("agent_chain::providers::openai::sse", LevelFilter::TRACE)
        .with_target("hyper", LevelFilter::OFF)
        .with_target("tokio", LevelFilter::OFF);

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(sentry::integrations::tracing::layer())
        .with(global_filter)
        .try_init()
        .expect("failed to initialize tracing subscriber");
}

async fn init_posthog() -> Result<(), BootstrapError> {
    let Some(api_key) = std::env::var("POSTHOG_API_KEY")
        .ok()
        .filter(|s| !s.is_empty())
    else {
        tracing::info!("POSTHOG_API_KEY not set, analytics disabled");
        return Ok(());
    };

    let host = require_env("POSTHOG_HOST")?;

    let posthog_options = posthog_rs::ClientOptionsBuilder::default()
        .api_key(api_key)
        .host(host)
        .build()
        .expect("valid posthog client options");
    match posthog_rs::init_global(posthog_options).await {
        Ok(()) => tracing::info!("PostHog analytics initialized"),
        Err(e) => tracing::warn!("Failed to initialize PostHog: {}", e),
    }
    Ok(())
}

fn log_dev_banner() {
    tracing::warn!("=========================================================");
    tracing::warn!(" DEV MODE (debug build)");
    tracing::warn!(" Email, payment, and update services will be skipped.");
    tracing::warn!(" New users are auto-verified. Do not expose publicly.");
    tracing::warn!("=========================================================");
}

/// Read a required environment variable. Returns
/// [`BootstrapError::MissingEnv`] if the variable is unset or blank
/// after trimming — there are no in-source fallbacks, so dev and prod
/// run the same code path. Dev defaults live in `.env.example`.
fn require_env(name: &'static str) -> Result<String, BootstrapError> {
    std::env::var(name)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .ok_or(BootstrapError::MissingEnv { name })
}

fn build_cors(web_origins: &HashSet<String>) -> CorsLayer {
    let allowed: Vec<HeaderValue> = web_origins
        .iter()
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

/// Read a required URL-valued environment variable, parsing it eagerly
/// so a typo (e.g. `BACKEND_URL=localhost:3000` missing the scheme)
/// fails at startup rather than the first time we try to derive the
/// listen address from it.
fn require_url(name: &'static str) -> Result<Url, BootstrapError> {
    let raw = require_env(name)?;
    Url::parse(&raw).map_err(|source| BootstrapError::InvalidUrl {
        name,
        value: raw,
        source,
    })
}

/// Bind address derived from `BACKEND_URL`. We always bind on
/// `0.0.0.0` so the backend is reachable from outside the host (the
/// desktop dev build runs in the same machine, but `docker compose`
/// and remote testing both need a wildcard bind); the URL only
/// contributes the port.
fn backend_bind_addr(backend_url: &Url) -> Result<SocketAddr, BootstrapError> {
    let port = backend_url
        .port_or_known_default()
        .ok_or_else(|| BootstrapError::InvalidUrl {
            name: "BACKEND_URL",
            value: backend_url.to_string(),
            source: url::ParseError::EmptyHost,
        })?;
    format!("0.0.0.0:{port}")
        .parse()
        .map_err(|source| BootstrapError::InvalidBindAddr {
            value: format!("0.0.0.0:{port}"),
            source,
        })
}

/// Build the SPA-origin allow-list from the workspace's two canonical
/// URL primitives plus the Tauri webview origin. Single source of
/// truth — every other layer (CORS, origin guard, cookie-mode
/// detection) consumes this same set.
fn compose_web_origins(backend_url: &Url, web_url: &Url) -> HashSet<String> {
    let mut origins = HashSet::new();
    origins.insert(strip_trailing_slash(backend_url));
    origins.insert(strip_trailing_slash(web_url));
    origins.insert(TAURI_WEB_ORIGIN.to_string());
    origins
}

/// `Url::to_string()` on a base URL with no path always emits a
/// trailing slash (`http://localhost:3000/`), but browsers send the
/// `Origin` header without one. Normalise so equality comparisons
/// downstream hit.
fn strip_trailing_slash(url: &Url) -> String {
    let s = url.to_string();
    s.strip_suffix('/').unwrap_or(&s).to_string()
}
