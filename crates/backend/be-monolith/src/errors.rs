//! Typed startup errors with actionable Display messages.
//!
//! `bootstrap::run` returns [`BootstrapError`]; `main` formats it into a
//! contributor-friendly message and exits non-zero. The goal is "panic with
//! the env-var name only" → "tell me what's missing, why it matters, and
//! how to fix it" without forcing every call site through `anyhow!`.
//!
//! Variants intentionally carry the Display text inline via `thiserror`'s
//! `#[error("...")]` rather than building it dynamically; the strings render
//! verbatim and lookup is in one place.

use std::net::SocketAddr;

use llm_core::ConfigError as LlmConfigError;

#[derive(Debug, thiserror::Error)]
pub enum BootstrapError {
    #[error(
        "Missing `{name}`.

The backend reads its configuration from environment variables. This one is required.

  - For local dev, run via `just dev` (or another `just` recipe). The
    justfile loads `.env` and exports every variable to the backend.
  - To run `cargo run -p be-monolith` directly, export the variable
    yourself first (`set -a; source .env; set +a; cargo run …`) or use
    `direnv` (the repo ships an `.envrc`).
  - In production / CI, inject `{name}` via the platform's secret
    manager or container env — there is no `.env` fallback.

See `crates/backend/be-monolith/README.md` for the full env-var surface."
    )]
    MissingEnv { name: &'static str },

    #[error(
        "LLM configuration is invalid.

{source}

Run with `EURORA_LLM_KIND=openai OPENAI_API_KEY=sk-... EURORA_CHAT_MODEL=gpt-4o-mini`
for the simplest path. For OpenAI-compatible servers, set
`EURORA_LLM_KIND=openai_compatible EURORA_LLM_BASE_URL=...`.

See `crates/backend/be-monolith/README.md` for the full env-var surface."
    )]
    LlmConfig {
        #[source]
        source: LlmConfigError,
    },

    #[error(
        "JWT signing secret `{name}` is unset or empty.

The backend signs and verifies sessions with this secret.

For local development, generate a random one and add it to your `.env`:
  {name}=$(openssl rand -hex 32)

For production, this MUST be a long random string and MUST NOT be a
placeholder. Rotating the secret invalidates every existing session."
    )]
    MissingJwtSecret { name: &'static str },

    #[error(
        "Invalid `{name}` value `{value}`: {source}

Expected an absolute URL like `http://localhost:3000` or `https://api.example.com`."
    )]
    InvalidUrl {
        name: &'static str,
        value: String,
        #[source]
        source: url::ParseError,
    },

    #[error(
        "Failed to construct backend bind address `{value}`: {source}

This usually means `BACKEND_URL` resolves to a port the OS rejected.
Pick a free TCP port (1024–65535) and update `BACKEND_URL` accordingly."
    )]
    InvalidBindAddr {
        value: String,
        #[source]
        source: std::net::AddrParseError,
    },

    #[error(
        "Invalid `AUTH_COOKIE_SECURE` value `{value}` (expected `true` or `false`).

Local dev sets `false` because the stack runs without TLS;
production deploys must set `true` so cookies are only sent over
HTTPS."
    )]
    InvalidCookieSecure { value: String },

    #[error(
        "Failed to bind HTTP listener at {addr}: {source}

Another process is already listening on that port. Run `just doctor` to
locate it, or update `BACKEND_URL` to a free port."
    )]
    BindFailed {
        addr: SocketAddr,
        #[source]
        source: std::io::Error,
    },

    #[error(
        "Failed to connect to PostgreSQL.

  {source}

If you're using `just dev`, check the postgres container is up and healthy:
  docker compose ps postgres
  docker compose logs postgres

Otherwise, verify `REMOTE_DATABASE_URL` points at a reachable Postgres
with the `eurora` database created."
    )]
    Database {
        #[source]
        source: anyhow::Error,
    },

    #[error(
        "Failed to apply database migrations.

  {source}

This usually means Postgres is reachable but rejected the migration —
check that the database is empty or that previous migrations recorded
in `_sqlx_migrations` aren't out of sync with the source files. If
you're using `just dev`, `just dev-reset` will wipe the volume and
re-apply migrations from scratch."
    )]
    Migration {
        #[source]
        source: anyhow::Error,
    },

    #[error(
        "Failed to load Casbin authorization policy from `{model_path}` / `{policy_path}`.

  {source}

`AUTHZ_MODEL_PATH` and `AUTHZ_POLICY_PATH` are resolved relative to
the backend's working directory. The values in `.env.example`
(`config/authz/model.conf`, `config/authz/policy.csv`) assume
`cargo run -p be-monolith` is invoked from the repo root; override
them if you run from a different directory or in a container."
    )]
    Authz {
        model_path: String,
        policy_path: String,
        #[source]
        source: anyhow::Error,
    },

    #[error(
        "Failed to initialise the email service.

  {source}

Email is required in production. To run without it, build with `cargo run`
(debug profile) — dev mode skips email-service init by design."
    )]
    EmailService {
        #[source]
        source: anyhow::Error,
    },

    #[error(
        "Failed to initialise the payment service.

  {source}

Stripe is required in production. To run without it, build with `cargo run`
(debug profile) — dev mode skips payment-service init by design."
    )]
    PaymentService {
        #[source]
        source: anyhow::Error,
    },

    #[error(
        "Failed to load asset storage configuration.

  {source}

For local development, set `ASSET_STORAGE_BACKEND=fs` and
`ASSET_STORAGE_FS_ROOT=../assets` in your `.env`."
    )]
    StorageConfig {
        #[source]
        source: anyhow::Error,
    },

    #[error(
        "Failed to initialise asset storage service.

  {source}"
    )]
    StorageService {
        #[source]
        source: anyhow::Error,
    },

    #[error(
        "Failed to wire the thread service from the LLM configuration.

  {source}

This typically means a role's provider id doesn't exist in the providers
map, or a provider kind is recognised in the schema but not yet wired in
the runtime. Check `be-thread-service::llm::providers` for supported
kinds."
    )]
    ThreadService {
        #[source]
        source: be_thread_service::BuildError,
    },

    #[error(
        "Failed to initialise the update service.

  {source}

The update service distributes desktop builds from S3. To run without it,
build with `cargo run` (debug profile) — dev mode skips update-service
init by design."
    )]
    UpdateService {
        #[source]
        source: anyhow::Error,
    },

    #[error("HTTP server error: {source}")]
    ServerRuntime {
        #[source]
        source: std::io::Error,
    },
}

impl From<llm_core::ConfigError> for BootstrapError {
    fn from(source: llm_core::ConfigError) -> Self {
        BootstrapError::LlmConfig { source }
    }
}

impl From<be_auth_core::JwtConfigError> for BootstrapError {
    fn from(value: be_auth_core::JwtConfigError) -> Self {
        match value {
            be_auth_core::JwtConfigError::MissingSecret { name } => {
                BootstrapError::MissingJwtSecret { name }
            }
        }
    }
}

impl From<be_thread_service::BuildError> for BootstrapError {
    fn from(source: be_thread_service::BuildError) -> Self {
        BootstrapError::ThreadService { source }
    }
}

impl From<be_auth_service::CookieConfigError> for BootstrapError {
    fn from(value: be_auth_service::CookieConfigError) -> Self {
        match value {
            be_auth_service::CookieConfigError::MissingEnv { name } => {
                BootstrapError::MissingEnv { name }
            }
            be_auth_service::CookieConfigError::InvalidCookieSecure { value } => {
                BootstrapError::InvalidCookieSecure { value }
            }
        }
    }
}
