//! Models API client for dynamically fetching available models from providers
//!
//! This module provides functionality to fetch model lists from various LLM providers
//! using their Models API endpoints.

use crate::error::{SageError, SageResult};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, warn};

/// Response from Anthropic Models API
#[derive(Debug, Clone, Deserialize)]
struct AnthropicModelsResponse {
    data: Vec<AnthropicModelInfo>,
    #[allow(dead_code)]
    has_more: bool,
}

/// Model info from Anthropic API
#[derive(Debug, Clone, Deserialize)]
struct AnthropicModelInfo {
    id: String,
    display_name: String,
    #[allow(dead_code)]
    created_at: String,
}

/// Response from OpenAI Models API
#[derive(Debug, Deserialize)]
struct OpenAiModelsResponse {
    data: Vec<OpenAiModelInfo>,
}

/// Model info from OpenAI API
#[derive(Debug, Deserialize)]
struct OpenAiModelInfo {
    id: String,
    #[allow(dead_code)]
    owned_by: String,
}

/// Fetched model information (normalized across providers)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchedModel {
    /// Model ID
    pub id: String,
    /// Display name
    pub name: String,
}

/// Models API client
pub struct ModelsApiClient {
    http_client: Client,
    timeout: Duration,
}

impl Default for ModelsApiClient {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelsApiClient {
    /// Create a new Models API client
    pub fn new() -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            http_client,
            timeout: Duration::from_secs(30),
        }
    }

    /// Create with custom timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Fetch models from Anthropic API
    pub async fn fetch_anthropic_models(
        &self,
        base_url: &str,
        api_key: &str,
    ) -> SageResult<Vec<FetchedModel>> {
        let url = format!("{}/v1/models", base_url.trim_end_matches('/'));

        debug!("Fetching Anthropic models from: {}", url);

        let response = self
            .http_client
            .get(&url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| SageError::llm(format!("Failed to fetch Anthropic models: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            warn!("Anthropic Models API error: {} - {}", status, error_text);
            return Err(SageError::llm(format!(
                "Anthropic Models API error ({}): {}",
                status, error_text
            )));
        }

        let api_response: AnthropicModelsResponse = response
            .json()
            .await
            .map_err(|e| SageError::llm(format!("Failed to parse response: {}", e)))?;

        let models: Vec<FetchedModel> = api_response
            .data
            .iter()
            .map(|m| FetchedModel {
                id: m.id.clone(),
                name: m.display_name.clone(),
            })
            .collect();

        debug!("Fetched {} Anthropic models", models.len());
        Ok(models)
    }

    /// Fetch models from OpenAI API
    pub async fn fetch_openai_models(
        &self,
        base_url: &str,
        api_key: &str,
    ) -> SageResult<Vec<FetchedModel>> {
        let url = format!("{}/models", base_url.trim_end_matches('/'));

        debug!("Fetching OpenAI models from: {}", url);

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(api_key)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| SageError::llm(format!("Failed to fetch OpenAI models: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(SageError::llm(format!(
                "OpenAI Models API error ({}): {}",
                status, error_text
            )));
        }

        let api_response: OpenAiModelsResponse = response
            .json()
            .await
            .map_err(|e| SageError::llm(format!("Failed to parse response: {}", e)))?;

        let models: Vec<FetchedModel> = api_response
            .data
            .into_iter()
            .filter(|m| m.id.starts_with("gpt-") || m.id.starts_with("o1") || m.id.starts_with("o3"))
            .map(|m| FetchedModel {
                name: m.id.clone(),
                id: m.id,
            })
            .collect();

        debug!("Fetched {} OpenAI models", models.len());
        Ok(models)
    }

    /// Fetch models from Ollama API
    pub async fn fetch_ollama_models(&self, base_url: &str) -> SageResult<Vec<FetchedModel>> {
        let url = format!("{}/api/tags", base_url.trim_end_matches('/'));

        debug!("Fetching Ollama models from: {}", url);

        let response = self
            .http_client
            .get(&url)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| SageError::llm(format!("Failed to fetch Ollama models: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(SageError::llm(format!(
                "Ollama API error ({}): {}",
                status, error_text
            )));
        }

        #[derive(Deserialize)]
        struct OllamaResponse {
            models: Vec<OllamaModel>,
        }

        #[derive(Deserialize)]
        struct OllamaModel {
            name: String,
        }

        let api_response: OllamaResponse = response
            .json()
            .await
            .map_err(|e| SageError::llm(format!("Failed to parse response: {}", e)))?;

        let models = api_response
            .models
            .into_iter()
            .map(|m| FetchedModel {
                name: m.name.clone(),
                id: m.name,
            })
            .collect();

        Ok(models)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = ModelsApiClient::new();
        assert_eq!(client.timeout, Duration::from_secs(30));
    }
}
