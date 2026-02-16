use async_trait::async_trait;
use std::sync::Mutex;
use std::time::Instant;

/// Base trait for rate limiters.
///
/// Usage of the base limiter is through the acquire and aacquire methods depending
/// on whether running in a sync or async context.
///
/// Implementations are free to add a timeout parameter to their initialize method
/// to allow users to specify a timeout for acquiring the necessary tokens when
/// using a blocking call.
///
/// Current limitations:
///
/// - Rate limiting information is not surfaced in tracing or callbacks. This means
///     that the total time it takes to invoke a chat model will encompass both
///     the time spent waiting for tokens and the time spent making the request.
#[async_trait]
pub trait BaseRateLimiter: Send + Sync {
    /// Attempt to acquire the necessary tokens for the rate limiter.
    ///
    /// This method blocks until the required tokens are available if `blocking`
    /// is set to `true`.
    ///
    /// If `blocking` is set to `false`, the method will immediately return the result
    /// of the attempt to acquire the tokens.
    ///
    /// # Arguments
    ///
    /// * `blocking` - If `true`, the method will block until the tokens are available.
    ///     If `false`, the method will return immediately with the result of
    ///     the attempt.
    ///
    /// # Returns
    ///
    /// `true` if the tokens were successfully acquired, `false` otherwise.
    fn acquire(&self, blocking: bool) -> bool;

    /// Attempt to acquire the necessary tokens for the rate limiter. Async version.
    ///
    /// This method blocks until the required tokens are available if `blocking`
    /// is set to `true`.
    ///
    /// If `blocking` is set to `false`, the method will immediately return the result
    /// of the attempt to acquire the tokens.
    ///
    /// # Arguments
    ///
    /// * `blocking` - If `true`, the method will block until the tokens are available.
    ///     If `false`, the method will return immediately with the result of
    ///     the attempt.
    ///
    /// # Returns
    ///
    /// `true` if the tokens were successfully acquired, `false` otherwise.
    async fn aacquire(&self, blocking: bool) -> bool;
}

/// Configuration for InMemoryRateLimiter.
#[derive(Debug, Clone)]
pub struct InMemoryRateLimiterConfig {
    /// The number of tokens to add per second to the bucket.
    /// The tokens represent "credit" that can be used to make requests.
    pub requests_per_second: f64,
    /// Check whether the tokens are available every this many seconds.
    /// Can be a float to represent fractions of a second.
    pub check_every_n_seconds: f64,
    /// The maximum number of tokens that can be in the bucket.
    /// Must be at least 1. Used to prevent bursts of requests.
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

/// An in memory rate limiter based on a token bucket algorithm.
///
/// This is an in memory rate limiter, so it cannot rate limit across
/// different processes.
///
/// The rate limiter only allows time-based rate limiting and does not
/// take into account any information about the input or the output, so it
/// cannot be used to rate limit based on the size of the request.
///
/// It is thread safe and can be used in either a sync or async context.
///
/// The in memory rate limiter is based on a token bucket. The bucket is filled
/// with tokens at a given rate. Each request consumes a token. If there are
/// not enough tokens in the bucket, the request is blocked until there are
/// enough tokens.
///
/// These tokens have nothing to do with LLM tokens. They are just
/// a way to keep track of how many requests can be made at a given time.
///
/// Current limitations:
///
/// - The rate limiter is not designed to work across different processes. It is
///   an in-memory rate limiter, but it is thread safe.
/// - The rate limiter only supports time-based rate limiting. It does not take
///   into account the size of the request or any other factors.
///
/// # Example
///
/// ```rust,ignore
/// use agent_chain_core::rate_limiters::{InMemoryRateLimiter, InMemoryRateLimiterConfig, BaseRateLimiter};
///
/// let rate_limiter = InMemoryRateLimiter::new(InMemoryRateLimiterConfig {
///     requests_per_second: 0.1,  // Can only make a request once every 10 seconds
///     check_every_n_seconds: 0.1,  // Wake up every 100 ms to check whether allowed to make a request
///     max_bucket_size: 10.0,  // Controls the maximum burst size
/// });
///
/// // In sync context
/// rate_limiter.acquire(true);
///
/// // In async context
/// rate_limiter.aacquire(true).await;
/// ```
pub struct InMemoryRateLimiter {
    requests_per_second: f64,
    max_bucket_size: f64,
    check_every_n_seconds: f64,
    state: Mutex<InMemoryRateLimiterState>,
}

impl InMemoryRateLimiter {
    /// Create a new InMemoryRateLimiter with the given configuration.
    ///
    /// These tokens have nothing to do with LLM tokens. They are just
    /// a way to keep track of how many requests can be made at a given time.
    ///
    /// This rate limiter is designed to work in a threaded environment.
    ///
    /// It works by filling up a bucket with tokens at a given rate. Each
    /// request consumes a given number of tokens. If there are not enough
    /// tokens in the bucket, the request is blocked until there are enough
    /// tokens.
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

    /// Try to consume a token.
    ///
    /// Returns `true` if the tokens were consumed and the caller can proceed to
    /// make the request. Returns `false` if the tokens were not consumed and
    /// the caller should try again later.
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
    /// Attempt to acquire a token from the rate limiter.
    ///
    /// This method blocks until the required tokens are available if `blocking`
    /// is set to `true`.
    ///
    /// If `blocking` is set to `false`, the method will immediately return the result
    /// of the attempt to acquire the tokens.
    ///
    /// # Arguments
    ///
    /// * `blocking` - If `true`, the method will block until the tokens are available.
    ///     If `false`, the method will return immediately with the result of
    ///     the attempt.
    ///
    /// # Returns
    ///
    /// `true` if the tokens were successfully acquired, `false` otherwise.
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

    /// Attempt to acquire a token from the rate limiter. Async version.
    ///
    /// This method blocks until the required tokens are available if `blocking`
    /// is set to `true`.
    ///
    /// If `blocking` is set to `false`, the method will immediately return the result
    /// of the attempt to acquire the tokens.
    ///
    /// # Arguments
    ///
    /// * `blocking` - If `true`, the method will block until the tokens are available.
    ///     If `false`, the method will return immediately with the result of
    ///     the attempt.
    ///
    /// # Returns
    ///
    /// `true` if the tokens were successfully acquired, `false` otherwise.
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
