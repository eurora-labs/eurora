use async_trait::async_trait;
use std::sync::Mutex;
use std::time::Instant;

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

struct InMemoryRateLimiterState {
    available_tokens: f64,
    last: Option<Instant>,
}

pub struct InMemoryRateLimiter {
    requests_per_second: f64,
    max_bucket_size: f64,
    check_every_n_seconds: f64,
    state: Mutex<InMemoryRateLimiterState>,
}

impl InMemoryRateLimiter {
    pub fn new(config: InMemoryRateLimiterConfig) -> Self {
        Self {
            requests_per_second: config.requests_per_second,
            max_bucket_size: config.max_bucket_size,
            check_every_n_seconds: config.check_every_n_seconds,
            state: Mutex::new(InMemoryRateLimiterState {
                available_tokens: 0.0,
                last: None,
            }),
        }
    }

    fn consume(&self) -> bool {
        let mut state = match self.state.lock() {
            Ok(guard) => guard,
            Err(error) => {
                tracing::error!("Rate limiter lock poisoned: {}", error);
                return false;
            }
        };
        let now = Instant::now();

        if let Some(last) = state.last {
            let elapsed = now.duration_since(last).as_secs_f64();

            if elapsed * self.requests_per_second >= 1.0 {
                state.available_tokens += elapsed * self.requests_per_second;
                state.last = Some(now);
            }
        } else {
            state.last = Some(now);
        }

        state.available_tokens = state.available_tokens.min(self.max_bucket_size);

        if state.available_tokens >= 1.0 {
            state.available_tokens -= 1.0;
            return true;
        }

        false
    }
}

#[async_trait]
impl BaseRateLimiter for InMemoryRateLimiter {
    fn acquire(&self, blocking: bool) -> bool {
        if !blocking {
            return self.consume();
        }

        while !self.consume() {
            std::thread::sleep(std::time::Duration::from_secs_f64(
                self.check_every_n_seconds,
            ));
        }
        true
    }

    async fn aacquire(&self, blocking: bool) -> bool {
        if !blocking {
            return self.consume();
        }

        while !self.consume() {
            tokio::time::sleep(std::time::Duration::from_secs_f64(
                self.check_every_n_seconds,
            ))
            .await;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_non_blocking() {
        let rate_limiter = InMemoryRateLimiter::new(InMemoryRateLimiterConfig {
            requests_per_second: 10.0,
            check_every_n_seconds: 0.01,
            max_bucket_size: 1.0,
        });

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
