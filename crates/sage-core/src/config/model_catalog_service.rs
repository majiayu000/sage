//! Runtime refresh service for provider model catalogs.

use super::model_catalog::{CatalogFreshness, ModelCatalogManager, ProviderCatalogSnapshot};
use super::models_api::{FetchedModel, FetchedModelsResponse, ModelsApiClient};
use super::provider_registry::{ModelInfo, ProviderInfo};
use crate::error::{SageError, SageResult};
use async_trait::async_trait;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone)]
struct ModelCatalogRefreshOptions {
    cache_root: PathBuf,
    ttl: Duration,
    now: SystemTime,
}

impl ModelCatalogRefreshOptions {
    fn new(cache_root: impl Into<PathBuf>) -> Self {
        Self {
            cache_root: cache_root.into(),
            ttl: Duration::from_secs(24 * 60 * 60),
            now: SystemTime::now(),
        }
    }

    #[cfg(test)]
    fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    #[cfg(test)]
    fn with_now(mut self, now: SystemTime) -> Self {
        self.now = now;
        self
    }
}

#[async_trait]
trait ModelCatalogFetcher: Send + Sync {
    async fn fetch_provider_models(
        &self,
        provider_id: &str,
        base_url: &str,
        api_key: Option<&str>,
        etag: Option<&str>,
    ) -> SageResult<FetchedModelsResponse>;
}

#[async_trait]
impl ModelCatalogFetcher for ModelsApiClient {
    async fn fetch_provider_models(
        &self,
        provider_id: &str,
        base_url: &str,
        api_key: Option<&str>,
        etag: Option<&str>,
    ) -> SageResult<FetchedModelsResponse> {
        match provider_id {
            "anthropic" | "glm" | "zhipu" => {
                self.fetch_anthropic_models_with_etag(base_url, api_key.unwrap_or_default(), etag)
                    .await
            }
            "openai" | "openrouter" | "zai" | "moonshot" | "kimi" => {
                self.fetch_openai_models_with_etag(base_url, api_key.unwrap_or_default(), etag)
                    .await
            }
            "ollama" => {
                let models = self.fetch_ollama_models(base_url).await?;
                Ok(FetchedModelsResponse {
                    models,
                    etag: None,
                    not_modified: false,
                })
            }
            _ => Err(SageError::config(format!(
                "provider '{provider_id}' does not support live model catalog refresh"
            ))),
        }
    }
}

pub async fn refresh_provider_model_catalog(
    provider_id: &str,
    static_provider: &ProviderInfo,
    base_url: &str,
    api_key: Option<&str>,
    cache_root: impl Into<PathBuf>,
) -> SageResult<ProviderCatalogSnapshot> {
    let client = ModelsApiClient::new();
    refresh_provider_model_catalog_with_fetcher(
        provider_id,
        static_provider,
        base_url,
        api_key,
        ModelCatalogRefreshOptions::new(cache_root),
        &client,
    )
    .await
}

async fn refresh_provider_model_catalog_with_fetcher(
    provider_id: &str,
    static_provider: &ProviderInfo,
    base_url: &str,
    api_key: Option<&str>,
    options: ModelCatalogRefreshOptions,
    fetcher: &dyn ModelCatalogFetcher,
) -> SageResult<ProviderCatalogSnapshot> {
    let manager = ModelCatalogManager::new(&options.cache_root).with_ttl(options.ttl);
    let cached = manager.snapshot(static_provider);
    if cached.freshness == CatalogFreshness::Fresh {
        return Ok(cached);
    }

    if !supports_live_refresh(provider_id) {
        return Ok(manager.fallback_snapshot(
            static_provider,
            "provider does not support live model catalog refresh",
        ));
    }

    if static_provider.requires_api_key && api_key.filter(|key| !key.is_empty()).is_none() {
        return Ok(manager.fallback_snapshot(
            static_provider,
            "missing API key for live model catalog refresh",
        ));
    }

    let response = match fetcher
        .fetch_provider_models(
            provider_id,
            base_url,
            api_key.filter(|key| !key.is_empty()),
            cached.etag.as_deref(),
        )
        .await
    {
        Ok(response) => response,
        Err(error) => {
            let reason = model_catalog_error_reason(&error).to_string();
            tracing::warn!(
                provider = provider_id,
                reason = %reason,
                "failed to refresh live model catalog; using cached or static fallback"
            );
            return Ok(manager.fallback_snapshot(static_provider, reason));
        }
    };

    if response.not_modified {
        return Ok(manager.not_modified_snapshot(static_provider, options.now));
    }

    let models = response
        .models
        .into_iter()
        .map(model_info_from_fetched)
        .collect();
    manager.merge_remote(static_provider, models, response.etag, options.now)
}

