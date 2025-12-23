//! Internal state tracking for models

use std::time::Instant;

use super::types::ModelConfig;

/// State tracking for a model
#[derive(Debug, Clone)]
pub(super) struct ModelState {
    /// Configuration
    pub config: ModelConfig,
    /// Last failure time
    pub last_failure: Option<Instant>,
    /// Consecutive failure count
    pub failure_count: u32,
    /// Total requests
    pub total_requests: u64,
    /// Successful requests
    pub successful_requests: u64,
}

impl ModelState {
    pub fn new(config: ModelConfig) -> Self {
        Self {
            config,
            last_failure: None,
            failure_count: 0,
            total_requests: 0,
            successful_requests: 0,
        }
    }

    pub fn is_available(&self) -> bool {
        if !self.config.healthy {
            return false;
        }

        match self.last_failure {
            Some(time) => time.elapsed() > self.config.cooldown,
            None => true,
        }
    }

    pub fn record_success(&mut self) {
        self.failure_count = 0;
        self.total_requests += 1;
        self.successful_requests += 1;
    }

    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.total_requests += 1;
        self.last_failure = Some(Instant::now());
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            1.0
        } else {
            self.successful_requests as f64 / self.total_requests as f64
        }
    }
}
