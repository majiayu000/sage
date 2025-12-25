//! Streaming token counter
//!
//! This module provides real-time token counting during streaming LLM responses.

mod counter;
mod metrics;
mod types;

#[cfg(test)]
mod tests;

pub use counter::StreamingTokenCounter;
pub use metrics::{SharedStreamingMetrics, StreamingMetrics};
pub use types::{AggregatedStats, StreamingStats};
