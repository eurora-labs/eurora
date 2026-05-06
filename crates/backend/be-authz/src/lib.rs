mod axum_layer;
mod bypass;
mod enforcer;
mod error;
mod http_token_gate;
mod origin_guard;
mod rate_limit;
mod token_gate;

pub use axum_layer::{AuthzState, authz_middleware};
pub use be_auth_core::*;
pub use enforcer::CasbinAuthz;
pub use error::AuthzError;
pub use http_token_gate::{HttpTokenGateState, http_token_gate_middleware};
pub use origin_guard::{OriginGuardConfig, origin_guard_middleware};
pub use rate_limit::{
    AuthFailureRateLimiter, HealthCheckRateLimiter, TrustedProxies, extract_client_ip,
    new_auth_failure_rate_limiter, new_health_check_rate_limiter,
};
pub use token_gate::{TokenGateError, TokenUsageRepo};
