//! File-based checkpoint storage implementation

use crate::error::{SageError, SageResult};
use async_trait::async_trait;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::super::types::{Checkpoint, CheckpointId, FileSnapshot, FileState};
use super::compression::{compress_content, compute_hash, decompress_content};
use super::{CheckpointStorage, CheckpointSummary};

/// File-based checkpoint storage
///
/// Stores checkpoints in a directory structure:
/// ```text
/// base_path/
///   checkpoints/
///     {checkpoint_id}.json
///   content/
///     {content_hash}.dat
///   index.json
/// ```
pub struct FileCheckpointStorage {
    base_path: PathBuf,
    /// Maximum content size to store inline (default: 100KB)
    max_inline_size: usize,
}

impl FileCheckpointStorage {
    /// Create a new file-based storage
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
            max_inline_size: 100 * 1024, // 100KB
        }
    }

    /// Set the maximum content size (in bytes) to store inline
    #[cfg(test)]
    pub fn with_max_inline_size(mut self, size: usize) -> Self {
        self.max_inline_size = size;
        self
    }

    /// Get the checkpoints directory
    fn checkpoints_dir(&self) -> PathBuf {
        self.base_path.join("checkpoints")
    }

    /// Get the content directory
    fn content_dir(&self) -> PathBuf {
        self.base_path.join("content")
    }

    /// Get the path for a checkpoint file
    fn checkpoint_path(&self, id: &CheckpointId) -> PathBuf {
        self.checkpoints_dir().join(format!("{}.json", id.as_str()))
    }

    /// Get the path for content file
    fn content_path(&self, content_ref: &str) -> PathBuf {
        self.content_dir().join(format!("{}.dat", content_ref))
    }

    /// Ensure directories exist
    async fn ensure_dirs(&self) -> SageResult<()> {
        fs::create_dir_all(self.checkpoints_dir())
            .await
            .map_err(|e| {
                SageError::storage(format!("Failed to create checkpoints directory: {}", e))
            })?;
        fs::create_dir_all(self.content_dir()).await.map_err(|e| {
            SageError::storage(format!("Failed to create content directory: {}", e))
        })?;
        Ok(())
    }

    /// Process file snapshots - externalize large content
    async fn process_for_storage(&self, checkpoint: &Checkpoint) -> SageResult<Checkpoint> {
        let mut processed = checkpoint.clone();
        let mut processed_files = Vec::new();

        for file in &checkpoint.files {
            let processed_file = self.process_file_snapshot(file).await?;
            processed_files.push(processed_file);
        }

        processed.files = processed_files;
        Ok(processed)
    }

    /// Process a single file snapshot
    async fn process_file_snapshot(&self, file: &FileSnapshot) -> SageResult<FileSnapshot> {
        let mut processed = file.clone();

        match &file.state {
            FileState::Exists { content, .. } => {
                if let Some(c) = content {
                    if c.len() > self.max_inline_size {
                        let content_ref = self.store_content(c).await?;
                        processed.state = FileState::Exists {
                            content: None,
                            content_ref: Some(content_ref),
                        };
                    }
                }
            }
            FileState::Created { content, .. } => {
                if let Some(c) = content {
                    if c.len() > self.max_inline_size {
                        let content_ref = self.store_content(c).await?;
                        processed.state = FileState::Created {
                            content: None,
                            content_ref: Some(content_ref),
                        };
                    }
                }
            }
            FileState::Modified {
                original_content,
                new_content,
                ..
            } => {
                let orig_ref = if let Some(c) = original_content {
                    if c.len() > self.max_inline_size {
                        Some(self.store_content(c).await?)
                    } else {
                        None
                    }
                } else {
                    None
                };

                let new_ref = if let Some(c) = new_content {
                    if c.len() > self.max_inline_size {
                        Some(self.store_content(c).await?)
                    } else {
                        None
                    }
                } else {
                    None
                };

                if orig_ref.is_some() || new_ref.is_some() {
                    processed.state = FileState::Modified {
                        original_content: if orig_ref.is_some() {
                            None
                        } else {
                            original_content.clone()
                        },
                        original_content_ref: orig_ref,
                        new_content: if new_ref.is_some() {
                            None
                        } else {
                            new_content.clone()
                        },
                        new_content_ref: new_ref,
                    };
                }
            }
            FileState::Deleted => {}
        }

        Ok(processed)
    }

    /// Restore externalized content in file snapshots
    async fn restore_content(&self, checkpoint: &Checkpoint) -> SageResult<Checkpoint> {
        let mut restored = checkpoint.clone();
        let mut restored_files = Vec::new();

        for file in &checkpoint.files {
            let restored_file = self.restore_file_content(file).await?;
            restored_files.push(restored_file);
        }

        restored.files = restored_files;
        Ok(restored)
    }

    /// Restore content in a single file snapshot
    async fn restore_file_content(&self, file: &FileSnapshot) -> SageResult<FileSnapshot> {
        let mut restored = file.clone();

        match &file.state {
            FileState::Exists {
                content,
                content_ref,
            } => {
                if content.is_none() {
                    if let Some(cref) = content_ref {
                        let loaded = self.load_content(cref).await?;
                        restored.state = FileState::Exists {
                            content: loaded,
                            content_ref: content_ref.clone(),
                        };
                    }
                }
            }
            FileState::Created {
                content,
                content_ref,
            } => {
                if content.is_none() {
                    if let Some(cref) = content_ref {
                        let loaded = self.load_content(cref).await?;
                        restored.state = FileState::Created {
                            content: loaded,
                            content_ref: content_ref.clone(),
                        };
                    }
                }
            }
            FileState::Modified {
                original_content,
                original_content_ref,
                new_content,
                new_content_ref,
            } => {
                let orig = if original_content.is_none() {
                    if let Some(cref) = original_content_ref {
                        self.load_content(cref).await?
                    } else {
                        None
                    }
                } else {
                    original_content.clone()
                };

                let new = if new_content.is_none() {
                    if let Some(cref) = new_content_ref {
                        self.load_content(cref).await?
                    } else {
                        None
                    }
                } else {
                    new_content.clone()
                };

                restored.state = FileState::Modified {
                    original_content: orig,
                    original_content_ref: original_content_ref.clone(),
                    new_content: new,
                    new_content_ref: new_content_ref.clone(),
                };
            }
            FileState::Deleted => {}
        }

        Ok(restored)
    }
}

