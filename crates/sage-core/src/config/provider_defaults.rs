//! Default `ModelParameters` for every shipped provider.
//!
//! `id`, `base_url`, and the default `model` are derived from the
//! canonical [`embedded_providers()`][crate::config::embedded_providers]
//! list — the single source of truth for that data. Per-provider
//! tuning (`temperature`, `top_p`, `top_k`, `parallel_tool_calls`,
//! `max_retries`, `api_version`) lives in [`provider_tuning`] below
//! because it is not part of the registry's contract.
//!
//! Closes #23 alias drift: `zhipu` and `kimi` no longer ship as
//! duplicate HashMap entries. They remain accepted as alias spellings
//! by `LlmProvider::from_str` and `validate_providers` — users who
//! configure those keys still get routed to the actual provider — but
//! the `Config::default()` shipped map now has only the canonical id
//! per provider.

use crate::config::ModelParameters;
use std::collections::HashMap;

/// Create default model providers configuration.
pub fn create_default_providers() -> HashMap<String, ModelParameters> {
    let mut providers = HashMap::new();

    // Derive id/base_url/default_model from the canonical registry.
    for info in super::embedded_providers::embedded_providers() {
        let model = info
            .default_model()
            .map(|m| m.id.clone())
            .unwrap_or_else(|| placeholder_model_for_id(&info.id));
        let base_url = if info.api_base_url.is_empty() {
            placeholder_base_url_for_id(&info.id)
        } else {
            Some(info.api_base_url.clone())
        };
        let params = provider_tuning(&info.id, model, base_url);
        providers.insert(info.id, params);
    }

    // Providers that ship in default config but aren't (yet) in
    // `embedded_providers()`. Keep their hand-maintained entries so
    // a user who picks the id from CLI / env still gets a sensible
    // shipped default.
    providers.insert("doubao".to_string(), doubao_defaults());

    providers
}

fn provider_tuning(id: &str, model: String, base_url: Option<String>) -> ModelParameters {
    let (temperature, top_p, top_k, parallel_tool_calls, max_retries, api_version): (
        f32,
        f32,
        Option<u32>,
        bool,
        u32,
        Option<&'static str>,
    ) = match id {
        "anthropic" => (0.5, 1.0, Some(0), false, 10, None),
        "openai" => (0.5, 1.0, None, true, 10, None),
        "google" => (0.5, 1.0, Some(0), false, 10, None),
        "azure" => (0.5, 1.0, None, true, 10, Some("2024-02-15-preview")),
        "openrouter" => (0.5, 1.0, None, true, 10, None),
        "ollama" => (0.5, 1.0, None, false, 3, None),
        "glm" => (0.7, 1.0, None, false, 3, Some("2023-06-01")),
        "zai" => (1.0, 0.95, None, true, 3, None),
        "moonshot" => (1.0, 0.95, None, true, 3, None),
        _ => (0.5, 1.0, None, true, 3, None),
    };

    ModelParameters {
        model,
        api_key: None,
        base_url,
        max_tokens: Some(4096),
        temperature: Some(temperature),
        top_p: Some(top_p),
        top_k,
        parallel_tool_calls: Some(parallel_tool_calls),
        max_retries: Some(max_retries),
        api_version: api_version.map(String::from),
        stop_sequences: None,
    }
}

fn placeholder_base_url_for_id(id: &str) -> Option<String> {
    match id {
        "azure" => Some("https://your-resource.openai.azure.com".to_string()),
        _ => None,
    }
}

fn placeholder_model_for_id(id: &str) -> String {
    match id {
        "azure" => "gpt-4".to_string(),
        _ => format!("{id}-default"),
    }
}

fn doubao_defaults() -> ModelParameters {
    ModelParameters {
        model: "doubao-pro-4k".to_string(),
        api_key: None,
        base_url: Some("https://ark.cn-beijing.volces.com".to_string()),
        max_tokens: Some(4096),
        temperature: Some(0.5),
        top_p: Some(1.0),
        top_k: None,
        parallel_tool_calls: Some(true),
        max_retries: Some(10),
        api_version: None,
        stop_sequences: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_cover_every_embedded_provider() {
        let defaults = create_default_providers();
        for info in crate::config::embedded_providers::embedded_providers() {
            assert!(
                defaults.contains_key(&info.id),
                "embedded provider {:?} is missing a default ModelParameters entry",
                info.id
            );
        }
    }

    #[test]
    fn defaults_do_not_drift_on_base_url_or_model() {
        // Regression for issue #23: provider_defaults previously
        // hardcoded `https://api.openai.com` while the registry said
        // `/v1`, and `anthropic/claude-3.5-sonnet` while the registry
        // had been bumped to `claude-sonnet-4`. Both now derive from
        // the registry.
        let defaults = create_default_providers();
        for info in crate::config::embedded_providers::embedded_providers() {
            let entry = defaults
                .get(&info.id)
                .unwrap_or_else(|| panic!("missing default for {:?}", info.id));
            if !info.api_base_url.is_empty() {
                assert_eq!(
                    entry.base_url.as_deref(),
                    Some(info.api_base_url.as_str()),
                    "base_url drift for {:?}",
                    info.id
                );
            }
            if let Some(default_model) = info.default_model() {
                assert_eq!(
                    entry.model, default_model.id,
                    "default model drift for {:?}",
                    info.id
                );
            }
        }
    }

    #[test]
    fn zhipu_and_kimi_aliases_are_not_shipped_as_default_entries() {
        // U-24: aliases must not appear as duplicate HashMap entries.
        let defaults = create_default_providers();
        assert!(!defaults.contains_key("zhipu"));
        assert!(!defaults.contains_key("kimi"));
        assert!(defaults.contains_key("glm"));
        assert!(defaults.contains_key("moonshot"));
    }

    #[test]
    fn doubao_default_is_shipped_even_without_registry_entry() {
        let defaults = create_default_providers();
        let doubao = defaults.get("doubao").expect("doubao default missing");
        assert_eq!(doubao.model, "doubao-pro-4k");
        assert_eq!(
            doubao.base_url.as_deref(),
            Some("https://ark.cn-beijing.volces.com")
        );
    }
}
