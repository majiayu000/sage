//! Backoff strategies for retry operations
//!
//! Provides configurable backoff algorithms for handling transient failures.

use std::time::Duration;

/// Configuration for backoff behavior
#[derive(Debug, Clone)]
pub struct BackoffConfig {
    /// Initial delay before first retry
    pub initial_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Multiplier for exponential backoff
    pub multiplier: f64,
    /// Add random jitter to prevent thundering herd
    pub jitter: bool,
    /// Maximum jitter ratio (0.0 - 1.0)
    pub jitter_ratio: f64,
}

impl Default for BackoffConfig {
    fn default() -> Self {
        Self {
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(60),
            multiplier: 2.0,
            jitter: true,
            jitter_ratio: 0.2,
        }
    }
}

impl BackoffConfig {
    /// Create a new backoff config with custom initial delay
    pub fn with_initial_delay(initial_delay: Duration) -> Self {
        Self {
            initial_delay,
            ..Default::default()
        }
    }

    /// Set the maximum delay
    pub fn max_delay(mut self, max_delay: Duration) -> Self {
        self.max_delay = max_delay;
        self
    }

    /// Set the multiplier
    pub fn multiplier(mut self, multiplier: f64) -> Self {
        self.multiplier = multiplier;
        self
    }

    /// Enable or disable jitter
    pub fn jitter(mut self, jitter: bool) -> Self {
        self.jitter = jitter;
        self
    }

    /// Create config optimized for aggressive retries
    pub fn aggressive() -> Self {
        Self {
            initial_delay: Duration::from_millis(50),
            max_delay: Duration::from_secs(5),
            multiplier: 1.5,
            jitter: true,
            jitter_ratio: 0.1,
        }
    }

    /// Create config optimized for rate-limited APIs
    pub fn rate_limited() -> Self {
        Self {
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(300),
            multiplier: 2.0,
            jitter: true,
            jitter_ratio: 0.3,
        }
    }
}

/// Backoff strategy trait
pub trait BackoffStrategy: Send + Sync {
    /// Get the delay for the given attempt number (0-indexed)
    fn delay_for_attempt(&self, attempt: u32) -> Duration;

    /// Reset the backoff state
    fn reset(&mut self);
}

/// Exponential backoff implementation
#[derive(Debug, Clone)]
pub struct ExponentialBackoff {
    config: BackoffConfig,
    current_delay: Duration,
    attempt: u32,
}

impl ExponentialBackoff {
    /// Create a new exponential backoff with default config
    pub fn new() -> Self {
        Self::with_config(BackoffConfig::default())
    }

    /// Create a new exponential backoff with custom config
    pub fn with_config(config: BackoffConfig) -> Self {
        let current_delay = config.initial_delay;
        Self {
            config,
            current_delay,
            attempt: 0,
        }
    }

    /// Get the next delay and advance the attempt counter
    pub fn next_delay(&mut self) -> Duration {
        let delay = self.delay_for_attempt(self.attempt);
        self.attempt += 1;
        delay
    }

    fn add_jitter(&self, delay: Duration) -> Duration {
        if !self.config.jitter {
            return delay;
        }

        let jitter_range = delay.as_secs_f64() * self.config.jitter_ratio;
        let jitter = rand_jitter(jitter_range);
        let jittered = delay.as_secs_f64() + jitter;

        Duration::from_secs_f64(jittered.max(0.0))
    }
}

impl Default for ExponentialBackoff {
    fn default() -> Self {
        Self::new()
    }
}

impl BackoffStrategy for ExponentialBackoff {
    fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let base_delay = self.config.initial_delay.as_secs_f64()
            * self.config.multiplier.powi(attempt as i32);

        let capped_delay = Duration::from_secs_f64(base_delay.min(self.config.max_delay.as_secs_f64()));

        self.add_jitter(capped_delay)
    }

    fn reset(&mut self) {
        self.current_delay = self.config.initial_delay;
        self.attempt = 0;
    }
}

/// Constant backoff - same delay for all attempts
#[derive(Debug, Clone)]
pub struct ConstantBackoff {
    delay: Duration,
}

impl ConstantBackoff {
    pub fn new(delay: Duration) -> Self {
        Self { delay }
    }
}

impl BackoffStrategy for ConstantBackoff {
    fn delay_for_attempt(&self, _attempt: u32) -> Duration {
        self.delay
    }

    fn reset(&mut self) {
        // No state to reset
    }
}

