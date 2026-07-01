//! Provider/model catalog cache and merge helpers.

use super::provider_registry::{ModelInfo, ProviderInfo};
use crate::error::{SageError, SageResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogFreshness {
    Fresh,
    Stale,
    NotModified,
    StaticFallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogSource {
    Remote,
    Cache,
    StaticFallback,
    Merged,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogCacheEntry {
    pub provider_id: String,
    pub models: Vec<ModelInfo>,
    pub etag: Option<String>,
    pub fetched_at: u64,
    pub ttl_seconds: u64,
    pub freshness: CatalogFreshness,
    pub source: CatalogSource,
    pub last_error: Option<String>,
}

impl CatalogCacheEntry {
    pub fn is_expired_at(&self, now: SystemTime) -> bool {
        epoch_seconds(now).saturating_sub(self.fetched_at) > self.ttl_seconds
    }
}

#[derive(Debug, Clone)]
pub struct ProviderCatalogSnapshot {
    pub provider: ProviderInfo,
    pub freshness: CatalogFreshness,
    pub source: CatalogSource,
    pub etag: Option<String>,
    pub fetched_at: Option<u64>,
    pub ttl_seconds: u64,
    pub last_error: Option<String>,
}

pub struct ModelCatalogManager {
    cache_dir: PathBuf,
    ttl: Duration,
}

impl ModelCatalogManager {
    pub fn new(cache_dir: impl Into<PathBuf>) -> Self {
        Self {
            cache_dir: cache_dir.into(),
            ttl: Duration::from_secs(24 * 60 * 60),
        }
    }

    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    pub fn snapshot(&self, static_provider: &ProviderInfo) -> ProviderCatalogSnapshot {
        match self.load_cache(&static_provider.id) {
            Ok(Some(cache)) if !cache.is_expired_at(SystemTime::now()) => {
                self.snapshot_from_cache(static_provider, cache, CatalogFreshness::Fresh)
            }
            Ok(Some(cache)) => {
                self.snapshot_from_cache(static_provider, cache, CatalogFreshness::Stale)
            }
            Ok(None) => self.static_snapshot(static_provider, None),
            Err(error) => self.static_snapshot(static_provider, Some(error.to_string())),
        }
    }

    pub fn merge_remote(
        &self,
        static_provider: &ProviderInfo,
        remote_models: Vec<ModelInfo>,
        etag: Option<String>,
        now: SystemTime,
    ) -> SageResult<ProviderCatalogSnapshot> {
        let provider = merge_provider_catalog(static_provider, remote_models);
        let cache = CatalogCacheEntry {
            provider_id: static_provider.id.clone(),
            models: provider.models.clone(),
            etag,
            fetched_at: epoch_seconds(now),
            ttl_seconds: self.ttl.as_secs(),
            freshness: CatalogFreshness::Fresh,
            source: CatalogSource::Remote,
            last_error: None,
        };
        self.save_cache(&cache)?;
        Ok(ProviderCatalogSnapshot {
            provider,
            freshness: CatalogFreshness::Fresh,
            source: CatalogSource::Merged,
            etag: cache.etag,
            fetched_at: Some(cache.fetched_at),
            ttl_seconds: cache.ttl_seconds,
            last_error: None,
        })
    }

    pub fn not_modified_snapshot(
        &self,
        static_provider: &ProviderInfo,
        now: SystemTime,
    ) -> ProviderCatalogSnapshot {
        match self.load_cache(&static_provider.id) {
            Ok(Some(mut cache)) => {
                cache.fetched_at = epoch_seconds(now);
                cache.freshness = CatalogFreshness::NotModified;
                let _ = self.save_cache(&cache);
                self.snapshot_from_cache(static_provider, cache, CatalogFreshness::NotModified)
            }
            Ok(None) => self.static_snapshot(
                static_provider,
                Some("remote returned not modified without cache".to_string()),
            ),
            Err(error) => self.static_snapshot(static_provider, Some(error.to_string())),
        }
    }

    fn snapshot_from_cache(
        &self,
        static_provider: &ProviderInfo,
        cache: CatalogCacheEntry,
        freshness: CatalogFreshness,
    ) -> ProviderCatalogSnapshot {
        let provider = merge_provider_catalog(static_provider, cache.models.clone());
        ProviderCatalogSnapshot {
            provider,
            freshness,
            source: CatalogSource::Cache,
            etag: cache.etag,
            fetched_at: Some(cache.fetched_at),
            ttl_seconds: cache.ttl_seconds,
            last_error: cache.last_error,
        }
    }

    fn static_snapshot(
        &self,
        static_provider: &ProviderInfo,
        last_error: Option<String>,
    ) -> ProviderCatalogSnapshot {
        ProviderCatalogSnapshot {
            provider: static_provider.clone(),
            freshness: CatalogFreshness::StaticFallback,
            source: CatalogSource::StaticFallback,
            etag: None,
            fetched_at: None,
            ttl_seconds: self.ttl.as_secs(),
            last_error,
        }
    }

    fn cache_path(&self, provider_id: &str) -> PathBuf {
        self.cache_dir
            .join("model_catalog")
            .join(format!("{provider_id}.json"))
    }

    fn load_cache(&self, provider_id: &str) -> SageResult<Option<CatalogCacheEntry>> {
        let path = self.cache_path(provider_id);
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&path).map_err(|error| {
            SageError::io_with_path(error.to_string(), path.display().to_string())
        })?;
        serde_json::from_str(&content)
            .map(Some)
            .map_err(|error| SageError::config(format!("invalid model catalog cache: {error}")))
    }

    fn save_cache(&self, cache: &CatalogCacheEntry) -> SageResult<()> {
        let path = self.cache_path(&cache.provider_id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                SageError::io_with_path(error.to_string(), parent.display().to_string())
            })?;
        }
        let content = serde_json::to_string_pretty(cache).map_err(|error| {
            SageError::config(format!("serialize model catalog cache: {error}"))
        })?;
        fs::write(&path, content)
            .map_err(|error| SageError::io_with_path(error.to_string(), path.display().to_string()))
    }
}

