//! Cache storage implementations

use super::types::{CacheEntry, CacheKey, StorageStatistics};
use crate::error::{SageError, SageResult};
use async_trait::async_trait;
use chrono::Utc;
use lru::LruCache;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::Mutex;

/// Cache storage interface
#[async_trait]
pub trait CacheStorage: Send + Sync {
    /// Get a cache entry
    async fn get(&self, key: &CacheKey) -> SageResult<Option<CacheEntry>>;

    /// Set a cache entry
    async fn set(&self, key: CacheKey, entry: CacheEntry) -> SageResult<()>;

    /// Remove a cache entry
    async fn remove(&self, key: &CacheKey) -> SageResult<()>;

    /// Clear all entries
    async fn clear(&self) -> SageResult<()>;

    /// Get storage statistics
    async fn statistics(&self) -> SageResult<StorageStatistics>;

    /// Cleanup expired entries
    async fn cleanup_expired(&self) -> SageResult<()>;
}

/// In-memory cache storage using LRU cache
#[derive(Debug)]
pub struct MemoryStorage {
    /// LRU cache for entries
    cache: Arc<Mutex<LruCache<u64, CacheEntry>>>,
    /// Statistics
    stats: Arc<Mutex<StorageStatistics>>,
}

impl MemoryStorage {
    /// Create a new memory storage with specified capacity
    pub fn new(capacity: usize) -> Self {
        use std::num::NonZeroUsize;
        let capacity = NonZeroUsize::new(capacity.max(1)).unwrap();
        Self {
            cache: Arc::new(Mutex::new(LruCache::new(capacity))),
            stats: Arc::new(Mutex::new(StorageStatistics::default())),
        }
    }
}

#[async_trait]
impl CacheStorage for MemoryStorage {
    async fn get(&self, key: &CacheKey) -> SageResult<Option<CacheEntry>> {
        let mut cache = self.cache.lock().await;
        let mut stats = self.stats.lock().await;

        if let Some(entry) = cache.get(&key.hash).cloned() {
            if entry.is_expired() {
                // Remove expired entry
                cache.pop(&key.hash);
                stats.evictions += 1;
                stats.entry_count = cache.len();
                stats.misses += 1;
                Ok(None)
            } else {
                // Update access stats
                let mut entry = entry.clone();
                entry.mark_accessed();
                if let Some(old_entry) = cache.put(key.hash, entry.clone()) {
                    stats.size_bytes = stats.size_bytes.saturating_sub(old_entry.size_bytes as u64);
                }
                stats.size_bytes += entry.size_bytes as u64;
                stats.hits += 1;
                Ok(Some(entry))
            }
        } else {
            stats.misses += 1;
            Ok(None)
        }
    }

    async fn set(&self, key: CacheKey, entry: CacheEntry) -> SageResult<()> {
        let mut cache = self.cache.lock().await;
        let mut stats = self.stats.lock().await;

        // Check if this will cause an eviction
        let will_evict = cache.len() >= cache.cap().get() && !cache.contains(&key.hash);

        // Update stats
        if let Some(old_entry) = cache.put(key.hash, entry.clone()) {
            // Replacing existing entry
            stats.size_bytes = stats.size_bytes.saturating_sub(old_entry.size_bytes as u64);
        } else if will_evict {
            // New entry that caused eviction
            stats.evictions += 1;
            // Note: entry_count stays the same due to eviction
        } else {
            // New entry, no eviction
            stats.entry_count += 1;
        }

        stats.size_bytes += entry.size_bytes as u64;
        stats.entry_count = cache.len(); // Ensure consistency

        Ok(())
    }

    async fn remove(&self, key: &CacheKey) -> SageResult<()> {
        let mut cache = self.cache.lock().await;
        let mut stats = self.stats.lock().await;

        if let Some(entry) = cache.pop(&key.hash) {
            stats.entry_count -= 1;
            stats.size_bytes = stats.size_bytes.saturating_sub(entry.size_bytes as u64);
            stats.evictions += 1;
        }

        Ok(())
    }

    async fn clear(&self) -> SageResult<()> {
        let mut cache = self.cache.lock().await;
        let mut stats = self.stats.lock().await;

        cache.clear();
        *stats = StorageStatistics::default();

        Ok(())
    }

    async fn statistics(&self) -> SageResult<StorageStatistics> {
        let stats = self.stats.lock().await;
        Ok(stats.clone())
    }

