//! Aggregator for multiple streaming sessions

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use super::counter::StreamingTokenCounter;
use super::types::AggregatedStats;

/// Aggregator for multiple streaming sessions
#[derive(Debug)]
pub struct StreamingMetrics {
    /// Total sessions
    session_count: AtomicUsize,
    /// Total tokens across all sessions
    total_tokens: AtomicUsize,
    /// Total time to first token (milliseconds)
    total_ttft_ms: AtomicUsize,
    /// Sessions with TTFT recorded
    ttft_count: AtomicUsize,
    /// Minimum TTFT (milliseconds)
    min_ttft_ms: AtomicUsize,
    /// Maximum TTFT (milliseconds)
    max_ttft_ms: AtomicUsize,
}

impl StreamingMetrics {
    /// Create new metrics aggregator
    pub fn new() -> Self {
        Self {
            session_count: AtomicUsize::new(0),
            total_tokens: AtomicUsize::new(0),
            total_ttft_ms: AtomicUsize::new(0),
            ttft_count: AtomicUsize::new(0),
            min_ttft_ms: AtomicUsize::new(usize::MAX),
            max_ttft_ms: AtomicUsize::new(0),
        }
    }

    /// Record stats from a streaming session
    pub async fn record(&self, counter: &StreamingTokenCounter) {
        self.session_count.fetch_add(1, Ordering::SeqCst);
        self.total_tokens
            .fetch_add(counter.estimated_tokens(), Ordering::SeqCst);

        if let Some(ttft) = counter.time_to_first_token().await {
            let ttft_ms = u64::try_from(ttft.as_millis()).unwrap_or(u64::MAX) as usize;
            self.total_ttft_ms.fetch_add(ttft_ms, Ordering::SeqCst);
            self.ttft_count.fetch_add(1, Ordering::SeqCst);

            // Update min/max (using compare-exchange loop for atomicity)
            loop {
                let current = self.min_ttft_ms.load(Ordering::SeqCst);
                if ttft_ms >= current {
                    break;
                }
                if self
                    .min_ttft_ms
                    .compare_exchange(current, ttft_ms, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    break;
                }
            }

            loop {
                let current = self.max_ttft_ms.load(Ordering::SeqCst);
                if ttft_ms <= current {
                    break;
                }
                if self
                    .max_ttft_ms
                    .compare_exchange(current, ttft_ms, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    break;
                }
            }
        }
    }

    /// Get total session count
    pub fn session_count(&self) -> usize {
        self.session_count.load(Ordering::SeqCst)
    }

    /// Get total tokens across all sessions
    pub fn total_tokens(&self) -> usize {
        self.total_tokens.load(Ordering::SeqCst)
    }

    /// Get average tokens per session
    pub fn avg_tokens_per_session(&self) -> f64 {
        let sessions = self.session_count();
        if sessions > 0 {
            self.total_tokens() as f64 / sessions as f64
        } else {
            0.0
        }
    }

    /// Get average time to first token
    pub fn avg_ttft(&self) -> Option<Duration> {
        let count = self.ttft_count.load(Ordering::SeqCst);
        if count > 0 {
            let total_ms = self.total_ttft_ms.load(Ordering::SeqCst);
            Some(Duration::from_millis((total_ms / count) as u64))
        } else {
            None
        }
    }

    /// Get minimum time to first token
    pub fn min_ttft(&self) -> Option<Duration> {
        let min = self.min_ttft_ms.load(Ordering::SeqCst);
        if min == usize::MAX {
            None
        } else {
            Some(Duration::from_millis(min as u64))
        }
    }

    /// Get maximum time to first token
    pub fn max_ttft(&self) -> Option<Duration> {
        let max = self.max_ttft_ms.load(Ordering::SeqCst);
        if max == 0 {
            None
        } else {
            Some(Duration::from_millis(max as u64))
        }
    }

    /// Get aggregated statistics
    pub fn summary(&self) -> AggregatedStats {
        AggregatedStats {
            session_count: self.session_count(),
            total_tokens: self.total_tokens(),
            avg_tokens_per_session: self.avg_tokens_per_session(),
            avg_ttft: self.avg_ttft(),
            min_ttft: self.min_ttft(),
            max_ttft: self.max_ttft(),
        }
    }

    /// Reset all metrics
    pub fn reset(&self) {
        self.session_count.store(0, Ordering::SeqCst);
        self.total_tokens.store(0, Ordering::SeqCst);
        self.total_ttft_ms.store(0, Ordering::SeqCst);
        self.ttft_count.store(0, Ordering::SeqCst);
        self.min_ttft_ms.store(usize::MAX, Ordering::SeqCst);
        self.max_ttft_ms.store(0, Ordering::SeqCst);
    }
}

impl Default for StreamingMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe wrapper for shared streaming metrics
pub type SharedStreamingMetrics = Arc<StreamingMetrics>;
