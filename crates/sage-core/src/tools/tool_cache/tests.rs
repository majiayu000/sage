//! Tests for tool cache

use super::*;
use serde_json::json;
use std::time::Duration;

#[test]
fn test_cache_key_creation() {
    let args = json!({"path": "/test/file.txt"});
    let key = ToolCacheKey::new("Read", &args);

    assert_eq!(key.tool_name, "Read");
    assert!(!key.args_hash.is_empty());
}

#[test]
fn test_cache_key_consistency() {
    let args1 = json!({"a": 1, "b": 2});
    let args2 = json!({"b": 2, "a": 1}); // Same content, different order

    let key1 = ToolCacheKey::new("Test", &args1);
    let key2 = ToolCacheKey::new("Test", &args2);

    // Keys should be the same (canonicalized)
    assert_eq!(key1.args_hash, key2.args_hash);
}

#[test]
fn test_cached_result_validity() {
    let result = CachedResult::new("test".to_string(), true, Duration::from_millis(100));
    assert!(result.is_valid());

    std::thread::sleep(Duration::from_millis(150));
    assert!(!result.is_valid());
}

#[test]
fn test_cached_result_time_remaining() {
    let result = CachedResult::new("test".to_string(), true, Duration::from_secs(10));
    let remaining = result.time_remaining();
    assert!(remaining.is_some());
    assert!(remaining.unwrap() > Duration::from_secs(9));
}

#[tokio::test]
async fn test_cache_set_get() {
    let cache = ToolCache::with_defaults();
    let key = ToolCacheKey::new("Read", &json!({"path": "/test"}));

    cache.set(key.clone(), "content".to_string(), true).await;

    let result = cache.get(&key).await;
    assert!(result.is_some());
    assert_eq!(result.unwrap().result, "content");
}

#[tokio::test]
async fn test_cache_miss() {
    let cache = ToolCache::with_defaults();
    let key = ToolCacheKey::new("Read", &json!({"path": "/nonexistent"}));

    let result = cache.get(&key).await;
    assert!(result.is_none());
}

#[tokio::test]
async fn test_cache_expiration() {
    let config = ToolCacheConfig::default().with_default_ttl(Duration::from_millis(50));
    let cache = ToolCache::new(config);
    let key = ToolCacheKey::new("Test", &json!({}));

    cache.set(key.clone(), "test".to_string(), true).await;
    assert!(cache.get(&key).await.is_some());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(cache.get(&key).await.is_none());
}

#[tokio::test]
async fn test_cache_excluded_tool() {
    let cache = ToolCache::with_defaults();
    let key = ToolCacheKey::new("Bash", &json!({"command": "ls"}));

    cache.set(key.clone(), "output".to_string(), true).await;

    // Bash is excluded, so get should return None
    let result = cache.get(&key).await;
    assert!(result.is_none());
}

#[tokio::test]
async fn test_cache_invalidate_tool() {
    let cache = ToolCache::with_defaults();

    cache
        .set(
            ToolCacheKey::new("Read", &json!({"path": "a"})),
            "a".to_string(),
            true,
        )
        .await;
    cache
        .set(
            ToolCacheKey::new("Read", &json!({"path": "b"})),
            "b".to_string(),
            true,
        )
        .await;
    cache
        .set(
            ToolCacheKey::new("Glob", &json!({"pattern": "*"})),
            "glob".to_string(),
            true,
        )
        .await;

    assert_eq!(cache.len().await, 3);

    cache.invalidate_tool("Read").await;

    assert_eq!(cache.len().await, 1);
}

#[tokio::test]
async fn test_cache_clear() {
    let cache = ToolCache::with_defaults();
    let key = ToolCacheKey::new("Read", &json!({"path": "/test"}));

    cache.set(key.clone(), "content".to_string(), true).await;
    assert!(!cache.is_empty().await);

    cache.clear().await;
    assert!(cache.is_empty().await);
}

#[tokio::test]
async fn test_cache_stats() {
    let cache = ToolCache::with_defaults();
    let key = ToolCacheKey::new("Read", &json!({"path": "/test"}));

    // Miss
    cache.get(&key).await;

    // Insert
    cache.set(key.clone(), "content".to_string(), true).await;

    // Hit
    cache.get(&key).await;
    cache.get(&key).await;

    let stats = cache.stats().await;
    assert_eq!(stats.misses, 1);
    assert_eq!(stats.hits, 2);
    assert_eq!(stats.inserts, 1);
    assert!((stats.hit_rate() - 0.666).abs() < 0.01);
}

