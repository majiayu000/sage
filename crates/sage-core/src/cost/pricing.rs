//! Model pricing definitions
//!
//! This module defines pricing for various LLM providers and models.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Price per 1M tokens
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TokenPrice {
    /// Price per 1M input tokens (USD)
    pub input: f64,
    /// Price per 1M output tokens (USD)
    pub output: f64,
}

impl TokenPrice {
    /// Create new token price
    pub const fn new(input: f64, output: f64) -> Self {
        Self { input, output }
    }

    /// Calculate cost for given token counts
    pub fn calculate(&self, input_tokens: usize, output_tokens: usize) -> f64 {
        let input_cost = (input_tokens as f64 / 1_000_000.0) * self.input;
        let output_cost = (output_tokens as f64 / 1_000_000.0) * self.output;
        input_cost + output_cost
    }
}

/// Model pricing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    /// Model identifier
    pub model_id: String,
    /// Provider name
    pub provider: String,
    /// Display name
    pub display_name: String,
    /// Token pricing
    pub price: TokenPrice,
    /// Context window size
    pub context_window: usize,
    /// Maximum output tokens
    pub max_output: usize,
}

impl ModelPricing {
    /// Create new model pricing
    pub fn new(
        model_id: impl Into<String>,
        provider: impl Into<String>,
        price: TokenPrice,
    ) -> Self {
        let model_id = model_id.into();
        Self {
            display_name: model_id.clone(),
            model_id,
            provider: provider.into(),
            price,
            context_window: 128_000,
            max_output: 4096,
        }
    }

    /// Set display name
    pub fn with_display_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = name.into();
        self
    }

    /// Set context window
    pub fn with_context_window(mut self, size: usize) -> Self {
        self.context_window = size;
        self
    }

    /// Set max output
    pub fn with_max_output(mut self, max: usize) -> Self {
        self.max_output = max;
        self
    }

    /// Calculate cost for given usage
    pub fn calculate_cost(&self, input_tokens: usize, output_tokens: usize) -> f64 {
        self.price.calculate(input_tokens, output_tokens)
    }
}

/// Pricing registry for all known models
#[derive(Debug, Clone, Default)]
pub struct PricingRegistry {
    /// Model pricing by model ID
    models: HashMap<String, ModelPricing>,
    /// Aliases for model IDs
    aliases: HashMap<String, String>,
}

