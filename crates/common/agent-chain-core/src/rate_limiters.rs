use async_trait::async_trait;
use governor::{Quota, RateLimiter, clock::DefaultClock, state::InMemoryState, state::NotKeyed};
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;

#[async_trait]
pub trait BaseRateLimiter: Send + Sync {
    fn acquire(&self, blocking: bool) -> bool;

    async fn aacquire(&self, blocking: bool) -> bool;
}

#[derive(Debug, Clone)]
pub struct InMemoryRateLimiterConfig {
    pub requests_per_second: f64,
    pub check_every_n_seconds: f64,
    pub max_bucket_size: f64,
}

impl Default for InMemoryRateLimiterConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 1.0,
            check_every_n_seconds: 0.1,
            max_bucket_size: 1.0,
        }
    }
}

pub struct InMemoryRateLimiter {
    limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
    check_every_n_seconds: f64,
}

impl InMemoryRateLimiter {
    pub fn new(config: InMemoryRateLimiterConfig) -> Self {
        let burst = config.max_bucket_size.ceil().max(1.0) as u32;
        let burst = NonZeroU32::new(burst).unwrap_or(NonZeroU32::new(1).unwrap());

        let period = Duration::from_secs_f64(1.0 / config.requests_per_second);
        let quota = Quota::with_period(period)
            .expect("valid rate limiter period")
            .allow_burst(burst);

        let limiter = RateLimiter::direct(quota);

        // Drain initial burst tokens to match the old behavior where
        // no tokens are available until time has passed.
        for _ in 0..burst.get() {
            let _ = limiter.check();
        }

        Self {
            limiter: Arc::new(limiter),
            check_every_n_seconds: config.check_every_n_seconds,
        }
    }
}

#[async_trait]
impl BaseRateLimiter for InMemoryRateLimiter {
    fn acquire(&self, blocking: bool) -> bool {
        if !blocking {
            return self.limiter.check().is_ok();
        }

        while self.limiter.check().is_err() {
            std::thread::sleep(Duration::from_secs_f64(self.check_every_n_seconds));
        }
        true
    }

    async fn aacquire(&self, blocking: bool) -> bool {
        if !blocking {
            return self.limiter.check().is_ok();
        }

        while self.limiter.check().is_err() {
            tokio::time::sleep(Duration::from_secs_f64(self.check_every_n_seconds)).await;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_rate_limiter_non_blocking() {
        let rate_limiter = InMemoryRateLimiter::new(InMemoryRateLimiterConfig {
            requests_per_second: 10.0,
            check_every_n_seconds: 0.01,
            max_bucket_size: 1.0,
        });

        // Drain the initial burst token
        let _ = rate_limiter.acquire(false);
        let result = rate_limiter.acquire(false);
        assert!(!result);
    }

    #[test]
    fn test_rate_limiter_blocking() {
        let rate_limiter = InMemoryRateLimiter::new(InMemoryRateLimiterConfig {
            requests_per_second: 100.0,
            check_every_n_seconds: 0.001,
            max_bucket_size: 1.0,
        });

        let start = Instant::now();
        let result = rate_limiter.acquire(true);
        let elapsed = start.elapsed();

        assert!(result);
        assert!(elapsed.as_millis() < 100);
    }

    #[tokio::test]
    async fn test_rate_limiter_async_non_blocking() {
        let rate_limiter = InMemoryRateLimiter::new(InMemoryRateLimiterConfig {
            requests_per_second: 10.0,
            check_every_n_seconds: 0.01,
            max_bucket_size: 1.0,
        });

        let _ = rate_limiter.aacquire(false).await;
        let result = rate_limiter.aacquire(false).await;
        assert!(!result);
    }

    #[tokio::test]
    async fn test_rate_limiter_async_blocking() {
        let rate_limiter = InMemoryRateLimiter::new(InMemoryRateLimiterConfig {
            requests_per_second: 100.0,
            check_every_n_seconds: 0.001,
            max_bucket_size: 1.0,
        });

        let start = Instant::now();
        let result = rate_limiter.aacquire(true).await;
        let elapsed = start.elapsed();

        assert!(result);
        assert!(elapsed.as_millis() < 100);
    }

    #[test]
    fn test_rate_limiter_burst() {
        let rate_limiter = InMemoryRateLimiter::new(InMemoryRateLimiterConfig {
            requests_per_second: 1000.0,
            check_every_n_seconds: 0.001,
            max_bucket_size: 5.0,
        });

        std::thread::sleep(std::time::Duration::from_millis(10));

        let mut successes = 0;
        for _ in 0..10 {
            if rate_limiter.acquire(false) {
                successes += 1;
            }
        }

        assert!(successes <= 5);
    }

    #[test]
    fn test_default_config() {
        let config = InMemoryRateLimiterConfig::default();
        assert!((config.requests_per_second - 1.0).abs() < f64::EPSILON);
        assert!((config.check_every_n_seconds - 0.1).abs() < f64::EPSILON);
        assert!((config.max_bucket_size - 1.0).abs() < f64::EPSILON);
    }
}
