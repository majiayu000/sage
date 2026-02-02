//! Prompt cache
//!
//! Caches parsed prompts with TTL-based expiration.

use super::file_loader::PromptFile;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Cache entry with expiration
struct CacheEntry {
    prompt: PromptFile,
    inserted_at: Instant,
}

/// TTL-based prompt cache
pub struct PromptCache {
    /// Cached entries
    entries: Arc<RwLock<HashMap<String, CacheEntry>>>,
    /// Time-to-live for cache entries
    ttl: Duration,
}

impl PromptCache {
    /// Create a new cache with default TTL (5 minutes)
    pub fn new() -> Self {
        Self::with_ttl(Duration::from_secs(300))
    }

    /// Create a new cache with custom TTL
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            ttl,
        }
    }

    /// Get a cached prompt
    pub fn get(&self, key: &str) -> Option<PromptFile> {
        let entries = self.entries.read();
        if let Some(entry) = entries.get(key) {
            if entry.inserted_at.elapsed() < self.ttl {
                return Some(entry.prompt.clone());
            }
        }
        None
    }

    /// Insert a prompt into the cache
    pub fn insert(&self, key: impl Into<String>, prompt: PromptFile) {
        let mut entries = self.entries.write();
        entries.insert(
            key.into(),
            CacheEntry {
                prompt,
                inserted_at: Instant::now(),
            },
        );
    }

    /// Remove a prompt from the cache
    pub fn remove(&self, key: &str) -> Option<PromptFile> {
        let mut entries = self.entries.write();
        entries.remove(key).map(|e| e.prompt)
    }

    /// Invalidate a specific entry
    pub fn invalidate(&self, key: &str) {
        self.remove(key);
    }

    /// Invalidate all entries matching a prefix
    pub fn invalidate_prefix(&self, prefix: &str) {
        let mut entries = self.entries.write();
        entries.retain(|k, _| !k.starts_with(prefix));
    }

    /// Clear all expired entries
    pub fn cleanup_expired(&self) {
        let mut entries = self.entries.write();
        entries.retain(|_, entry| entry.inserted_at.elapsed() < self.ttl);
    }

    /// Clear all entries
    pub fn clear(&self) {
        let mut entries = self.entries.write();
        entries.clear();
    }

    /// Get the number of cached entries
    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.entries.read().is_empty()
    }

    /// Check if a key exists and is not expired
    pub fn contains(&self, key: &str) -> bool {
        let entries = self.entries.read();
        if let Some(entry) = entries.get(key) {
            entry.inserted_at.elapsed() < self.ttl
        } else {
            false
        }
    }

    /// Get or insert a prompt using a factory function
    pub fn get_or_insert<F>(&self, key: &str, factory: F) -> Option<PromptFile>
    where
        F: FnOnce() -> Option<PromptFile>,
    {
        // Try to get from cache first
        if let Some(prompt) = self.get(key) {
            return Some(prompt);
        }

        // Generate and cache
        if let Some(prompt) = factory() {
            self.insert(key, prompt.clone());
            Some(prompt)
        } else {
            None
        }
    }
}

impl Default for PromptCache {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for PromptCache {
    fn clone(&self) -> Self {
        Self {
            entries: Arc::clone(&self.entries),
            ttl: self.ttl,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompts::loader::file_loader::PromptMetadata;

    fn make_test_prompt(name: &str) -> PromptFile {
        PromptFile {
            metadata: PromptMetadata {
                name: name.to_string(),
                ..Default::default()
            },
            content: format!("Content for {}", name),
            source_path: None,
        }
    }

    #[test]
    fn test_cache_insert_and_get() {
        let cache = PromptCache::new();
        let prompt = make_test_prompt("test");

        cache.insert("test-key", prompt.clone());

        let retrieved = cache.get("test-key");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().metadata.name, "test");
    }

    #[test]
    fn test_cache_miss() {
        let cache = PromptCache::new();
        assert!(cache.get("nonexistent").is_none());
    }

    #[test]
    fn test_cache_remove() {
        let cache = PromptCache::new();
        let prompt = make_test_prompt("test");

        cache.insert("test-key", prompt);
        assert!(cache.contains("test-key"));

        cache.remove("test-key");
        assert!(!cache.contains("test-key"));
    }

    #[test]
    fn test_cache_clear() {
        let cache = PromptCache::new();
        cache.insert("key1", make_test_prompt("test1"));
        cache.insert("key2", make_test_prompt("test2"));

        assert_eq!(cache.len(), 2);

        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_invalidate_prefix() {
        let cache = PromptCache::new();
        cache.insert("system-prompt/identity", make_test_prompt("identity"));
        cache.insert("system-prompt/tone", make_test_prompt("tone"));
        cache.insert("agent-prompt/explore", make_test_prompt("explore"));

        assert_eq!(cache.len(), 3);

        cache.invalidate_prefix("system-prompt/");
        assert_eq!(cache.len(), 1);
        assert!(cache.contains("agent-prompt/explore"));
    }

    #[test]
    fn test_cache_expiration() {
        let cache = PromptCache::with_ttl(Duration::from_millis(10));
        cache.insert("test-key", make_test_prompt("test"));

        assert!(cache.contains("test-key"));

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(20));

        assert!(!cache.contains("test-key"));
        assert!(cache.get("test-key").is_none());
    }

    #[test]
    fn test_get_or_insert() {
        let cache = PromptCache::new();

        // First call should insert
        let result = cache.get_or_insert("test-key", || Some(make_test_prompt("test")));
        assert!(result.is_some());
        assert_eq!(cache.len(), 1);

        // Second call should get from cache
        let result = cache.get_or_insert("test-key", || panic!("Should not be called"));
        assert!(result.is_some());
    }
}
