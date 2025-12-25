//! Alternative rate limiting strategies

use super::types::RateLimitError;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::sleep;

/// Sliding window rate limiter for more accurate rate limiting
#[derive(Debug)]
pub struct SlidingWindowRateLimiter {
    /// Window size
    window_size: Duration,
    /// Maximum requests per window
    max_requests: u32,
    /// Request timestamps
    timestamps: Arc<Mutex<VecDeque<Instant>>>,
    /// Maximum wait time
    max_wait: Duration,
}

impl SlidingWindowRateLimiter {
    /// Create a new sliding window rate limiter
    pub fn new(max_requests: u32, window_size: Duration) -> Self {
        Self {
            window_size,
            max_requests,
            timestamps: Arc::new(Mutex::new(VecDeque::new())),
            max_wait: Duration::from_secs(60),
        }
    }

    /// Create with requests per minute
    pub fn per_minute(requests: u32) -> Self {
        Self::new(requests, Duration::from_secs(60))
    }

    /// Create with requests per second
    pub fn per_second(requests: u32) -> Self {
        Self::new(requests, Duration::from_secs(1))
    }

    /// Set max wait time
    pub fn with_max_wait(mut self, max_wait: Duration) -> Self {
        self.max_wait = max_wait;
        self
    }

    /// Clean up old timestamps
    async fn cleanup(&self) {
        let mut timestamps = self.timestamps.lock().await;
        let cutoff = Instant::now() - self.window_size;

        while let Some(front) = timestamps.front() {
            if *front < cutoff {
                timestamps.pop_front();
            } else {
                break;
            }
        }
    }

    /// Try to record a request without waiting
    pub async fn try_record(&self) -> bool {
        self.cleanup().await;

        let mut timestamps = self.timestamps.lock().await;
        if timestamps.len() < self.max_requests as usize {
            timestamps.push_back(Instant::now());
            true
        } else {
            false
        }
    }

    /// Record a request, waiting if necessary
    pub async fn record(&self) -> Result<(), RateLimitError> {
        let start = Instant::now();

        loop {
            if start.elapsed() >= self.max_wait {
                return Err(RateLimitError::Timeout {
                    waited: start.elapsed(),
                });
            }

            self.cleanup().await;

            let mut timestamps = self.timestamps.lock().await;
            if timestamps.len() < self.max_requests as usize {
                timestamps.push_back(Instant::now());
                return Ok(());
            }

            // Calculate wait time until oldest request expires
            if let Some(oldest) = timestamps.front() {
                let age = Instant::now().duration_since(*oldest);
                if age < self.window_size {
                    let wait = self.window_size - age;
                    drop(timestamps);
                    sleep(wait.min(Duration::from_millis(100))).await;
                }
            }
        }
    }

    /// Get current request count in window
    pub async fn current_count(&self) -> usize {
        self.cleanup().await;
        self.timestamps.lock().await.len()
    }

    /// Check if rate limited
    pub async fn is_limited(&self) -> bool {
        self.cleanup().await;
        self.timestamps.lock().await.len() >= self.max_requests as usize
    }
}
