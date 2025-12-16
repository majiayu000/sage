//! Streaming token counter
//!
//! This module provides real-time token counting during streaming LLM responses.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Counter for tracking tokens during streaming
#[derive(Debug)]
pub struct StreamingTokenCounter {
    /// Estimated characters per token
    chars_per_token: f32,
    /// Current character count
    char_count: AtomicUsize,
    /// Estimated token count
    estimated_tokens: AtomicUsize,
    /// Chunks received
    chunk_count: AtomicUsize,
    /// Start time
    start_time: RwLock<Option<Instant>>,
    /// First token time
    first_token_time: RwLock<Option<Instant>>,
}

impl StreamingTokenCounter {
    /// Create a new streaming counter
    pub fn new() -> Self {
        Self {
            chars_per_token: 4.0,
            char_count: AtomicUsize::new(0),
            estimated_tokens: AtomicUsize::new(0),
            chunk_count: AtomicUsize::new(0),
            start_time: RwLock::new(None),
            first_token_time: RwLock::new(None),
        }
    }

    /// Create with custom chars per token estimate
    pub fn with_chars_per_token(mut self, chars_per_token: f32) -> Self {
        self.chars_per_token = chars_per_token;
        self
    }

    /// Create for a specific provider
    pub fn for_provider(provider: &str) -> Self {
        let chars_per_token = match provider.to_lowercase().as_str() {
            "anthropic" => 3.5,
            "openai" => 4.0,
            "google" => 4.0,
            _ => 4.0,
        };
        Self::new().with_chars_per_token(chars_per_token)
    }

    /// Start the counter (call before streaming begins)
    pub async fn start(&self) {
        let mut start = self.start_time.write().await;
        *start = Some(Instant::now());
    }

    /// Process a streaming chunk
    pub async fn process_chunk(&self, content: &str) {
        let chunk_chars = content.len();

        // Update char count
        self.char_count.fetch_add(chunk_chars, Ordering::SeqCst);

        // Update estimated tokens
        let total_chars = self.char_count.load(Ordering::SeqCst);
        let estimated = (total_chars as f32 / self.chars_per_token).ceil() as usize;
        self.estimated_tokens.store(estimated, Ordering::SeqCst);

        // Track chunk count
        let was_first = self.chunk_count.fetch_add(1, Ordering::SeqCst) == 0;

        // Record first token time
        if was_first {
            let mut first_time = self.first_token_time.write().await;
            if first_time.is_none() {
                *first_time = Some(Instant::now());
            }
        }
    }

    /// Get current estimated token count
    pub fn estimated_tokens(&self) -> usize {
        self.estimated_tokens.load(Ordering::SeqCst)
    }

    /// Get character count
    pub fn char_count(&self) -> usize {
        self.char_count.load(Ordering::SeqCst)
    }

    /// Get chunk count
    pub fn chunk_count(&self) -> usize {
        self.chunk_count.load(Ordering::SeqCst)
    }

    /// Get time to first token (TTFT)
    pub async fn time_to_first_token(&self) -> Option<Duration> {
        let start = self.start_time.read().await;
        let first = self.first_token_time.read().await;

        match (*start, *first) {
            (Some(s), Some(f)) => Some(f.duration_since(s)),
            _ => None,
        }
    }

    /// Get elapsed time since start
    pub async fn elapsed(&self) -> Option<Duration> {
        let start = self.start_time.read().await;
        start.map(|s| s.elapsed())
    }

    /// Get tokens per second rate
    pub async fn tokens_per_second(&self) -> Option<f64> {
        let elapsed = self.elapsed().await?;
        let tokens = self.estimated_tokens();
        let secs = elapsed.as_secs_f64();

        if secs > 0.0 {
            Some(tokens as f64 / secs)
        } else {
            None
        }
    }

    /// Get streaming statistics
    pub async fn stats(&self) -> StreamingStats {
        StreamingStats {
            char_count: self.char_count(),
            estimated_tokens: self.estimated_tokens(),
            chunk_count: self.chunk_count(),
            elapsed: self.elapsed().await,
            time_to_first_token: self.time_to_first_token().await,
            tokens_per_second: self.tokens_per_second().await,
        }
    }

    /// Reset the counter
    pub async fn reset(&self) {
        self.char_count.store(0, Ordering::SeqCst);
        self.estimated_tokens.store(0, Ordering::SeqCst);
        self.chunk_count.store(0, Ordering::SeqCst);
        *self.start_time.write().await = None;
        *self.first_token_time.write().await = None;
    }
}