    async fn cleanup_expired(&self) -> SageResult<()> {
        let mut cache = self.cache.lock().await;
        let mut stats = self.stats.lock().await;

        let now = Utc::now();
        let expired_keys: Vec<u64> = cache
            .iter()
            .filter_map(|(k, v)| {
                if v.expires_at.map_or(false, |exp| exp < now) {
                    Some(*k)
                } else {
                    None
                }
            })
            .collect();

        for key in expired_keys {
            if let Some(entry) = cache.pop(&key) {
                stats.entry_count -= 1;
                stats.size_bytes = stats.size_bytes.saturating_sub(entry.size_bytes as u64);
                stats.evictions += 1;
            }
        }

        Ok(())
    }
}

/// Disk-based cache storage
#[derive(Debug)]
pub struct DiskStorage {
    /// Base directory for cache files
    base_dir: PathBuf,
    /// Maximum capacity in bytes
    capacity: u64,
    /// Current size in bytes
    current_size: Arc<Mutex<u64>>,
    /// Index of cache entries
    index: Arc<Mutex<HashMap<u64, PathBuf>>>,
    /// Statistics
    stats: Arc<Mutex<StorageStatistics>>,
}

impl DiskStorage {
    /// Create a new disk storage
    pub fn new(base_dir: impl AsRef<Path>, capacity: u64) -> SageResult<Self> {
        let base_dir = base_dir.as_ref().to_path_buf();

        // Create directory if it doesn't exist
        if !base_dir.exists() {
            std::fs::create_dir_all(&base_dir)
                .map_err(|e| SageError::Io(format!("Failed to create cache directory: {}", e)))?;
        }

        Ok(Self {
            base_dir,
            capacity,
            current_size: Arc::new(Mutex::new(0)),
            index: Arc::new(Mutex::new(HashMap::new())),
            stats: Arc::new(Mutex::new(StorageStatistics::default())),
        })
    }

    /// Initialize the disk storage by scanning existing files
    pub async fn initialize(&self) -> SageResult<()> {
        self.initialize_index().await
    }

    /// Get file path for a cache key
    fn get_file_path(&self, key: &CacheKey) -> PathBuf {
        let filename = format!("{}.json", key.hash);
        let namespace_dir = self.base_dir.join(&key.namespace);
        namespace_dir.join(filename)
    }

