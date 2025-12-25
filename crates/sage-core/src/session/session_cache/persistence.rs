//! File I/O operations for session cache

use crate::error::{SageError, SageResult};
use std::path::Path;
use tokio::fs;

use super::types::SessionCacheData;

/// Load cache data from a file
pub async fn load_cache_file(path: &Path) -> SageResult<Option<SessionCacheData>> {
    if !path.exists() {
        return Ok(None);
    }

    match fs::read_to_string(path).await {
        Ok(content) => match serde_json::from_str::<SessionCacheData>(&content) {
            Ok(data) => {
                tracing::debug!("Loaded session cache from {:?}", path);
                Ok(Some(data))
            }
            Err(e) => {
                tracing::warn!("Failed to parse cache file at {:?}: {}", path, e);
                Ok(None)
            }
        },
        Err(e) => {
            tracing::debug!("No cache file found at {:?}: {}", path, e);
            Ok(None)
        }
    }
}

/// Save cache data to a file
pub async fn save_cache_file(path: &Path, data: &SessionCacheData) -> SageResult<()> {
    // Ensure directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|e| SageError::io(format!("Failed to create cache directory: {}", e)))?;
    }

    // Update last saved timestamp
    let mut data = data.clone();
    data.last_saved = Some(chrono::Utc::now());

    let content = serde_json::to_string_pretty(&data)?;
    fs::write(path, content)
        .await
        .map_err(|e| SageError::io(format!("Failed to write cache: {}", e)))?;

    tracing::debug!("Saved session cache to {:?}", path);
    Ok(())
}