/// Linear backoff - delay increases linearly
#[derive(Debug, Clone)]
pub struct LinearBackoff {
    initial_delay: Duration,
    increment: Duration,
    max_delay: Duration,
}

impl LinearBackoff {
    pub fn new(initial_delay: Duration, increment: Duration, max_delay: Duration) -> Self {
        Self {
            initial_delay,
            increment,
            max_delay,
        }
    }
}

impl BackoffStrategy for LinearBackoff {
    fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let delay = self.initial_delay + self.increment * attempt;
        delay.min(self.max_delay)
    }

    fn reset(&mut self) {
        // No state to reset
    }
}

/// Decorrelated jitter backoff (AWS style)
/// Better distribution than simple exponential with jitter
#[derive(Debug, Clone)]
pub struct DecorrelatedJitterBackoff {
    base: Duration,
    cap: Duration,
    last_delay: Duration,
}

impl DecorrelatedJitterBackoff {
    pub fn new(base: Duration, cap: Duration) -> Self {
        Self {
            base,
            cap,
            last_delay: base,
        }
    }
}

impl BackoffStrategy for DecorrelatedJitterBackoff {
    fn delay_for_attempt(&self, _attempt: u32) -> Duration {
        // sleep = min(cap, random_between(base, sleep * 3))
        let min = self.base.as_secs_f64();
        let max = self.last_delay.as_secs_f64() * 3.0;

        let delay = min + rand_jitter(max - min);
        let capped = delay.min(self.cap.as_secs_f64());

        Duration::from_secs_f64(capped)
    }

    fn reset(&mut self) {
        self.last_delay = self.base;
    }
}

/// Simple pseudo-random jitter generator
/// In production, consider using a proper RNG
fn rand_jitter(range: f64) -> f64 {
    use std::time::SystemTime;

    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);

    // Simple hash-based pseudo-random
    let hash = nanos.wrapping_mul(2654435761);
    let normalized = (hash as f64) / (u32::MAX as f64);

    normalized * range
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exponential_backoff_delays() {
        let config = BackoffConfig {
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            multiplier: 2.0,
            jitter: false,
            jitter_ratio: 0.0,
        };

        let backoff = ExponentialBackoff::with_config(config);

        assert_eq!(backoff.delay_for_attempt(0), Duration::from_millis(100));
        assert_eq!(backoff.delay_for_attempt(1), Duration::from_millis(200));
        assert_eq!(backoff.delay_for_attempt(2), Duration::from_millis(400));
        assert_eq!(backoff.delay_for_attempt(3), Duration::from_millis(800));
    }

    #[test]
    fn test_exponential_backoff_cap() {
        let config = BackoffConfig {
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(5),
            multiplier: 2.0,
            jitter: false,
            jitter_ratio: 0.0,
        };

        let backoff = ExponentialBackoff::with_config(config);

        // Attempt 10 would be 2^10 = 1024 seconds, but should be capped at 5
        assert_eq!(backoff.delay_for_attempt(10), Duration::from_secs(5));
    }

    #[test]
    fn test_constant_backoff() {
        let backoff = ConstantBackoff::new(Duration::from_secs(1));

        assert_eq!(backoff.delay_for_attempt(0), Duration::from_secs(1));
        assert_eq!(backoff.delay_for_attempt(5), Duration::from_secs(1));
        assert_eq!(backoff.delay_for_attempt(100), Duration::from_secs(1));
    }

    #[test]
    fn test_linear_backoff() {
        let backoff = LinearBackoff::new(
            Duration::from_millis(100),
            Duration::from_millis(100),
            Duration::from_secs(1),
        );

        assert_eq!(backoff.delay_for_attempt(0), Duration::from_millis(100));
        assert_eq!(backoff.delay_for_attempt(1), Duration::from_millis(200));
        assert_eq!(backoff.delay_for_attempt(2), Duration::from_millis(300));

        // Should be capped at 1 second
        assert_eq!(backoff.delay_for_attempt(20), Duration::from_secs(1));
    }

    #[test]
    fn test_backoff_reset() {
        let mut backoff = ExponentialBackoff::new();

        // Advance several attempts
        let _ = backoff.next_delay();
        let _ = backoff.next_delay();
        let _ = backoff.next_delay();

        assert_eq!(backoff.attempt, 3);

        backoff.reset();
        assert_eq!(backoff.attempt, 0);
    }
}