#[async_trait]
impl CheckpointStorage for FileCheckpointStorage {
    async fn save(&self, checkpoint: &Checkpoint) -> SageResult<()> {
        self.ensure_dirs().await?;

        // Process checkpoint to externalize large content
        let processed = self.process_for_storage(checkpoint).await?;

        // Serialize to JSON
        let json = serde_json::to_string_pretty(&processed)
            .map_err(|e| SageError::storage(format!("Failed to serialize checkpoint: {}", e)))?;

        // Write to file
        let path = self.checkpoint_path(&checkpoint.id);
        let mut file = fs::File::create(&path)
            .await
            .map_err(|e| SageError::storage(format!("Failed to create checkpoint file: {}", e)))?;

        file.write_all(json.as_bytes())
            .await
            .map_err(|e| SageError::storage(format!("Failed to write checkpoint file: {}", e)))?;

        tracing::debug!("Saved checkpoint {} to {:?}", checkpoint.id, path);
        Ok(())
    }

    async fn load(&self, id: &CheckpointId) -> SageResult<Option<Checkpoint>> {
        let path = self.checkpoint_path(id);

        if !path.exists() {
            return Ok(None);
        }

        let mut file = fs::File::open(&path)
            .await
            .map_err(|e| SageError::storage(format!("Failed to open checkpoint file: {}", e)))?;

        let mut content = String::new();
        file.read_to_string(&mut content)
            .await
            .map_err(|e| SageError::storage(format!("Failed to read checkpoint file: {}", e)))?;

        let checkpoint: Checkpoint = serde_json::from_str(&content)
            .map_err(|e| SageError::storage(format!("Failed to deserialize checkpoint: {}", e)))?;

        // Restore externalized content
        let restored = self.restore_content(&checkpoint).await?;

        Ok(Some(restored))
    }

    async fn list(&self) -> SageResult<Vec<CheckpointSummary>> {
        let checkpoints_dir = self.checkpoints_dir();

        if !checkpoints_dir.exists() {
            return Ok(Vec::new());
        }

        let mut summaries = Vec::new();
        let mut entries = fs::read_dir(&checkpoints_dir).await.map_err(|e| {
            SageError::storage(format!("Failed to read checkpoints directory: {}", e))
        })?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| SageError::storage(format!("Failed to read directory entry: {}", e)))?
        {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                if let Some(stem) = path.file_stem() {
                    let id = CheckpointId::from_string(stem.to_string_lossy());
                    if let Ok(Some(checkpoint)) = self.load(&id).await {
                        summaries.push(CheckpointSummary::from(&checkpoint));
                    }
                }
            }
        }

        // Sort by creation time (newest first)
        summaries.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(summaries)
    }

    async fn delete(&self, id: &CheckpointId) -> SageResult<()> {
        let path = self.checkpoint_path(id);

        if path.exists() {
            fs::remove_file(&path).await.map_err(|e| {
                SageError::storage(format!("Failed to delete checkpoint file: {}", e))
            })?;
            tracing::debug!("Deleted checkpoint {}", id);
        }

        Ok(())
    }

    async fn exists(&self, id: &CheckpointId) -> SageResult<bool> {
        Ok(self.checkpoint_path(id).exists())
    }

    async fn latest(&self) -> SageResult<Option<Checkpoint>> {
        let summaries = self.list().await?;
        if let Some(summary) = summaries.first() {
            self.load(&summary.id).await
        } else {
            Ok(None)
        }
    }

    async fn store_content(&self, content: &str) -> SageResult<String> {
        self.ensure_dirs().await?;

        let content_ref = compute_hash(content);
        let path = self.content_path(&content_ref);

        // Check if already exists
        if path.exists() {
            return Ok(content_ref);
        }

        // Compress content
        let compressed = compress_content(content)?;

        // Write compressed content
        let mut file = fs::File::create(&path)
            .await
            .map_err(|e| SageError::storage(format!("Failed to create content file: {}", e)))?;
        file.write_all(&compressed)
            .await
            .map_err(|e| SageError::storage(format!("Failed to write content file: {}", e)))?;

        tracing::debug!(
            "Stored content {} ({} -> {} bytes)",
            content_ref,
            content.len(),
            compressed.len()
        );
        Ok(content_ref)
    }

    async fn load_content(&self, content_ref: &str) -> SageResult<Option<String>> {
        let path = self.content_path(content_ref);

        if !path.exists() {
            return Ok(None);
        }

        // Read compressed content
        let mut file = fs::File::open(&path)
            .await
            .map_err(|e| SageError::storage(format!("Failed to open content file: {}", e)))?;

        let mut compressed = Vec::new();
        file.read_to_end(&mut compressed)
            .await
            .map_err(|e| SageError::storage(format!("Failed to read content file: {}", e)))?;

        // Decompress
        let decompressed = decompress_content(&compressed)?;

        Ok(Some(decompressed))
    }
}