impl Default for StreamingTokenCounter {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics from streaming
#[derive(Debug, Clone)]
pub struct StreamingStats {
    /// Total characters received
    pub char_count: usize,
    /// Estimated token count
    pub estimated_tokens: usize,
    /// Number of chunks received
    pub chunk_count: usize,
    /// Total elapsed time
    pub elapsed: Option<Duration>,
    /// Time to first token
    pub time_to_first_token: Option<Duration>,
    /// Tokens per second rate
    pub tokens_per_second: Option<f64>,
}

impl StreamingStats {
    /// Format stats as a summary string
    pub fn summary(&self) -> String {
        let mut parts = vec![];

        parts.push(format!("{} chars", self.char_count));
        parts.push(format!("~{} tokens", self.estimated_tokens));
        parts.push(format!("{} chunks", self.chunk_count));

        if let Some(ttft) = self.time_to_first_token {
            parts.push(format!("TTFT: {:?}", ttft));
        }

        if let Some(tps) = self.tokens_per_second {
            parts.push(format!("{:.1} tok/s", tps));
        }

        parts.join(", ")
    }
}

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
            let ttft_ms = ttft.as_millis() as usize;
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

/// Aggregated streaming statistics
#[derive(Debug, Clone)]
pub struct AggregatedStats {
    /// Total number of streaming sessions
    pub session_count: usize,
    /// Total tokens across all sessions
    pub total_tokens: usize,
    /// Average tokens per session
    pub avg_tokens_per_session: f64,
    /// Average time to first token
    pub avg_ttft: Option<Duration>,
    /// Minimum time to first token
    pub min_ttft: Option<Duration>,
    /// Maximum time to first token
    pub max_ttft: Option<Duration>,
}

impl AggregatedStats {
    /// Format as summary string
    pub fn summary(&self) -> String {
        let mut parts = vec![
            format!("{} sessions", self.session_count),
            format!("{} total tokens", self.total_tokens),
            format!("{:.1} avg tokens", self.avg_tokens_per_session),
        ];

        if let Some(avg) = self.avg_ttft {
            parts.push(format!("avg TTFT: {:?}", avg));
        }

        if let (Some(min), Some(max)) = (self.min_ttft, self.max_ttft) {
            parts.push(format!("TTFT range: {:?}-{:?}", min, max));
        }

        parts.join(", ")
    }
}

/// Thread-safe wrapper for shared streaming metrics
pub type SharedStreamingMetrics = Arc<StreamingMetrics>;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_streaming_counter_basic() {
        let counter = StreamingTokenCounter::new();
        counter.start().await;

        counter.process_chunk("Hello ").await;
        counter.process_chunk("World!").await;

        assert_eq!(counter.char_count(), 12);
        assert!(counter.estimated_tokens() > 0);
        assert_eq!(counter.chunk_count(), 2);
    }

    #[tokio::test]
    async fn test_streaming_counter_tokens_estimate() {
        let counter = StreamingTokenCounter::new().with_chars_per_token(4.0);
        counter.start().await;

        // 40 chars / 4 = 10 tokens
        counter.process_chunk(&"a".repeat(40)).await;

        assert_eq!(counter.char_count(), 40);
        assert_eq!(counter.estimated_tokens(), 10);
    }

    #[tokio::test]
    async fn test_streaming_counter_provider() {
        let anthropic = StreamingTokenCounter::for_provider("anthropic");
        let openai = StreamingTokenCounter::for_provider("openai");

        anthropic.start().await;
        openai.start().await;

        let text = &"a".repeat(35);
        anthropic.process_chunk(text).await;
        openai.process_chunk(text).await;

        // Anthropic uses 3.5 chars/token, OpenAI uses 4.0
        // 35 / 3.5 = 10, 35 / 4.0 = 8.75 -> 9
        assert_eq!(anthropic.estimated_tokens(), 10);
        assert_eq!(openai.estimated_tokens(), 9);
    }

    #[tokio::test]
    async fn test_streaming_counter_time_to_first_token() {
        let counter = StreamingTokenCounter::new();
        counter.start().await;

        // Small delay before first chunk
        tokio::time::sleep(Duration::from_millis(10)).await;
        counter.process_chunk("First").await;

        let ttft = counter.time_to_first_token().await;
        assert!(ttft.is_some());
        assert!(ttft.unwrap() >= Duration::from_millis(10));
    }

