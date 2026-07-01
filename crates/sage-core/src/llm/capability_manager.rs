//! Unified model capability lookup.

use super::model_capabilities::{ModelCapability, get_static_model_capability};
use crate::config::{ModelInfo, ProviderCatalogSnapshot};
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct CapabilityManager {
    catalog_capabilities: HashMap<String, ModelCapability>,
}

impl CapabilityManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_catalog_snapshot(snapshot: &ProviderCatalogSnapshot) -> Self {
        let mut manager = Self::new();
        for model in &snapshot.provider.models {
            manager.insert_model(model);
        }
        manager
    }

    pub fn capability(&self, model: &str) -> ModelCapability {
        self.catalog_capabilities
            .get(model)
            .cloned()
            .unwrap_or_else(|| get_static_model_capability(model))
    }

    pub fn recommended_max_tokens(&self, model: &str) -> u32 {
        let cap = self.capability(model);
        let tokens_f64 = cap.max_output_tokens as f64 * 0.75;
        if tokens_f64.is_finite() && tokens_f64 >= 0.0 {
            (tokens_f64 as u32).min(cap.max_output_tokens)
        } else {
            cap.max_output_tokens
        }
    }

    pub fn max_output_tokens(&self, model: &str) -> u32 {
        self.capability(model).max_output_tokens
    }

    fn insert_model(&mut self, model: &ModelInfo) {
        let mut capability = ModelCapability::default();
        if let Some(max_output_tokens) = model.max_output_tokens {
            capability.max_output_tokens = max_output_tokens;
        }
        if let Some(context_window) = model.context_window {
            capability.context_window = context_window;
        }
        self.catalog_capabilities
            .insert(model.id.clone(), capability);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CatalogFreshness, CatalogSource, ProviderCatalogSnapshot, ProviderInfo};

    fn snapshot() -> ProviderCatalogSnapshot {
        ProviderCatalogSnapshot {
            provider: ProviderInfo {
                id: "test".to_string(),
                name: "Test".to_string(),
                description: "Test".to_string(),
                api_base_url: "https://example.test".to_string(),
                env_var: "TEST_API_KEY".to_string(),
                help_url: None,
                requires_api_key: true,
                models: vec![ModelInfo {
                    id: "catalog-model".to_string(),
                    name: "Catalog Model".to_string(),
                    default: true,
                    context_window: Some(321_000),
                    max_output_tokens: Some(12_345),
                }],
            },
            freshness: CatalogFreshness::Fresh,
            source: CatalogSource::Merged,
            etag: Some("etag".to_string()),
            fetched_at: Some(10),
            ttl_seconds: 60,
            last_error: None,
        }
    }

    #[test]
    fn catalog_exact_match_wins() {
        let manager = CapabilityManager::from_catalog_snapshot(&snapshot());

        let cap = manager.capability("catalog-model");

        assert_eq!(cap.context_window, 321_000);
        assert_eq!(cap.max_output_tokens, 12_345);
    }

    #[test]
    fn static_longest_prefix_still_applies() {
        let manager = CapabilityManager::new();
        let cap = manager.capability("claude-sonnet-4-6-preview");
        let registered = manager.capability("claude-sonnet-4-6");

        assert_eq!(cap.max_output_tokens, registered.max_output_tokens);
    }

    #[test]
    fn unknown_model_uses_conservative_default() {
        let manager = CapabilityManager::new();
        let cap = manager.capability("unlisted-model");
        let default = ModelCapability::default();

        assert_eq!(cap.max_output_tokens, default.max_output_tokens);
        assert_eq!(cap.context_window, default.context_window);
    }
}
