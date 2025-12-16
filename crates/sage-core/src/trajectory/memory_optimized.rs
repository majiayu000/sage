//! Memory-optimized trajectory recording
//!
//! This module provides memory-efficient trajectory recording that prevents
//! memory leaks during long-running agent sessions.

use crate::error::SageResult;
use crate::trajectory::recorder::TrajectoryRecord;
use crate::types::Id;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs;
use tokio::sync::{Mutex, RwLock};
use tokio::time::interval;

/// Configuration for memory-optimized trajectory recording
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryOptimizedConfig {
    /// Maximum number of records to keep in memory
    pub max_memory_records: usize,
    /// Maximum memory usage in bytes
    pub max_memory_bytes: usize,
    /// Directory for persistent storage
    pub storage_dir: PathBuf,
    /// How often to flush records to disk
    pub flush_interval: Duration,
    /// Maximum age of records before archiving
    pub max_record_age: Duration,
    /// Enable compression for stored records
    pub enable_compression: bool,
    /// Batch size for disk operations
    pub batch_size: usize,
}

impl Default for MemoryOptimizedConfig {
    fn default() -> Self {
        Self {
            max_memory_records: 1000,
            max_memory_bytes: 50 * 1024 * 1024, // 50MB
            storage_dir: PathBuf::from("trajectories"),
            flush_interval: Duration::from_secs(30),
            max_record_age: Duration::from_secs(3600 * 24), // 24 hours
            enable_compression: true,
            batch_size: 100,
        }
    }
}

/// Memory-optimized trajectory recorder
pub struct MemoryOptimizedRecorder {
    /// Configuration
    config: MemoryOptimizedConfig,
    /// In-memory record buffer (LRU-like)
    memory_buffer: Arc<RwLock<VecDeque<TrajectoryRecord>>>,
    /// Current memory usage in bytes
    current_memory_bytes: Arc<Mutex<usize>>,
    /// Index for fast lookups
    record_index: Arc<RwLock<HashMap<Id, usize>>>,
    /// Statistics
    stats: Arc<Mutex<RecorderStatistics>>,
    /// Background flush task handle
    _flush_task: tokio::task::JoinHandle<()>,
}

/// Recorder statistics
#[derive(Debug, Clone, Default)]
pub struct RecorderStatistics {
    /// Total records processed
    pub total_records: u64,
    /// Records currently in memory
    pub memory_records: usize,
    /// Current memory usage
    pub memory_bytes: usize,
    /// Records flushed to disk
    pub flushed_records: u64,
    /// Records archived
    pub archived_records: u64,
    /// Memory evictions
    pub memory_evictions: u64,
}

impl MemoryOptimizedRecorder {
    /// Create a new memory-optimized recorder
    pub async fn new(config: MemoryOptimizedConfig) -> SageResult<Self> {
        // Create storage directory
        if !config.storage_dir.exists() {
            fs::create_dir_all(&config.storage_dir).await?;
        }

        let memory_buffer = Arc::new(RwLock::new(VecDeque::new()));
        let current_memory_bytes = Arc::new(Mutex::new(0));
        let record_index = Arc::new(RwLock::new(HashMap::new()));
        let stats = Arc::new(Mutex::new(RecorderStatistics::default()));

        // Start background flush task
        let flush_task = Self::start_flush_task(
            config.clone(),
            memory_buffer.clone(),
            current_memory_bytes.clone(),
            record_index.clone(),
            stats.clone(),
        );

        Ok(Self {
            config,
            memory_buffer,
            current_memory_bytes,
            record_index,
            stats,
            _flush_task: flush_task,
        })
    }

    /// Add a record to the trajectory
    pub async fn add_record(&self, record: TrajectoryRecord) -> SageResult<()> {
        let record_size = self.estimate_record_size(&record);

        // Check if we need to evict records
        self.ensure_memory_capacity(record_size).await?;

        // Add to memory buffer
        {
            let mut buffer = self.memory_buffer.write().await;
            let mut index = self.record_index.write().await;
            let mut memory_bytes = self.current_memory_bytes.lock().await;
            let mut stats = self.stats.lock().await;

            // Add to buffer
            buffer.push_back(record.clone());
            index.insert(record.id.clone(), buffer.len() - 1);
            *memory_bytes += record_size;

            // Update stats
            stats.total_records += 1;
            stats.memory_records = buffer.len();
            stats.memory_bytes = *memory_bytes;
        }

        Ok(())
    }

    /// Get a record by ID
    pub async fn get_record(&self, id: &Id) -> SageResult<Option<TrajectoryRecord>> {
        // Check memory buffer first
        {
            let buffer = self.memory_buffer.read().await;
            let index = self.record_index.read().await;

            if let Some(&position) = index.get(id) {
                if let Some(record) = buffer.get(position) {
                    return Ok(Some(record.clone()));
                }
            }
        }

        // Check disk storage
        self.load_from_disk(id).await
    }

    /// Get recent records
    pub async fn get_recent_records(&self, limit: usize) -> SageResult<Vec<TrajectoryRecord>> {
        let buffer = self.memory_buffer.read().await;
        let records = buffer.iter().rev().take(limit).cloned().collect();
        Ok(records)
    }