fn supports_live_refresh(provider_id: &str) -> bool {
    matches!(
        provider_id,
        "anthropic"
            | "glm"
            | "zhipu"
            | "openai"
            | "openrouter"
            | "zai"
            | "moonshot"
            | "kimi"
            | "ollama"
    )
}

fn model_info_from_fetched(model: FetchedModel) -> ModelInfo {
    ModelInfo {
        id: model.id,
        name: model.name,
        default: false,
        context_window: None,
        max_output_tokens: None,
    }
}

pub fn model_catalog_error_reason(error: &dyn std::fmt::Display) -> &'static str {
    let message = error.to_string().to_ascii_lowercase();
    if message.contains("401")
        || message.contains("403")
        || message.contains("unauthorized")
        || message.contains("forbidden")
        || message.contains("invalid api key")
    {
        "authentication or authorization error"
    } else if message.contains("429") || message.contains("rate limit") {
        "rate limit error"
    } else if message.contains("timeout") || message.contains("timed out") {
        "network timeout"
    } else if message.contains("parse response")
        || message.contains("decode")
        || message.contains("json")
    {
        "response parse error"
    } else if message.contains("failed to fetch")
        || message.contains("connection")
        || message.contains("dns")
        || message.contains("request")
    {
        "network or endpoint error"
    } else {
        "provider request error"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tempfile::tempdir;

    struct StubFetcher {
        calls: AtomicUsize,
        etags: Mutex<Vec<Option<String>>>,
        outcome: StubOutcome,
    }

    #[derive(Clone)]
    enum StubOutcome {
        Success(FetchedModelsResponse),
        Error(&'static str),
    }

    #[async_trait]
    impl ModelCatalogFetcher for StubFetcher {
        async fn fetch_provider_models(
            &self,
            _provider_id: &str,
            _base_url: &str,
            _api_key: Option<&str>,
            etag: Option<&str>,
        ) -> SageResult<FetchedModelsResponse> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.etags
                .lock()
                .expect("etag mutex poisoned")
                .push(etag.map(ToString::to_string));
            match self.outcome.clone() {
                StubOutcome::Success(response) => Ok(response),
                StubOutcome::Error(message) => Err(SageError::llm(message.to_string())),
            }
        }
    }

    fn provider() -> ProviderInfo {
        ProviderInfo {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
            description: "OpenAI".to_string(),
            api_base_url: "https://api.openai.com/v1".to_string(),
            env_var: "OPENAI_API_KEY".to_string(),
            help_url: None,
            requires_api_key: true,
            models: vec![ModelInfo {
                id: "static-model".to_string(),
                name: "Static".to_string(),
                default: true,
                context_window: Some(100),
                max_output_tokens: Some(10),
            }],
        }
    }

    fn fetched(id: &str) -> FetchedModel {
        FetchedModel {
            id: id.to_string(),
            name: id.to_string(),
        }
    }

    fn success(models: Vec<FetchedModel>, etag: Option<&str>) -> FetchedModelsResponse {
        FetchedModelsResponse {
            models,
            etag: etag.map(ToString::to_string),
            not_modified: false,
        }
    }

    #[tokio::test]
    async fn fresh_cache_skips_remote_fetch() {
        let dir = tempdir().expect("temp dir");
        let manager = ModelCatalogManager::new(dir.path()).with_ttl(Duration::from_secs(60));
        let now = SystemTime::now();
        manager
            .merge_remote(
                &provider(),
                vec![model_info_from_fetched(fetched("cached-model"))],
                Some("etag-1".to_string()),
                now,
            )
            .expect("cache write");
        let fetcher = StubFetcher {
            calls: AtomicUsize::new(0),
            etags: Mutex::new(Vec::new()),
            outcome: StubOutcome::Error("should not fetch"),
        };

        let snapshot = refresh_provider_model_catalog_with_fetcher(
            "openai",
            &provider(),
            "https://api.openai.com/v1",
            Some("key"),
            ModelCatalogRefreshOptions::new(dir.path())
                .with_ttl(Duration::from_secs(60))
                .with_now(now),
            &fetcher,
        )
        .await
        .expect("snapshot");

        assert_eq!(fetcher.calls.load(Ordering::SeqCst), 0);
        assert_eq!(snapshot.freshness, CatalogFreshness::Fresh);
        assert!(
            snapshot
                .provider
                .models
                .iter()
                .any(|m| m.id == "cached-model")
        );
    }

    #[tokio::test]
    async fn stale_cache_remote_success_refreshes_catalog() {
        let dir = tempdir().expect("temp dir");
        let manager = ModelCatalogManager::new(dir.path()).with_ttl(Duration::from_secs(0));
        manager
            .merge_remote(
                &provider(),
                vec![model_info_from_fetched(fetched("old-model"))],
                Some("etag-old".to_string()),
                SystemTime::UNIX_EPOCH + Duration::from_secs(1),
            )
            .expect("cache write");
        let fetcher = StubFetcher {
            calls: AtomicUsize::new(0),
            etags: Mutex::new(Vec::new()),
            outcome: StubOutcome::Success(success(vec![fetched("new-model")], Some("etag-new"))),
        };

        let snapshot = refresh_provider_model_catalog_with_fetcher(
            "openai",
            &provider(),
            "https://api.openai.com/v1",
            Some("key"),
            ModelCatalogRefreshOptions::new(dir.path())
                .with_ttl(Duration::from_secs(0))
                .with_now(SystemTime::UNIX_EPOCH + Duration::from_secs(20)),
            &fetcher,
        )
        .await
        .expect("snapshot");

        assert_eq!(snapshot.freshness, CatalogFreshness::Fresh);
        assert_eq!(snapshot.etag.as_deref(), Some("etag-new"));
        assert!(snapshot.provider.models.iter().any(|m| m.id == "new-model"));
        assert_eq!(
            fetcher
                .etags
                .lock()
                .expect("etag mutex poisoned")
                .as_slice(),
            &[Some("etag-old".to_string())]
        );
    }

    #[tokio::test]
    async fn remote_failure_returns_stale_cache_with_reason() {
        let dir = tempdir().expect("temp dir");
        let manager = ModelCatalogManager::new(dir.path()).with_ttl(Duration::from_secs(0));
        manager
            .merge_remote(
                &provider(),
                vec![model_info_from_fetched(fetched("cached-model"))],
                None,
                SystemTime::UNIX_EPOCH + Duration::from_secs(1),
            )
            .expect("cache write");
        let fetcher = StubFetcher {
            calls: AtomicUsize::new(0),
            etags: Mutex::new(Vec::new()),
            outcome: StubOutcome::Error("Failed to fetch OpenAI models: dns error"),
        };

        let snapshot = refresh_provider_model_catalog_with_fetcher(
            "openai",
            &provider(),
            "https://api.openai.com/v1",
            Some("key"),
            ModelCatalogRefreshOptions::new(dir.path())
                .with_ttl(Duration::from_secs(0))
                .with_now(SystemTime::UNIX_EPOCH + Duration::from_secs(20)),
            &fetcher,
        )
        .await
        .expect("snapshot");

        assert_eq!(snapshot.freshness, CatalogFreshness::Stale);
        assert_eq!(
            snapshot.last_error.as_deref(),
            Some("network or endpoint error")
        );
        assert!(
            snapshot
                .provider
                .models
                .iter()
                .any(|m| m.id == "cached-model")
        );
    }

    #[tokio::test]
    async fn not_modified_updates_cache_timestamp() {
        let dir = tempdir().expect("temp dir");
        let manager = ModelCatalogManager::new(dir.path()).with_ttl(Duration::from_secs(0));
        manager
            .merge_remote(
                &provider(),
                vec![model_info_from_fetched(fetched("cached-model"))],
                Some("etag-1".to_string()),
                SystemTime::UNIX_EPOCH + Duration::from_secs(1),
            )
            .expect("cache write");
        let fetcher = StubFetcher {
            calls: AtomicUsize::new(0),
            etags: Mutex::new(Vec::new()),
            outcome: StubOutcome::Success(FetchedModelsResponse {
                models: Vec::new(),
                etag: Some("etag-1".to_string()),
                not_modified: true,
            }),
        };

        let snapshot = refresh_provider_model_catalog_with_fetcher(
            "openai",
            &provider(),
            "https://api.openai.com/v1",
            Some("key"),
            ModelCatalogRefreshOptions::new(dir.path())
                .with_ttl(Duration::from_secs(0))
                .with_now(SystemTime::UNIX_EPOCH + Duration::from_secs(20)),
            &fetcher,
        )
        .await
        .expect("snapshot");

        assert_eq!(snapshot.freshness, CatalogFreshness::NotModified);
        assert_eq!(snapshot.fetched_at, Some(20));
        assert!(
            snapshot
                .provider
                .models
                .iter()
                .any(|m| m.id == "cached-model")
        );
    }

    #[tokio::test]
    async fn missing_api_key_returns_static_fallback_without_fetch() {
        let dir = tempdir().expect("temp dir");
        let fetcher = StubFetcher {
            calls: AtomicUsize::new(0),
            etags: Mutex::new(Vec::new()),
            outcome: StubOutcome::Error("should not fetch"),
        };

        let snapshot = refresh_provider_model_catalog_with_fetcher(
            "openai",
            &provider(),
            "https://api.openai.com/v1",
            None,
            ModelCatalogRefreshOptions::new(dir.path()),
            &fetcher,
        )
        .await
        .expect("snapshot");

        assert_eq!(fetcher.calls.load(Ordering::SeqCst), 0);
        assert_eq!(snapshot.freshness, CatalogFreshness::StaticFallback);
        assert_eq!(
            snapshot.last_error.as_deref(),
            Some("missing API key for live model catalog refresh")
        );
    }
}
