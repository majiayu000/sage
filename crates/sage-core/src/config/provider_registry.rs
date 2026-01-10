//! Dynamic provider registry with caching
//!
//! This module provides functionality for discovering and caching
//! available LLM providers, similar to Crush's Catwalk integration.

use crate::error::{SageError, SageResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tracing::{debug, warn};

/// Default cache TTL (24 hours)
const DEFAULT_CACHE_TTL_SECS: u64 = 24 * 60 * 60;

/// Information about an LLM model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model identifier (e.g., "claude-sonnet-4-20250514")
    pub id: String,
    /// Display name (e.g., "Claude 4 Sonnet")
    pub name: String,
    /// Whether this is the default model for the provider
    #[serde(default)]
    pub default: bool,
    /// Maximum context window size
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u32>,
    /// Maximum output tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
}

/// Information about an LLM provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    /// Provider identifier (e.g., "anthropic")
    pub id: String,
    /// Display name (e.g., "Anthropic")
    pub name: String,
    /// Short description
    pub description: String,
    /// API base URL
    pub api_base_url: String,
    /// Environment variable name for API key
    pub env_var: String,
    /// Help URL for getting API keys
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help_url: Option<String>,
    /// Available models
    #[serde(default)]
    pub models: Vec<ModelInfo>,
    /// Whether this provider requires an API key
    #[serde(default = "default_true")]
    pub requires_api_key: bool,
}

fn default_true() -> bool {
    true
}

impl ProviderInfo {
    /// Get the default model for this provider
    pub fn default_model(&self) -> Option<&ModelInfo> {
        self.models.iter().find(|m| m.default).or(self.models.first())
    }
}

/// Cached provider list
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProviderCache {
    /// Cache version
    version: String,
    /// Timestamp when cache was created
    timestamp: u64,
    /// Cached providers
    providers: Vec<ProviderInfo>,
}

/// Provider registry with caching
pub struct ProviderRegistry {
    /// Base directory for cache storage
    cache_dir: PathBuf,
    /// Cache TTL
    cache_ttl: Duration,
    /// Loaded providers
    providers: Option<Vec<ProviderInfo>>,
}

impl ProviderRegistry {
    /// Create a new provider registry
    pub fn new(cache_dir: &Path) -> Self {
        Self {
            cache_dir: cache_dir.to_path_buf(),
            cache_ttl: Duration::from_secs(DEFAULT_CACHE_TTL_SECS),
            providers: None,
        }
    }

    /// Create a registry with default paths (~/.sage)
    pub fn with_defaults() -> Self {
        let cache_dir = dirs::home_dir()
            .unwrap_or_default()
            .join(".sage");
        Self::new(&cache_dir)
    }

