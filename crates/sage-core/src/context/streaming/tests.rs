//! Tests for streaming token counter and metrics

use std::time::Duration;

use super::counter::StreamingTokenCounter;
use super::metrics::StreamingMetrics;
use super::types::{AggregatedStats, StreamingStats};

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
        counter
            .process_chunk(&format!("Session {} content", i))
            .await;
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
