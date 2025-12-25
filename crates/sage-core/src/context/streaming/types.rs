//! Types for streaming statistics and results

use std::time::Duration;

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