    #[tokio::test]
    async fn test_streaming_counter_stats() {
        let counter = StreamingTokenCounter::new();
        counter.start().await;

        counter.process_chunk("Hello").await;
        counter.process_chunk(" World").await;

        let stats = counter.stats().await;

        assert_eq!(stats.char_count, 11);
        assert_eq!(stats.chunk_count, 2);
        assert!(stats.elapsed.is_some());
        assert!(stats.time_to_first_token.is_some());
    }

    #[tokio::test]
    async fn test_streaming_counter_reset() {
        let counter = StreamingTokenCounter::new();
        counter.start().await;
        counter.process_chunk("Test").await;

        assert!(counter.char_count() > 0);

        counter.reset().await;

        assert_eq!(counter.char_count(), 0);
        assert_eq!(counter.estimated_tokens(), 0);
        assert_eq!(counter.chunk_count(), 0);
    }

    #[tokio::test]
    async fn test_streaming_metrics_record() {
        let metrics = StreamingMetrics::new();
        let counter = StreamingTokenCounter::new();

        counter.start().await;
        counter.process_chunk("Test content").await;
        metrics.record(&counter).await;

        assert_eq!(metrics.session_count(), 1);
        assert!(metrics.total_tokens() > 0);
    }

    #[tokio::test]
    async fn test_streaming_metrics_multiple_sessions() {
        let metrics = StreamingMetrics::new();

        for i in 0..3 {
            let counter = StreamingTokenCounter::new();
            counter.start().await;
            counter.process_chunk(&format!("Session {} content", i)).await;
            metrics.record(&counter).await;
        }

        assert_eq!(metrics.session_count(), 3);
        assert!(metrics.avg_tokens_per_session() > 0.0);
    }

    #[tokio::test]
    async fn test_streaming_metrics_ttft_aggregation() {
        let metrics = StreamingMetrics::new();

        // Simulate sessions with different TTFT
        for delay_ms in [10u64, 20, 30] {
            let counter = StreamingTokenCounter::new();
            counter.start().await;
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            counter.process_chunk("Content").await;
            metrics.record(&counter).await;
        }

        let summary = metrics.summary();
        assert!(summary.avg_ttft.is_some());
        assert!(summary.min_ttft.is_some());
        assert!(summary.max_ttft.is_some());

        // Min should be less than max
        assert!(summary.min_ttft.unwrap() <= summary.max_ttft.unwrap());
    }

    #[tokio::test]
    async fn test_streaming_metrics_reset() {
        let metrics = StreamingMetrics::new();
        let counter = StreamingTokenCounter::new();

        counter.start().await;
        counter.process_chunk("Test").await;
        metrics.record(&counter).await;

        assert!(metrics.session_count() > 0);

        metrics.reset();

        assert_eq!(metrics.session_count(), 0);
        assert_eq!(metrics.total_tokens(), 0);
    }

    #[test]
    fn test_streaming_stats_summary() {
        let stats = StreamingStats {
            char_count: 100,
            estimated_tokens: 25,
            chunk_count: 5,
            elapsed: Some(Duration::from_secs(1)),
            time_to_first_token: Some(Duration::from_millis(50)),
            tokens_per_second: Some(25.0),
        };

        let summary = stats.summary();
        assert!(summary.contains("100 chars"));
        assert!(summary.contains("~25 tokens"));
        assert!(summary.contains("5 chunks"));
        assert!(summary.contains("TTFT"));
    }

    #[test]
    fn test_aggregated_stats_summary() {
        let stats = AggregatedStats {
            session_count: 10,
            total_tokens: 500,
            avg_tokens_per_session: 50.0,
            avg_ttft: Some(Duration::from_millis(100)),
            min_ttft: Some(Duration::from_millis(50)),
            max_ttft: Some(Duration::from_millis(200)),
        };

        let summary = stats.summary();
        assert!(summary.contains("10 sessions"));
        assert!(summary.contains("500 total tokens"));
    }

    #[tokio::test]
    async fn test_tokens_per_second() {
        let counter = StreamingTokenCounter::new().with_chars_per_token(4.0);
        counter.start().await;

        // Process 100 chars = 25 tokens
        counter.process_chunk(&"a".repeat(100)).await;

        // Wait a bit to get measurable rate
        tokio::time::sleep(Duration::from_millis(50)).await;

        let tps = counter.tokens_per_second().await;
        assert!(tps.is_some());
        assert!(tps.unwrap() > 0.0);
    }
}
