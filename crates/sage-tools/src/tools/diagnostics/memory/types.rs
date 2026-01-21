//! Memory tool types and global manager initialization

use sage_core::memory::{MemoryConfig, MemoryManager, SharedMemoryManager};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::OnceCell;

/// Global memory manager instance
static GLOBAL_MEMORY_MANAGER: OnceCell<SharedMemoryManager> = OnceCell::const_new();

/// Initialize the global memory manager
pub async fn init_global_memory_manager(storage_path: Option<PathBuf>) -> Result<(), String> {
    let config = if let Some(path) = storage_path {
        MemoryConfig::with_file_storage(path)
    } else {
        MemoryConfig::default()
    };

    let manager = MemoryManager::new(config)
        .await
        .map_err(|e| format!("Failed to create memory manager: {}", e))?;

    GLOBAL_MEMORY_MANAGER
        .set(Arc::new(manager))
        .map_err(|_| "Memory manager already initialized".to_string())
}

/// Get the global memory manager
pub fn get_global_memory_manager() -> Option<SharedMemoryManager> {
    GLOBAL_MEMORY_MANAGER.get().cloned()
}

/// Ensure memory manager is initialized (creates in-memory if not)
pub(crate) async fn ensure_memory_manager() -> Result<SharedMemoryManager, String> {
    if let Some(manager) = GLOBAL_MEMORY_MANAGER.get() {
        return Ok(manager.clone());
    }

    // Initialize with default in-memory storage
    let config = MemoryConfig::default();
    let manager = MemoryManager::new(config)
        .await
        .map_err(|e| format!("Failed to create default memory manager: {}", e))?;
    let shared = Arc::new(manager);

    // Try to set, if fails (race condition), just get the existing one
    let _ = GLOBAL_MEMORY_MANAGER.set(shared.clone());
    Ok(GLOBAL_MEMORY_MANAGER.get().cloned().unwrap_or(shared))
}
