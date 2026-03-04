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

pub struct InMemoryRateLimiter {
    limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
    check_interval: Duration,
}

#[bon::bon]
impl InMemoryRateLimiter {
    #[builder]
    pub fn new(
        #[builder(default = 1.0)] requests_per_second: f64,
        #[builder(default = 0.1)] check_every_n_seconds: f64,
        #[builder(default = 1.0)] max_bucket_size: f64,
    ) -> Self {
        let burst = max_bucket_size.ceil().max(1.0) as u32;
        let burst = NonZeroU32::new(burst).unwrap_or(NonZeroU32::new(1).unwrap());

        let period = Duration::from_secs_f64(1.0 / requests_per_second);
        let quota = Quota::with_period(period)
            .expect("valid rate limiter period")
            .allow_burst(burst);

        Self {
            limiter: Arc::new(RateLimiter::direct(quota)),
            check_interval: Duration::from_secs_f64(check_every_n_seconds),
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
            std::thread::sleep(self.check_interval);
        }
        true
    }

    async fn aacquire(&self, blocking: bool) -> bool {
        if !blocking {
            return self.limiter.check().is_ok();
        }

        self.limiter.until_ready().await;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_rate_limiter_non_blocking() {
        let rate_limiter = InMemoryRateLimiter::builder()
            .requests_per_second(10.0)
            .check_every_n_seconds(0.01)
            .build();

        assert!(rate_limiter.acquire(false));
        assert!(!rate_limiter.acquire(false));
    }

    #[test]
    fn test_rate_limiter_blocking() {
        let rate_limiter = InMemoryRateLimiter::builder()
            .requests_per_second(100.0)
            .check_every_n_seconds(0.001)
            .build();

        let start = Instant::now();
        assert!(rate_limiter.acquire(true));
        assert!(start.elapsed().as_millis() < 50);

        let start = Instant::now();
        assert!(rate_limiter.acquire(true));
        assert!(start.elapsed().as_millis() >= 5);
    }

    #[tokio::test]
    async fn test_rate_limiter_async_non_blocking() {
        let rate_limiter = InMemoryRateLimiter::builder()
            .requests_per_second(10.0)
            .check_every_n_seconds(0.01)
            .build();

        assert!(rate_limiter.aacquire(false).await);
        assert!(!rate_limiter.aacquire(false).await);
    }

    #[tokio::test]
    async fn test_rate_limiter_async_blocking() {
        let rate_limiter = InMemoryRateLimiter::builder()
            .requests_per_second(100.0)
            .check_every_n_seconds(0.001)
            .build();

        let start = Instant::now();
        assert!(rate_limiter.aacquire(true).await);
        assert!(start.elapsed().as_millis() < 50);

        let start = Instant::now();
        assert!(rate_limiter.aacquire(true).await);
        assert!(start.elapsed().as_millis() >= 5);
    }

    #[test]
    fn test_rate_limiter_burst() {
        let rate_limiter = InMemoryRateLimiter::builder()
            .requests_per_second(10.0)
            .check_every_n_seconds(0.001)
            .max_bucket_size(5.0)
            .build();

        let mut successes = 0;
        for _ in 0..10 {
            if rate_limiter.acquire(false) {
                successes += 1;
            }
        }

        assert_eq!(successes, 5);
    }

    #[test]
    fn test_default_config() {
        let rate_limiter = InMemoryRateLimiter::builder().build();
        assert!(rate_limiter.acquire(false));
        assert!(!rate_limiter.acquire(false));
    }
}
