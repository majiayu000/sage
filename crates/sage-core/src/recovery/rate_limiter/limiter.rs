//! Token bucket rate limiter implementation

use super::types::{RateLimitConfig, RateLimitError, RateLimitGuard};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};
use tokio::time::sleep;

/// Rate limiter using token bucket algorithm
#[derive(Debug)]
pub struct RateLimiter {
    /// Configuration
    config: RateLimitConfig,
    /// Current tokens available
    tokens: Arc<Mutex<f64>>,
    /// Last refill time
    last_refill: Arc<Mutex<Instant>>,
    /// Semaphore for concurrency limiting
    concurrent_semaphore: Arc<Semaphore>,
}

impl RateLimiter {
    /// Create a new rate limiter with default configuration
    pub fn new() -> Self {
        Self::with_config(RateLimitConfig::default())
    }

    /// Create a new rate limiter with custom configuration
    pub fn with_config(config: RateLimitConfig) -> Self {
        let tokens = config.burst_size as f64;
        Self {
            concurrent_semaphore: Arc::new(Semaphore::new(if config.max_concurrent == 0 {
                Semaphore::MAX_PERMITS
            } else {
                config.max_concurrent as usize
            })),
            config,
            tokens: Arc::new(Mutex::new(tokens)),
            last_refill: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// Refill tokens based on elapsed time
    async fn refill(&self) {
        let mut tokens = self.tokens.lock().await;
        let mut last_refill = self.last_refill.lock().await;

        let now = Instant::now();
        let elapsed = now.duration_since(*last_refill).as_secs_f64();
        let new_tokens = elapsed * self.config.requests_per_second();

        *tokens = (*tokens + new_tokens).min(self.config.burst_size as f64);
        *last_refill = now;
    }

    /// Try to acquire a token without waiting
    pub async fn try_acquire(&self) -> Option<RateLimitGuard> {
        // Check concurrency limit
        let permit = self.concurrent_semaphore.clone().try_acquire_owned().ok()?;

        // Check token bucket
        self.refill().await;
        let mut tokens = self.tokens.lock().await;

        if *tokens >= 1.0 {
            *tokens -= 1.0;
            Some(RateLimitGuard {
                _permit: Some(permit),
            })
        } else {
            // Return permit if we can't get a token
            drop(permit);
            None
        }
    }

    /// Acquire a token, waiting if necessary
    pub async fn acquire(&self) -> Result<RateLimitGuard, RateLimitError> {
        let start = Instant::now();

        // First acquire concurrency permit
        let permit = if self.config.blocking {
            match tokio::time::timeout(
                self.config.max_wait,
                self.concurrent_semaphore.clone().acquire_owned(),
            )
            .await
            {
                Ok(Ok(permit)) => permit,
                Ok(Err(_)) => return Err(RateLimitError::Closed),
                Err(_) => {
                    return Err(RateLimitError::Timeout {
                        waited: start.elapsed(),
                    });
                }
            }
        } else {
            match self.concurrent_semaphore.clone().try_acquire_owned() {
                Ok(permit) => permit,
                Err(_) => {
                    return Err(RateLimitError::ConcurrencyExceeded {
                        max: self.config.max_concurrent,
                    });
                }
            }
        };

        // Now acquire token bucket token
        loop {
            if start.elapsed() >= self.config.max_wait {
                return Err(RateLimitError::Timeout {
                    waited: start.elapsed(),
                });
            }

            self.refill().await;
            let mut tokens = self.tokens.lock().await;

            if *tokens >= 1.0 {
                *tokens -= 1.0;
                return Ok(RateLimitGuard {
                    _permit: Some(permit),
                });
            }

            if !self.config.blocking {
                return Err(RateLimitError::WouldBlock);
            }

            // Calculate wait time for next token
            let tokens_needed = 1.0 - *tokens;
            let wait_secs = tokens_needed / self.config.requests_per_second();
            let wait_duration = Duration::from_secs_f64(wait_secs).min(Duration::from_millis(100));

            drop(tokens);
            sleep(wait_duration).await;
        }
    }

    /// Get current available tokens
    pub async fn available_tokens(&self) -> f64 {
        self.refill().await;
        *self.tokens.lock().await
    }

    /// Get current concurrency usage
    pub fn concurrent_requests(&self) -> usize {
        {
            let total = if self.config.max_concurrent == 0 {
                Semaphore::MAX_PERMITS
            } else {
                self.config.max_concurrent as usize
            };
            total - self.concurrent_semaphore.available_permits()
        }
    }

    /// Check if rate limited (would need to wait)
    #[cfg(test)]
    pub async fn is_limited(&self) -> bool {
        self.available_tokens().await < 1.0
    }

    /// Get configuration
    pub fn config(&self) -> &RateLimitConfig {
        &self.config
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}