impl PricingRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Create registry with default pricing
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register_defaults();
        registry
    }

    /// Register a model
    pub fn register(&mut self, pricing: ModelPricing) {
        self.models.insert(pricing.model_id.clone(), pricing);
    }

    /// Register an alias
    pub fn register_alias(&mut self, alias: impl Into<String>, model_id: impl Into<String>) {
        self.aliases.insert(alias.into(), model_id.into());
    }

    /// Get pricing for a model
    pub fn get(&self, model_id: &str) -> Option<&ModelPricing> {
        // Check direct match
        if let Some(pricing) = self.models.get(model_id) {
            return Some(pricing);
        }

        // Check aliases
        if let Some(actual_id) = self.aliases.get(model_id) {
            return self.models.get(actual_id);
        }

        // Try partial match
        self.models
            .values()
            .find(|p| model_id.contains(&p.model_id) || p.model_id.contains(model_id))
    }

    /// Calculate cost for a model
    pub fn calculate_cost(
        &self,
        model_id: &str,
        input_tokens: usize,
        output_tokens: usize,
    ) -> Option<f64> {
        self.get(model_id)
            .map(|p| p.calculate_cost(input_tokens, output_tokens))
    }

    /// List all models (returns iterator to avoid allocation)
    pub fn list_models(&self) -> impl Iterator<Item = &ModelPricing> {
        self.models.values()
    }

    /// List models by provider (returns iterator to avoid allocation)
    pub fn list_by_provider<'a>(
        &'a self,
        provider: &'a str,
    ) -> impl Iterator<Item = &'a ModelPricing> {
        self.models
            .values()
            .filter(move |p| p.provider.eq_ignore_ascii_case(provider))
    }

    /// Register default model pricing (as of late 2024)
    fn register_defaults(&mut self) {
        // Anthropic models
        self.register(
            ModelPricing::new(
                "claude-3-5-sonnet-20241022",
                "anthropic",
                TokenPrice::new(3.0, 15.0),
            )
            .with_display_name("Claude 3.5 Sonnet")
            .with_context_window(200_000)
            .with_max_output(8192),
        );
        self.register_alias("claude-3-5-sonnet", "claude-3-5-sonnet-20241022");
        self.register_alias("sonnet", "claude-3-5-sonnet-20241022");

        self.register(
            ModelPricing::new(
                "claude-3-5-haiku-20241022",
                "anthropic",
                TokenPrice::new(0.80, 4.0),
            )
            .with_display_name("Claude 3.5 Haiku")
            .with_context_window(200_000)
            .with_max_output(8192),
        );
        self.register_alias("claude-3-5-haiku", "claude-3-5-haiku-20241022");
        self.register_alias("haiku", "claude-3-5-haiku-20241022");

        self.register(
            ModelPricing::new(
                "claude-3-opus-20240229",
                "anthropic",
                TokenPrice::new(15.0, 75.0),
            )
            .with_display_name("Claude 3 Opus")
            .with_context_window(200_000)
            .with_max_output(4096),
        );
        self.register_alias("claude-3-opus", "claude-3-opus-20240229");
        self.register_alias("opus", "claude-3-opus-20240229");

        self.register(
            ModelPricing::new(
                "claude-opus-4-5-20251101",
                "anthropic",
                TokenPrice::new(15.0, 75.0),
            )
            .with_display_name("Claude Opus 4.5")
            .with_context_window(200_000)
            .with_max_output(16384),
        );

        // OpenAI models
        self.register(
            ModelPricing::new("gpt-4-turbo", "openai", TokenPrice::new(10.0, 30.0))
                .with_display_name("GPT-4 Turbo")
                .with_context_window(128_000)
                .with_max_output(4096),
        );

        self.register(
            ModelPricing::new("gpt-4o", "openai", TokenPrice::new(2.50, 10.0))
                .with_display_name("GPT-4o")
                .with_context_window(128_000)
                .with_max_output(16384),
        );

        self.register(
            ModelPricing::new("gpt-4o-mini", "openai", TokenPrice::new(0.15, 0.60))
                .with_display_name("GPT-4o Mini")
                .with_context_window(128_000)
                .with_max_output(16384),
        );

        self.register(
            ModelPricing::new("o1-preview", "openai", TokenPrice::new(15.0, 60.0))
                .with_display_name("o1 Preview")
                .with_context_window(128_000)
                .with_max_output(32768),
        );

        self.register(
            ModelPricing::new("o1-mini", "openai", TokenPrice::new(3.0, 12.0))
                .with_display_name("o1 Mini")
                .with_context_window(128_000)
                .with_max_output(65536),
        );

        // Google models
        self.register(
            ModelPricing::new(
                "gemini-1.5-pro",
                "google",
                TokenPrice::new(1.25, 5.0), // Up to 128K context
            )
            .with_display_name("Gemini 1.5 Pro")
            .with_context_window(1_000_000)
            .with_max_output(8192),
        );

        self.register(
            ModelPricing::new("gemini-1.5-flash", "google", TokenPrice::new(0.075, 0.30))
                .with_display_name("Gemini 1.5 Flash")
                .with_context_window(1_000_000)
                .with_max_output(8192),
        );

        self.register(
            ModelPricing::new("gemini-2.0-flash", "google", TokenPrice::new(0.10, 0.40))
                .with_display_name("Gemini 2.0 Flash")
                .with_context_window(1_000_000)
                .with_max_output(8192),
        );

        // DeepSeek models
        self.register(
            ModelPricing::new("deepseek-chat", "deepseek", TokenPrice::new(0.14, 0.28))
                .with_display_name("DeepSeek Chat")
                .with_context_window(64_000)
                .with_max_output(4096),
        );

        self.register(
            ModelPricing::new("deepseek-coder", "deepseek", TokenPrice::new(0.14, 0.28))
                .with_display_name("DeepSeek Coder")
                .with_context_window(64_000)
                .with_max_output(4096),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_price_calculate() {
        let price = TokenPrice::new(3.0, 15.0);
        let cost = price.calculate(1_000_000, 1_000_000);
        assert!((cost - 18.0).abs() < 0.001);
    }

    #[test]
    fn test_token_price_small_usage() {
        let price = TokenPrice::new(3.0, 15.0);
        let cost = price.calculate(1000, 500);
        assert!((cost - 0.0105).abs() < 0.0001);
    }

    #[test]
    fn test_model_pricing() {
        let pricing = ModelPricing::new("test-model", "test", TokenPrice::new(1.0, 2.0))
            .with_display_name("Test Model")
            .with_context_window(100_000);

        assert_eq!(pricing.model_id, "test-model");
        assert_eq!(pricing.display_name, "Test Model");
        assert_eq!(pricing.context_window, 100_000);
    }

    #[test]
    fn test_pricing_registry_defaults() {
        let registry = PricingRegistry::with_defaults();

        assert!(registry.get("claude-3-5-sonnet-20241022").is_some());
        assert!(registry.get("gpt-4o").is_some());
        assert!(registry.get("gemini-1.5-pro").is_some());
    }

    #[test]
    fn test_pricing_registry_aliases() {
        let registry = PricingRegistry::with_defaults();

        // Alias should resolve to full model ID
        let sonnet = registry.get("sonnet");
        assert!(sonnet.is_some());
        assert!(sonnet.unwrap().model_id.contains("sonnet"));
    }

    #[test]
    fn test_pricing_registry_calculate() {
        let registry = PricingRegistry::with_defaults();

        let cost = registry.calculate_cost("gpt-4o", 10_000, 5_000);
        assert!(cost.is_some());
        assert!(cost.unwrap() > 0.0);
    }

    #[test]
    fn test_list_by_provider() {
        let registry = PricingRegistry::with_defaults();

        let anthropic: Vec<_> = registry.list_by_provider("anthropic").collect();
        assert!(!anthropic.is_empty());
        assert!(anthropic.iter().all(|p| p.provider == "anthropic"));

        let openai_count = registry.list_by_provider("openai").count();
        assert!(openai_count > 0);
    }

    #[test]
    fn test_partial_match() {
        let registry = PricingRegistry::with_defaults();

        // Should find via partial match
        let result = registry.get("claude-3-5-sonnet");
        assert!(result.is_some());
    }

    #[test]
    fn test_unknown_model() {
        let registry = PricingRegistry::with_defaults();
        assert!(registry.get("unknown-model-xyz").is_none());
    }
}
