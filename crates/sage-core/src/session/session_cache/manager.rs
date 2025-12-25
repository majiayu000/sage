//! SessionCache manager implementation

use crate::error::SageResult;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::persistence::{load_cache_file, save_cache_file};
use super::types::{
    CACHE_FILE_NAME, GLOBAL_CACHE_DIR, PROJECT_CACHE_DIR, SessionCacheConfig, SessionCacheData,
};

/// Session cache manager
pub struct SessionCache {
    /// Configuration
    pub(super) config: SessionCacheConfig,
    /// Global cache data
    pub(super) global_cache: Arc<RwLock<SessionCacheData>>,
    /// Project-specific cache data
    pub(super) project_cache: Arc<RwLock<Option<SessionCacheData>>>,
    /// Global cache file path
    pub(super) global_path: PathBuf,
    /// Project cache file path (if any)
    pub(super) project_path: Option<PathBuf>,
    /// Whether cache has unsaved changes
    pub(super) dirty: Arc<RwLock<bool>>,
}

impl SessionCache {
    /// Create a new session cache
    pub async fn new(config: SessionCacheConfig) -> SageResult<Self> {
        let global_path = config.global_cache_path.clone().unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(GLOBAL_CACHE_DIR)
                .join(CACHE_FILE_NAME)
        });

        let cache = Self {
            config,
            global_cache: Arc::new(RwLock::new(SessionCacheData::new())),
            project_cache: Arc::new(RwLock::new(None)),
            global_path,
            project_path: None,
            dirty: Arc::new(RwLock::new(false)),
        };

        // Load existing cache
        cache.load_global().await?;

        Ok(cache)
    }

    /// Initialize with project directory
    pub async fn with_project_dir(mut self, project_dir: &Path) -> SageResult<Self> {
        if self.config.use_project_cache {
            self.project_path = Some(project_dir.join(PROJECT_CACHE_DIR).join(CACHE_FILE_NAME));
            self.load_project().await?;
        }
        Ok(self)
    }

    /// Load global cache from disk
    async fn load_global(&self) -> SageResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        if let Some(data) = load_cache_file(&self.global_path).await? {
            *self.global_cache.write().await = data;
        }

        Ok(())
    }

    /// Load project cache from disk
    async fn load_project(&self) -> SageResult<()> {
        if !self.config.enabled || !self.config.use_project_cache {
            return Ok(());
        }

        if let Some(path) = &self.project_path {
            if let Some(data) = load_cache_file(path).await? {
                *self.project_cache.write().await = Some(data);
            }
        }

        Ok(())
    }

    /// Save cache to disk
    pub async fn save(&self) -> SageResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Save global cache
        save_cache_file(&self.global_path, &*self.global_cache.read().await).await?;

        // Save project cache if exists
        if let Some(path) = &self.project_path {
            if let Some(data) = &*self.project_cache.read().await {
                save_cache_file(path, data).await?;
            }
        }

        *self.dirty.write().await = false;
        Ok(())
    }
}

impl Default for SessionCache {
    fn default() -> Self {
        let config = SessionCacheConfig::default();
        let global_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(GLOBAL_CACHE_DIR)
            .join(CACHE_FILE_NAME);

        Self {
            config,
            global_cache: Arc::new(RwLock::new(SessionCacheData::new())),
            project_cache: Arc::new(RwLock::new(None)),
            global_path,
            project_path: None,
            dirty: Arc::new(RwLock::new(false)),
        }
    }
}