    /// Set custom cache TTL
    pub fn with_cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = ttl;
        self
    }

    /// Get the cache file path
    fn cache_path(&self) -> PathBuf {
        self.cache_dir.join("provider_cache.json")
    }

    /// Get all available providers
    ///
    /// Loads from cache if available and not expired,
    /// otherwise falls back to embedded providers.
    pub fn get_providers(&mut self) -> &[ProviderInfo] {
        if self.providers.is_none() {
            self.providers = Some(self.load_providers());
        }
        self.providers.as_ref().unwrap()
    }

    /// Force refresh the provider list
    pub fn refresh(&mut self) {
        // Clear current cache
        self.providers = None;

        // Try to fetch from remote (not implemented yet)
        // For now, just reload embedded providers
        let providers = self.embedded_providers();

        // Save to cache
        if let Err(e) = self.save_cache(&providers) {
            warn!("Failed to save provider cache: {}", e);
        }

        self.providers = Some(providers);
    }

    /// Load providers from cache or embedded list
    fn load_providers(&self) -> Vec<ProviderInfo> {
        // Try to load from cache
        if let Some(cached) = self.load_cache() {
            debug!("Loaded {} providers from cache", cached.len());
            return cached;
        }

        // Fall back to embedded providers
        let providers = self.embedded_providers();
        debug!("Using {} embedded providers", providers.len());

        // Try to save to cache for next time
        if let Err(e) = self.save_cache(&providers) {
            debug!("Failed to save provider cache: {}", e);
        }

        providers
    }

    /// Load providers from cache file
    fn load_cache(&self) -> Option<Vec<ProviderInfo>> {
        let cache_path = self.cache_path();

        if !cache_path.exists() {
            return None;
        }

        let content = fs::read_to_string(&cache_path).ok()?;
        let cache: ProviderCache = serde_json::from_str(&content).ok()?;

        // Check if cache is expired
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .ok()?
            .as_secs();

        if now - cache.timestamp > self.cache_ttl.as_secs() {
            debug!("Provider cache expired");
            return None;
        }

        Some(cache.providers)
    }

    /// Save providers to cache file
    fn save_cache(&self, providers: &[ProviderInfo]) -> SageResult<()> {
        if let Some(parent) = self.cache_path().parent() {
            fs::create_dir_all(parent)
                .map_err(|e| SageError::io(format!("Failed to create cache dir: {}", e)))?;
        }

        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| SageError::config(format!("Failed to get timestamp: {}", e)))?
            .as_secs();

        let cache = ProviderCache {
            version: "1.0".to_string(),
            timestamp: now,
            providers: providers.to_vec(),
        };

        let content = serde_json::to_string_pretty(&cache)
            .map_err(|e| SageError::config(format!("Failed to serialize cache: {}", e)))?;

        fs::write(self.cache_path(), content)
            .map_err(|e| SageError::io(format!("Failed to write cache: {}", e)))?;

        Ok(())
    }

    /// Get embedded (built-in) provider list
    pub fn embedded_providers(&self) -> Vec<ProviderInfo> {
        vec![
            ProviderInfo {
                id: "anthropic".to_string(),
                name: "Anthropic".to_string(),
                description: "Claude models (Opus, Sonnet, Haiku)".to_string(),
                api_base_url: "https://api.anthropic.com".to_string(),
                env_var: "ANTHROPIC_API_KEY".to_string(),
                help_url: Some("https://console.anthropic.com/settings/keys".to_string()),
                requires_api_key: true,
                models: vec![
                    ModelInfo {
                        id: "claude-sonnet-4-20250514".to_string(),
                        name: "Claude 4 Sonnet".to_string(),
                        default: true,
                        context_window: Some(200_000),
                        max_output_tokens: Some(64_000),
                    },
                    ModelInfo {
                        id: "claude-opus-4-20250514".to_string(),
                        name: "Claude 4 Opus".to_string(),
                        default: false,
                        context_window: Some(200_000),
                        max_output_tokens: Some(32_000),
                    },
                    ModelInfo {
                        id: "claude-3-5-haiku-20241022".to_string(),
                        name: "Claude 3.5 Haiku".to_string(),
                        default: false,
                        context_window: Some(200_000),
                        max_output_tokens: Some(8_192),
                    },
                ],
            },
            ProviderInfo {
                id: "openai".to_string(),
                name: "OpenAI".to_string(),
                description: "GPT-4 and GPT-3.5 models".to_string(),
                api_base_url: "https://api.openai.com/v1".to_string(),
                env_var: "OPENAI_API_KEY".to_string(),
                help_url: Some("https://platform.openai.com/api-keys".to_string()),
                requires_api_key: true,
                models: vec![
                    ModelInfo {
                        id: "gpt-4o".to_string(),
                        name: "GPT-4o".to_string(),
                        default: true,
                        context_window: Some(128_000),
                        max_output_tokens: Some(16_384),
                    },
                    ModelInfo {
                        id: "gpt-4o-mini".to_string(),
                        name: "GPT-4o Mini".to_string(),
                        default: false,
                        context_window: Some(128_000),
                        max_output_tokens: Some(16_384),
                    },
                    ModelInfo {
                        id: "o1".to_string(),
                        name: "o1".to_string(),
                        default: false,
                        context_window: Some(200_000),
                        max_output_tokens: Some(100_000),
                    },
                ],
            },
            ProviderInfo {
                id: "google".to_string(),
                name: "Google".to_string(),
                description: "Gemini models".to_string(),
                api_base_url: "https://generativelanguage.googleapis.com".to_string(),
                env_var: "GOOGLE_API_KEY".to_string(),
                help_url: Some("https://aistudio.google.com/apikey".to_string()),
                requires_api_key: true,
                models: vec![
                    ModelInfo {
                        id: "gemini-2.0-flash".to_string(),
                        name: "Gemini 2.0 Flash".to_string(),
                        default: true,
                        context_window: Some(1_000_000),
                        max_output_tokens: Some(8_192),
                    },
                    ModelInfo {
                        id: "gemini-2.0-pro".to_string(),
                        name: "Gemini 2.0 Pro".to_string(),
                        default: false,
                        context_window: Some(2_000_000),
                        max_output_tokens: Some(8_192),
                    },
                ],
            },
            ProviderInfo {
                id: "glm".to_string(),
                name: "GLM (智谱)".to_string(),
                description: "Zhipu AI GLM models".to_string(),
                api_base_url: "https://open.bigmodel.cn/api/anthropic".to_string(),
                env_var: "GLM_API_KEY".to_string(),
                help_url: Some("https://open.bigmodel.cn/usercenter/apikeys".to_string()),
                requires_api_key: true,
                models: vec![
                    ModelInfo {
                        id: "glm-4-plus".to_string(),
                        name: "GLM-4 Plus".to_string(),
                        default: true,
                        context_window: Some(128_000),
                        max_output_tokens: Some(4_096),
                    },
                    ModelInfo {
                        id: "glm-4-flash".to_string(),
                        name: "GLM-4 Flash".to_string(),
                        default: false,
                        context_window: Some(128_000),
                        max_output_tokens: Some(4_096),
                    },
                ],
            },
            ProviderInfo {
                id: "ollama".to_string(),
                name: "Ollama".to_string(),
                description: "Local models via Ollama".to_string(),
                api_base_url: "http://localhost:11434".to_string(),
                env_var: "OLLAMA_API_KEY".to_string(),
                help_url: Some("https://ollama.ai".to_string()),
                requires_api_key: false,
                models: vec![
                    ModelInfo {
                        id: "llama3.1".to_string(),
                        name: "Llama 3.1".to_string(),
                        default: true,
                        context_window: Some(128_000),
                        max_output_tokens: None,
                    },
                    ModelInfo {
                        id: "qwen2.5".to_string(),
                        name: "Qwen 2.5".to_string(),
                        default: false,
                        context_window: Some(128_000),
                        max_output_tokens: None,
                    },
                ],
            },
            ProviderInfo {
                id: "openrouter".to_string(),
                name: "OpenRouter".to_string(),
                description: "Access multiple providers via OpenRouter".to_string(),
                api_base_url: "https://openrouter.ai/api/v1".to_string(),
                env_var: "OPENROUTER_API_KEY".to_string(),
                help_url: Some("https://openrouter.ai/keys".to_string()),
                requires_api_key: true,
                models: vec![
                    ModelInfo {
                        id: "anthropic/claude-sonnet-4".to_string(),
                        name: "Claude 4 Sonnet (via OpenRouter)".to_string(),
                        default: true,
                        context_window: Some(200_000),
                        max_output_tokens: Some(64_000),
                    },
                ],
            },
            ProviderInfo {
                id: "azure".to_string(),
                name: "Azure OpenAI".to_string(),
                description: "Azure-hosted OpenAI models".to_string(),
                api_base_url: "".to_string(), // User must configure
                env_var: "AZURE_OPENAI_API_KEY".to_string(),
                help_url: Some("https://portal.azure.com/#view/Microsoft_Azure_ProjectOxford/CognitiveServicesHub/~/OpenAI".to_string()),
                requires_api_key: true,
                models: vec![],
            },
        ]
    }

    /// Get a provider by ID
    pub fn get_provider(&mut self, id: &str) -> Option<&ProviderInfo> {
        self.get_providers().iter().find(|p| p.id == id)
    }

    /// Get provider IDs
    pub fn provider_ids(&mut self) -> Vec<String> {
        self.get_providers().iter().map(|p| p.id.clone()).collect()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_embedded_providers() {
        let registry = ProviderRegistry::with_defaults();
        let providers = registry.embedded_providers();

        assert!(!providers.is_empty());

        // Check anthropic provider exists
        let anthropic = providers.iter().find(|p| p.id == "anthropic");
        assert!(anthropic.is_some());
        assert!(anthropic.unwrap().requires_api_key);

        // Check ollama doesn't require API key
        let ollama = providers.iter().find(|p| p.id == "ollama");
        assert!(ollama.is_some());
        assert!(!ollama.unwrap().requires_api_key);
    }

    #[test]
    fn test_get_providers() {
        let dir = tempdir().unwrap();
        let mut registry = ProviderRegistry::new(dir.path());

        let providers = registry.get_providers();
        assert!(!providers.is_empty());
    }

    #[test]
    fn test_cache_save_load() {
        let dir = tempdir().unwrap();
        let mut registry = ProviderRegistry::new(dir.path());

        // Get providers (should save to cache)
        let providers = registry.get_providers().to_vec();

        // Create new registry and load from cache
        let mut registry2 = ProviderRegistry::new(dir.path());
        let providers2 = registry2.get_providers();

        assert_eq!(providers.len(), providers2.len());
    }

    #[test]
    fn test_provider_default_model() {
        let provider = ProviderInfo {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Test provider".to_string(),
            api_base_url: "http://localhost".to_string(),
            env_var: "TEST_API_KEY".to_string(),
            help_url: None,
            requires_api_key: true,
            models: vec![
                ModelInfo {
                    id: "model1".to_string(),
                    name: "Model 1".to_string(),
                    default: false,
                    context_window: None,
                    max_output_tokens: None,
                },
                ModelInfo {
                    id: "model2".to_string(),
                    name: "Model 2".to_string(),
                    default: true,
                    context_window: None,
                    max_output_tokens: None,
                },
            ],
        };

        let default_model = provider.default_model();
        assert!(default_model.is_some());
        assert_eq!(default_model.unwrap().id, "model2");
    }

    #[test]
    fn test_get_provider_by_id() {
        let dir = tempdir().unwrap();
        let mut registry = ProviderRegistry::new(dir.path());

        let anthropic = registry.get_provider("anthropic");
        assert!(anthropic.is_some());
        assert_eq!(anthropic.unwrap().name, "Anthropic");

        let nonexistent = registry.get_provider("nonexistent");
        assert!(nonexistent.is_none());
    }
}