    /// Initialize the cache index
    async fn initialize_index(&self) -> SageResult<()> {
        let mut index = self.index.lock().await;
        let mut current_size = self.current_size.lock().await;
        let mut stats = self.stats.lock().await;

        // Clear existing index
        index.clear();
        *current_size = 0;
        stats.entry_count = 0;
        stats.size_bytes = 0;

        // Scan cache directory
        let mut total_size = 0;
        let mut entry_count = 0;

        if self.base_dir.exists() {
            let mut dirs = fs::read_dir(&self.base_dir)
                .await
                .map_err(|e| SageError::cache(format!("Failed to read cache directory: {}", e)))?;

            while let Some(dir_entry) = dirs
                .next_entry()
                .await
                .map_err(|e| SageError::cache(format!("Failed to read directory entry: {}", e)))?
            {
                if dir_entry
                    .file_type()
                    .await
                    .map_err(|e| SageError::cache(format!("Failed to get file type: {}", e)))?
                    .is_dir()
                {
                    let _namespace = dir_entry.file_name();
                    let namespace_path = dir_entry.path();

                    let mut files = fs::read_dir(&namespace_path).await.map_err(|e| {
                        SageError::cache(format!("Failed to read namespace directory: {}", e))
                    })?;

                    while let Some(file_entry) = files.next_entry().await.map_err(|e| {
                        SageError::cache(format!("Failed to read file entry: {}", e))
                    })? {
                        if file_entry
                            .file_type()
                            .await
                            .map_err(|e| {
                                SageError::cache(format!("Failed to get file type: {}", e))
                            })?
                            .is_file()
                        {
                            let file_path = file_entry.path();
                            if let Some(filename) = file_path.file_name() {
                                if let Some(filename_str) = filename.to_str() {
                                    if filename_str.ends_with(".json") {
                                        if let Some(key_str) = filename_str.strip_suffix(".json") {
                                            if let Ok(key_hash) = key_str.parse::<u64>() {
                                                let metadata =
                                                    file_entry.metadata().await.map_err(|e| {
                                                        SageError::cache(format!(
                                                            "Failed to get file metadata: {}",
                                                            e
                                                        ))
                                                    })?;

                                                total_size += metadata.len();
                                                entry_count += 1;
                                                index.insert(key_hash, file_path);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        *current_size = total_size;
        stats.entry_count = entry_count;
        stats.size_bytes = total_size;

        Ok(())
    }
}

#[async_trait]
impl CacheStorage for DiskStorage {
    async fn get(&self, key: &CacheKey) -> SageResult<Option<CacheEntry>> {
        let mut stats = self.stats.lock().await;
        let index = self.index.lock().await;

        if let Some(file_path) = index.get(&key.hash) {
            match fs::read_to_string(file_path).await {
                Ok(content) => {
                    match serde_json::from_str::<CacheEntry>(&content) {
                        Ok(entry) => {
                            if entry.is_expired() {
                                // Entry is expired, remove it
                                drop(index);
                                self.remove(key).await?;
                                stats.misses += 1;
                                Ok(None)
                            } else {
                                stats.hits += 1;
                                Ok(Some(entry))
                            }
                        }
                        Err(_) => {
                            // Corrupted entry, remove it
                            drop(index);
                            self.remove(key).await?;
                            stats.misses += 1;
                            Ok(None)
                        }
                    }
                }
                Err(_) => {
                    // File doesn't exist or can't be read
                    stats.misses += 1;
                    Ok(None)
                }
            }
        } else {
            stats.misses += 1;
            Ok(None)
        }
    }

    async fn set(&self, key: CacheKey, entry: CacheEntry) -> SageResult<()> {
        let file_path = self.get_file_path(&key);

        // Create namespace directory if it doesn't exist
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                SageError::cache(format!("Failed to create namespace directory: {}", e))
            })?;
        }

        // Serialize entry
        let content = serde_json::to_string_pretty(&entry)
            .map_err(|e| SageError::cache(format!("Failed to serialize cache entry: {}", e)))?;

        // Write to file
        fs::write(&file_path, content)
            .await
            .map_err(|e| SageError::cache(format!("Failed to write cache file: {}", e)))?;

        let entry_size = entry.size_bytes as u64;

        // Check if we need to evict entries to make space
        loop {
            let mut index = self.index.lock().await;
            let current_size = self.current_size.lock().await;
            let mut stats = self.stats.lock().await;

            if *current_size + entry_size <= self.capacity || index.is_empty() {
                break;
            }

            // Simple eviction: remove oldest entry
            if let Some((oldest_key, oldest_path)) = index.iter().next() {
                let oldest_key = *oldest_key;
                let oldest_path = oldest_path.clone();

                // Remove from index first
                index.remove(&oldest_key);
                stats.evictions += 1;
                stats.entry_count -= 1;

                drop(index);
                drop(stats);

                // Remove file and update size
                if let Ok(metadata) = fs::metadata(&oldest_path).await {
                    let mut current_size = self.current_size.lock().await;
                    *current_size = current_size.saturating_sub(metadata.len());
                }
                let _ = fs::remove_file(&oldest_path).await;
            } else {
                break;
            }
        }

        // Add new entry
        let mut index = self.index.lock().await;
        let mut current_size = self.current_size.lock().await;
        let mut stats = self.stats.lock().await;

        // Add new entry
        if !index.contains_key(&key.hash) {
            stats.entry_count += 1;
        }
        index.insert(key.hash, file_path);
        *current_size += entry_size;
        stats.size_bytes = *current_size;

        Ok(())
    }

    async fn remove(&self, key: &CacheKey) -> SageResult<()> {
        let mut index = self.index.lock().await;
        let mut current_size = self.current_size.lock().await;
        let mut stats = self.stats.lock().await;

        if let Some(file_path) = index.remove(&key.hash) {
            if let Ok(metadata) = fs::metadata(&file_path).await {
                *current_size = current_size.saturating_sub(metadata.len());
                stats.size_bytes = *current_size;
            }

            let _ = fs::remove_file(&file_path).await;
            stats.entry_count -= 1;
            stats.evictions += 1;
        }

        Ok(())
    }

    async fn clear(&self) -> SageResult<()> {
        let mut index = self.index.lock().await;
        let mut current_size = self.current_size.lock().await;
        let mut stats = self.stats.lock().await;

        // Remove all files
        for file_path in index.values() {
            let _ = fs::remove_file(file_path).await;
        }

        // Clear index and stats
        index.clear();
        *current_size = 0;
        *stats = StorageStatistics::default();

        Ok(())
    }

    async fn statistics(&self) -> SageResult<StorageStatistics> {
        let stats = self.stats.lock().await;
        Ok(stats.clone())
    }

    async fn cleanup_expired(&self) -> SageResult<()> {
        let index = self.index.lock().await;
        let expired_keys: Vec<u64> = {
            let mut expired = Vec::new();

            for (key_hash, file_path) in index.iter() {
                if let Ok(content) = fs::read_to_string(file_path).await {
                    if let Ok(entry) = serde_json::from_str::<CacheEntry>(&content) {
                        if entry.is_expired() {
                            expired.push(*key_hash);
                        }
                    }
                }
            }

            expired
        };

        drop(index);

        // Remove expired entries
        for key_hash in expired_keys {
            let dummy_key = CacheKey {
                namespace: String::new(),
                identifier: String::new(),
                hash: key_hash,
            };
            self.remove(&dummy_key).await?;
        }

        Ok(())
    }
}