    /// Get statistics
    pub async fn statistics(&self) -> RecorderStatistics {
        let stats = self.stats.lock().await;
        stats.clone()
    }

    /// Force flush all records to disk
    pub async fn flush(&self) -> SageResult<()> {
        self.flush_to_disk().await
    }

    /// Estimate the memory size of a record
    fn estimate_record_size(&self, record: &TrajectoryRecord) -> usize {
        // Rough estimation based on serialized size
        serde_json::to_string(record)
            .map(|s| s.len())
            .unwrap_or(1024) // Default estimate
    }

    /// Ensure we have enough memory capacity
    async fn ensure_memory_capacity(&self, needed_bytes: usize) -> SageResult<()> {
        loop {
            let (should_evict, _current_memory, current_records) = {
                let memory_bytes = self.current_memory_bytes.lock().await;
                let stats = self.stats.lock().await;

                let should_evict = (*memory_bytes + needed_bytes > self.config.max_memory_bytes)
                    || (stats.memory_records >= self.config.max_memory_records);

                (should_evict, *memory_bytes, stats.memory_records)
            };

            if !should_evict || current_records == 0 {
                break;
            }

            // Evict oldest record (this will acquire locks internally)
            let evicted_size = self.evict_oldest_record().await?;

            // Update memory tracking
            {
                let mut memory_bytes = self.current_memory_bytes.lock().await;
                let mut stats = self.stats.lock().await;

                *memory_bytes = memory_bytes.saturating_sub(evicted_size);
                stats.memory_evictions += 1;
                stats.memory_records = stats.memory_records.saturating_sub(1);
            }
        }

        Ok(())
    }

    /// Evict the oldest record from memory
    async fn evict_oldest_record(&self) -> SageResult<usize> {
        let mut buffer = self.memory_buffer.write().await;
        let mut index = self.record_index.write().await;

        if let Some(record) = buffer.pop_front() {
            let record_size = self.estimate_record_size(&record);

            // Remove from index and update positions
            index.remove(&record.id);

            // Update all positions in index
            for position in index.values_mut() {
                if *position > 0 {
                    *position -= 1;
                }
            }

            // Save to disk before evicting
            self.save_to_disk(&record).await?;

            Ok(record_size)
        } else {
            Ok(0)
        }
    }

    /// Save a record to disk
    async fn save_to_disk(&self, record: &TrajectoryRecord) -> SageResult<()> {
        let file_path = self.config.storage_dir.join(format!("{}.json", record.id));

        let content = if self.config.enable_compression {
            // Simple compression using gzip
            use flate2::Compression;
            use flate2::write::GzEncoder;
            use std::io::Write;

            let json_data = serde_json::to_vec(record)?;

            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&json_data)?;
            encoder.finish()?
        } else {
            serde_json::to_vec_pretty(record)?
        };

        fs::write(&file_path, content).await?;

        Ok(())
    }

    /// Load a record from disk
    async fn load_from_disk(&self, id: &Id) -> SageResult<Option<TrajectoryRecord>> {
        let file_path = self.config.storage_dir.join(format!("{}.json", id));

        if !file_path.exists() {
            return Ok(None);
        }

        let content = fs::read(&file_path).await?;

        let record = if self.config.enable_compression {
            // Decompress
            use flate2::read::GzDecoder;
            use std::io::Read;

            let mut decoder = GzDecoder::new(&content[..]);
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed)?;

            serde_json::from_slice(&decompressed)?
        } else {
            serde_json::from_slice(&content)?
        };

        Ok(Some(record))
    }

    /// Flush records to disk
    async fn flush_to_disk(&self) -> SageResult<()> {
        let buffer = self.memory_buffer.read().await;
        let mut stats = self.stats.lock().await;

        let mut flushed_count = 0;
        for record in buffer.iter() {
            self.save_to_disk(record).await?;
            flushed_count += 1;
        }

        stats.flushed_records += flushed_count;
        Ok(())
    }

    /// Start background flush task
    fn start_flush_task(
        config: MemoryOptimizedConfig,
        memory_buffer: Arc<RwLock<VecDeque<TrajectoryRecord>>>,
        current_memory_bytes: Arc<Mutex<usize>>,
        record_index: Arc<RwLock<HashMap<Id, usize>>>,
        stats: Arc<Mutex<RecorderStatistics>>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = interval(config.flush_interval);

            loop {
                interval.tick().await;

                // Flush records to disk periodically
                let buffer = memory_buffer.read().await;
                if !buffer.is_empty() {
                    drop(buffer);

                    // Create a temporary recorder instance for flushing
                    let temp_recorder = MemoryOptimizedRecorder {
                        config: config.clone(),
                        memory_buffer: memory_buffer.clone(),
                        current_memory_bytes: current_memory_bytes.clone(),
                        record_index: record_index.clone(),
                        stats: stats.clone(),
                        _flush_task: tokio::spawn(async {}), // Dummy task
                    };

                    if let Err(e) = temp_recorder.flush_to_disk().await {
                        tracing::error!("Failed to flush trajectory records: {}", e);
                    }
                }
            }
        })
    }
}
