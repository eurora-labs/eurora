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
