//! Tests for metrics collector

#[cfg(test)]
mod tests {
    use super::super::collector::MetricsCollector;
    use super::super::types::create_metrics_collector;

    #[tokio::test]
    async fn test_collector_creation() {
        let collector = MetricsCollector::new();
        assert_eq!(collector.llm_requests.get(), 0);
        assert_eq!(collector.tool_calls.get(), 0);
    }

    #[tokio::test]
    async fn test_record_llm_request() {
        let collector = MetricsCollector::new();

        collector.record_llm_request(100, 50, 0.5, true).await;

        assert_eq!(collector.llm_requests.get(), 1);
        assert_eq!(collector.llm_tokens_input.get(), 100);
        assert_eq!(collector.llm_tokens_output.get(), 50);
        assert_eq!(collector.llm_errors.get(), 0);
    }

    #[tokio::test]
    async fn test_record_llm_error() {
        let collector = MetricsCollector::new();

        collector.record_llm_request(100, 0, 1.0, false).await;

        assert_eq!(collector.llm_requests.get(), 1);
        assert_eq!(collector.llm_errors.get(), 1);
    }

    #[tokio::test]
    async fn test_record_tool_call() {
        let collector = MetricsCollector::new();

        collector.record_tool_call(0.1, true).await;
        collector.record_tool_call(0.2, false).await;

        assert_eq!(collector.tool_calls.get(), 2);
        assert_eq!(collector.tool_success.get(), 1);
        assert_eq!(collector.tool_errors.get(), 1);
    }

    #[tokio::test]
    async fn test_session_tracking() {
        let collector = MetricsCollector::new();

        collector.record_session_start();
        collector.record_session_start();
        assert_eq!(collector.active_sessions.get() as u64, 2);
        assert_eq!(collector.total_sessions.get(), 2);

        collector.record_session_end(300.0).await;
        assert_eq!(collector.active_sessions.get() as u64, 1);
    }

    #[tokio::test]
    async fn test_cache_metrics() {
        let collector = MetricsCollector::new();

        collector.record_cache_hit();
        collector.record_cache_hit();
        collector.record_cache_miss();

        assert_eq!(collector.cache_hits.get(), 2);
        assert_eq!(collector.cache_misses.get(), 1);
        assert!((collector.cache_hit_rate() - 0.666).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_custom_counter() {
        let collector = MetricsCollector::new();

        collector
            .register_counter("custom_events", "Custom events")
            .await;
        collector.inc_counter("custom_events").await;
        collector.inc_counter_by("custom_events", 5).await;

        let counters = collector.custom_counters.read().await;
        assert_eq!(counters.get("custom_events").unwrap().get(), 6);
    }

    #[tokio::test]
    async fn test_custom_gauge() {
        let collector = MetricsCollector::new();

        collector.register_gauge("queue_size", "Queue size").await;
        collector.set_gauge("queue_size", 42.0).await;

        let gauges = collector.custom_gauges.read().await;
        assert!((gauges.get("queue_size").unwrap().get() - 42.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_custom_histogram() {
        let collector = MetricsCollector::new();

        collector
            .register_histogram("custom_duration", "Custom duration")
            .await;
        collector.observe_histogram("custom_duration", 0.5).await;

        let histograms = collector.custom_histograms.read().await;
        let data = histograms.get("custom_duration").unwrap().get_data();
        assert_eq!(data.count, 1);
    }

    #[tokio::test]
    async fn test_snapshot() {
        let collector = MetricsCollector::new();

        collector.record_llm_request(100, 50, 0.5, true).await;
        collector.record_tool_call(0.1, true).await;

        let snapshot = collector.snapshot().await;

        assert_eq!(snapshot.llm_requests, 1);
        assert_eq!(snapshot.total_tokens(), 150);
        assert_eq!(snapshot.tool_calls, 1);
    }

    #[tokio::test]
    async fn test_success_rates() {
        let collector = MetricsCollector::new();

        // Tool success rate
        collector.record_tool_call(0.1, true).await;
        collector.record_tool_call(0.1, true).await;
        collector.record_tool_call(0.1, false).await;

        assert!((collector.tool_success_rate() - 0.666).abs() < 0.01);

        // LLM error rate
        collector.record_llm_request(100, 50, 0.5, true).await;
        collector.record_llm_request(100, 0, 1.0, false).await;

        assert!((collector.llm_error_rate() - 0.5).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_reset() {
        let collector = MetricsCollector::new();

        collector.record_llm_request(100, 50, 0.5, true).await;
        collector.record_tool_call(0.1, true).await;

        collector.reset().await;

        assert_eq!(collector.llm_requests.get(), 0);
        assert_eq!(collector.tool_calls.get(), 0);
    }

    #[tokio::test]
    async fn test_summary() {
        let collector = MetricsCollector::new();

        collector.record_llm_request(1000, 500, 0.5, true).await;
        collector.record_tool_call(0.1, true).await;

        let summary = collector.summary().await;

        assert!(summary.contains("LLM:"));
        assert!(summary.contains("Tools:"));
        assert!(summary.contains("1500 tokens"));
    }

    #[tokio::test]
    async fn test_total_tokens() {
        let collector = MetricsCollector::new();

        collector.record_llm_request(100, 50, 0.5, true).await;
        collector.record_llm_request(200, 100, 0.3, true).await;

        assert_eq!(collector.total_tokens(), 450);
    }

    #[test]
    fn test_shared_collector() {
        let collector = create_metrics_collector();
        collector.llm_requests.inc();
        assert_eq!(collector.llm_requests.get(), 1);
    }
}