#[tokio::test]
async fn test_cache_cleanup_expired() {
    let config = ToolCacheConfig::default().with_default_ttl(Duration::from_millis(50));
    let cache = ToolCache::new(config);

    cache
        .set(
            ToolCacheKey::new("Test", &json!({"a": 1})),
            "1".to_string(),
            true,
        )
        .await;
    cache
        .set(
            ToolCacheKey::new("Test", &json!({"a": 2})),
            "2".to_string(),
            true,
        )
        .await;

    assert_eq!(cache.len().await, 2);

    tokio::time::sleep(Duration::from_millis(100)).await;

    let removed = cache.cleanup_expired().await;
    assert_eq!(removed, 2);
    assert!(cache.is_empty().await);
}

#[tokio::test]
async fn test_cache_max_entries() {
    let config = ToolCacheConfig {
        max_entries: 2,
        ..Default::default()
    };
    let cache = ToolCache::new(config);

    cache
        .set(
            ToolCacheKey::new("Read", &json!({"a": 1})),
            "1".to_string(),
            true,
        )
        .await;
    cache
        .set(
            ToolCacheKey::new("Read", &json!({"a": 2})),
            "2".to_string(),
            true,
        )
        .await;
    cache
        .set(
            ToolCacheKey::new("Read", &json!({"a": 3})),
            "3".to_string(),
            true,
        )
        .await;

    // Should only keep max_entries
    assert_eq!(cache.len().await, 2);
}

#[tokio::test]
async fn test_cache_max_result_size() {
    let config = ToolCacheConfig {
        max_result_size: 10,
        ..Default::default()
    };
    let cache = ToolCache::new(config);

    // Small result should be cached
    cache
        .set(
            ToolCacheKey::new("Read", &json!({"a": 1})),
            "small".to_string(),
            true,
        )
        .await;
    assert_eq!(cache.len().await, 1);

    // Large result should not be cached
    cache
        .set(
            ToolCacheKey::new("Read", &json!({"a": 2})),
            "this is a very large result".to_string(),
            true,
        )
        .await;
    assert_eq!(cache.len().await, 1);
}

#[test]
fn test_config_tool_ttl() {
    let config = ToolCacheConfig::default();

    // Read has custom TTL
    assert_eq!(config.ttl_for_tool("Read"), Duration::from_secs(30));

    // Unknown tool uses default
    assert_eq!(config.ttl_for_tool("Unknown"), config.default_ttl);
}

#[test]
fn test_config_should_cache() {
    let config = ToolCacheConfig::default();

    assert!(config.should_cache("Read"));
    assert!(config.should_cache("Glob"));
    assert!(!config.should_cache("Bash"));
    assert!(!config.should_cache("Write"));
}

#[test]
fn test_config_builder() {
    let config = ToolCacheConfig::default()
        .with_default_ttl(Duration::from_secs(60))
        .with_tool_ttl("Custom", Duration::from_secs(30))
        .exclude_tool("ExcludedTool");

    assert_eq!(config.default_ttl, Duration::from_secs(60));
    assert_eq!(config.ttl_for_tool("Custom"), Duration::from_secs(30));
    assert!(!config.should_cache("ExcludedTool"));
}

#[test]
fn test_cache_stats_summary() {
    let stats = ToolCacheStats {
        hits: 80,
        misses: 20,
        inserts: 100,
        expirations: 10,
        clears: 1,
    };

    let summary = stats.summary();
    assert!(summary.contains("80"));
    assert!(summary.contains("80.0%"));
}

#[tokio::test]
async fn test_hit_count_tracking() {
    let cache = ToolCache::with_defaults();
    let key = ToolCacheKey::new("Read", &json!({"path": "/test"}));

    cache.set(key.clone(), "content".to_string(), true).await;

    cache.get(&key).await;
    cache.get(&key).await;
    cache.get(&key).await;

    let result = cache.get(&key).await.unwrap();
    assert_eq!(result.hit_count, 4); // Including this get
}
