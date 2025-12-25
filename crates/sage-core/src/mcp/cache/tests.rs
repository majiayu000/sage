//! Cache tests

#![cfg(test)]

use super::{CacheConfig, CacheEntry, McpCache};
use crate::mcp::types::McpTool;
use std::time::Duration;

#[test]
fn test_cache_entry_expiration() {
    let entry = CacheEntry::new("test", Some(Duration::from_millis(1)));
    assert!(!entry.is_expired());

    std::thread::sleep(Duration::from_millis(2));
    assert!(entry.is_expired());
}

#[test]
fn test_cache_entry_no_expiration() {
    let entry: CacheEntry<&str> = CacheEntry::new("test", None);
    assert!(!entry.is_expired());
}

#[test]
fn test_cache_tools() {
    let cache = McpCache::new();
    let tools = vec![McpTool::new("test_tool")];

    cache.cache_tools("server1", tools.clone());
    let cached = cache.get_tools("server1");

    assert!(cached.is_some());
    assert_eq!(cached.unwrap().len(), 1);
}

#[test]
fn test_cache_miss() {
    let cache = McpCache::new();
    let result = cache.get_tools("nonexistent");
    assert!(result.is_none());
    assert_eq!(cache.stats().misses(), 1);
}

#[test]
fn test_cache_hit_tracking() {
    let cache = McpCache::new();
    let tools = vec![McpTool::new("test_tool")];

    cache.cache_tools("server1", tools);

    let _ = cache.get_tools("server1");
    let _ = cache.get_tools("server1");

    assert_eq!(cache.stats().hits(), 2);
}

#[test]
fn test_cache_invalidation() {
    let cache = McpCache::new();
    let tools = vec![McpTool::new("test_tool")];

    cache.cache_tools("server1", tools);
    assert!(cache.get_tools("server1").is_some());

    cache.invalidate_tools("server1");
    let result = cache.get_tools("server1");
    assert!(result.is_none());
}

#[test]
fn test_cache_clear() {
    let cache = McpCache::new();
    let tools = vec![McpTool::new("test_tool")];

    cache.cache_tools("server1", tools);
    cache.cache_prompts("server1", vec![]);

    cache.clear();

    assert_eq!(cache.total_entries(), 0);
}

#[test]
fn test_cache_expiration() {
    let config = CacheConfig::with_ttl(Duration::from_millis(1));
    let cache = McpCache::with_config(config);
    let tools = vec![McpTool::new("test_tool")];

    cache.cache_tools("server1", tools);

    std::thread::sleep(Duration::from_millis(2));

    let result = cache.get_tools("server1");
    assert!(result.is_none());
}

#[test]
fn test_cache_stats() {
    let cache = McpCache::new();
    let tools = vec![McpTool::new("test_tool")];

    cache.cache_tools("server1", tools);

    let _ = cache.get_tools("server1"); // hit
    let _ = cache.get_tools("nonexistent"); // miss

    assert_eq!(cache.stats().hits(), 1);
    assert_eq!(cache.stats().misses(), 1);
    assert_eq!(cache.stats().hit_rate(), 50.0);
}

#[test]
fn test_size_breakdown() {
    let cache = McpCache::new();

    cache.cache_tools("server1", vec![]);
    cache.cache_resources("server1", vec![]);
    cache.cache_prompts("server1", vec![]);

    let breakdown = cache.size_breakdown();
    assert_eq!(breakdown.tools, 1);
    assert_eq!(breakdown.resources, 1);
    assert_eq!(breakdown.prompts, 1);
    assert_eq!(breakdown.total(), 3);
}