pub fn merge_provider_catalog(
    static_provider: &ProviderInfo,
    remote_models: Vec<ModelInfo>,
) -> ProviderInfo {
    let mut provider = static_provider.clone();
    for remote in remote_models {
        match provider
            .models
            .iter_mut()
            .find(|model| model.id == remote.id)
        {
            Some(existing) => *existing = merge_model(existing, &remote),
            None => provider.models.push(remote),
        }
    }
    provider.models.sort_by(|a, b| a.id.cmp(&b.id));
    if !provider.models.iter().any(|model| model.default)
        && let Some(first) = provider.models.first_mut()
    {
        first.default = true;
    }
    provider
}

fn merge_model(existing: &ModelInfo, remote: &ModelInfo) -> ModelInfo {
    ModelInfo {
        id: existing.id.clone(),
        name: remote.name.clone(),
        default: existing.default || remote.default,
        context_window: remote.context_window.or(existing.context_window),
        max_output_tokens: remote.max_output_tokens.or(existing.max_output_tokens),
    }
}

fn epoch_seconds(now: SystemTime) -> u64 {
    now.duration_since(SystemTime::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn provider() -> ProviderInfo {
        ProviderInfo {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Test".to_string(),
            api_base_url: "https://example.test".to_string(),
            env_var: "TEST_API_KEY".to_string(),
            help_url: None,
            requires_api_key: true,
            models: vec![ModelInfo {
                id: "static".to_string(),
                name: "Static".to_string(),
                default: true,
                context_window: Some(100),
                max_output_tokens: Some(10),
            }],
        }
    }

    fn model(id: &str) -> ModelInfo {
        ModelInfo {
            id: id.to_string(),
            name: id.to_string(),
            default: false,
            context_window: Some(200),
            max_output_tokens: Some(20),
        }
    }

    #[test]
    fn remote_catalog_merges_with_static_models() {
        let merged = merge_provider_catalog(&provider(), vec![model("remote"), model("static")]);
        assert!(merged.models.iter().any(|model| model.id == "remote"));
        let static_model = merged
            .models
            .iter()
            .find(|model| model.id == "static")
            .unwrap();
        assert_eq!(static_model.context_window, Some(200));
        assert!(static_model.default);
    }

    #[test]
    fn catalog_uses_static_fallback_without_cache() {
        let dir = tempdir().unwrap();
        let manager = ModelCatalogManager::new(dir.path());
        let snapshot = manager.snapshot(&provider());
        assert_eq!(snapshot.freshness, CatalogFreshness::StaticFallback);
        assert_eq!(snapshot.provider.models.len(), 1);
    }

    #[test]
    fn catalog_tracks_etag_and_not_modified_refresh() {
        let dir = tempdir().unwrap();
        let manager = ModelCatalogManager::new(dir.path());
        let first = manager
            .merge_remote(
                &provider(),
                vec![model("remote")],
                Some("etag-1".to_string()),
                SystemTime::UNIX_EPOCH + Duration::from_secs(10),
            )
            .unwrap();
        assert_eq!(first.etag.as_deref(), Some("etag-1"));

        let second = manager.not_modified_snapshot(
            &provider(),
            SystemTime::UNIX_EPOCH + Duration::from_secs(20),
        );
        assert_eq!(second.freshness, CatalogFreshness::NotModified);
        assert!(
            second
                .provider
                .models
                .iter()
                .any(|model| model.id == "remote")
        );
        assert_eq!(second.fetched_at, Some(20));
    }

    #[test]
    fn expired_cache_is_marked_stale() {
        let dir = tempdir().unwrap();
        let manager = ModelCatalogManager::new(dir.path()).with_ttl(Duration::from_secs(0));
        manager
            .merge_remote(
                &provider(),
                vec![model("remote")],
                None,
                SystemTime::UNIX_EPOCH,
            )
            .unwrap();

        let snapshot = manager.snapshot(&provider());
        assert_eq!(snapshot.freshness, CatalogFreshness::Stale);
    }
}
