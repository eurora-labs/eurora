use std::{net::IpAddr, num::NonZeroU32, sync::Arc};

use governor::{Quota, RateLimiter, clock::DefaultClock, state::keyed::DefaultKeyedStateStore};

pub type AuthFailureRateLimiter =
    Arc<RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>>;

pub type HealthCheckRateLimiter =
    Arc<RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>>;

pub fn new_auth_failure_rate_limiter() -> AuthFailureRateLimiter {
    Arc::new(RateLimiter::keyed(
        Quota::per_minute(NonZeroU32::new(50).unwrap()).allow_burst(NonZeroU32::new(20).unwrap()),
    ))
}

pub fn new_health_check_rate_limiter() -> HealthCheckRateLimiter {
    Arc::new(RateLimiter::keyed(
        Quota::per_second(NonZeroU32::new(20).unwrap()).allow_burst(NonZeroU32::new(50).unwrap()),
    ))
}

/// Extract the real client IP from request headers and TCP peer address.
///
/// Checks (in order):
/// 1. `X-Forwarded-For` header — uses the leftmost (original client) IP
/// 2. `X-Real-Ip` header
/// 3. TCP peer address from `ConnectInfo`
/// 4. Falls back to `127.0.0.1`
///
/// **Important:** The reverse proxy MUST strip or overwrite client-supplied
/// `X-Forwarded-For` headers to prevent spoofing.
pub fn extract_client_ip(headers: &http::HeaderMap, peer_addr: Option<IpAddr>) -> IpAddr {
    if let Some(forwarded) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok())
        && let Some(first_ip) = forwarded.split(',').next()
        && let Ok(ip) = first_ip.trim().parse::<IpAddr>()
    {
        return ip;
    }

    if let Some(real_ip) = headers.get("x-real-ip").and_then(|v| v.to_str().ok())
        && let Ok(ip) = real_ip.trim().parse::<IpAddr>()
    {
        return ip;
    }

    peer_addr.unwrap_or(IpAddr::from([127, 0, 0, 1]))
}
