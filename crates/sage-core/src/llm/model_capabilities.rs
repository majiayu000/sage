//! Model capabilities and limits
//!
//! This module defines the capabilities and limits for various LLM models.
//! Instead of hardcoding max_tokens per provider, we define them per model.

use std::collections::HashMap;
use std::sync::LazyLock;

/// Model capability information
#[derive(Debug, Clone)]
pub struct ModelCapability {
    /// Maximum output tokens the model supports
    pub max_output_tokens: u32,
    /// Maximum context window (input + output)
    pub context_window: u32,
    /// Whether the model supports tool/function calling
    pub supports_tools: bool,
    /// Whether the model supports vision/images
    pub supports_vision: bool,
    /// Whether the model supports streaming
    pub supports_streaming: bool,
}

impl Default for ModelCapability {
    fn default() -> Self {
        Self {
            max_output_tokens: 4096,
            context_window: 128_000,
            supports_tools: true,
            supports_vision: false,
            supports_streaming: true,
        }
    }
}

/// Static map of known model capabilities
static MODEL_CAPABILITIES: LazyLock<HashMap<&'static str, ModelCapability>> = LazyLock::new(|| {
    let mut m = HashMap::new();

    // Anthropic Claude models
    m.insert(
        "claude-3-5-sonnet-20241022",
        ModelCapability {
            max_output_tokens: 8192,
            context_window: 200_000,
            supports_tools: true,
            supports_vision: true,
            supports_streaming: true,
        },
    );
    m.insert(
        "claude-sonnet-4-20250514",
        ModelCapability {
            max_output_tokens: 16384,
            context_window: 200_000,
            supports_tools: true,
            supports_vision: true,
            supports_streaming: true,
        },
    );
    m.insert(
        "claude-3-opus-20240229",
        ModelCapability {
            max_output_tokens: 4096,
            context_window: 200_000,
            supports_tools: true,
            supports_vision: true,
            supports_streaming: true,
        },
    );
    m.insert(
        "claude-3-haiku-20240307",
        ModelCapability {
            max_output_tokens: 4096,
            context_window: 200_000,
            supports_tools: true,
            supports_vision: true,
            supports_streaming: true,
        },
    );

    // OpenAI GPT models
    m.insert(
        "gpt-4",
        ModelCapability {
            max_output_tokens: 8192,
            context_window: 128_000,
            supports_tools: true,
            supports_vision: false,
            supports_streaming: true,
        },
    );
    m.insert(
        "gpt-4-turbo",
        ModelCapability {
            max_output_tokens: 4096,
            context_window: 128_000,
            supports_tools: true,
            supports_vision: true,
            supports_streaming: true,
        },
    );
    m.insert(
        "gpt-4o",
        ModelCapability {
            max_output_tokens: 16384,
            context_window: 128_000,
            supports_tools: true,
            supports_vision: true,
            supports_streaming: true,
        },
    );
    m.insert(
        "gpt-4o-mini",
        ModelCapability {
            max_output_tokens: 16384,
            context_window: 128_000,
            supports_tools: true,
            supports_vision: true,
            supports_streaming: true,
        },
    );
    m.insert(
        "o1",
        ModelCapability {
            max_output_tokens: 100_000,
            context_window: 200_000,
            supports_tools: true,
            supports_vision: true,
            supports_streaming: true,
        },
    );
    m.insert(
        "o1-mini",
        ModelCapability {
            max_output_tokens: 65_536,
            context_window: 128_000,
            supports_tools: true,
            supports_vision: false,
            supports_streaming: true,
        },
    );

    // Google Gemini models
    m.insert(
        "gemini-1.5-pro",
        ModelCapability {
            max_output_tokens: 8192,
            context_window: 2_000_000,
            supports_tools: true,
            supports_vision: true,
            supports_streaming: true,
        },
    );
    m.insert(
        "gemini-1.5-flash",
        ModelCapability {
            max_output_tokens: 8192,
            context_window: 1_000_000,
            supports_tools: true,
            supports_vision: true,
            supports_streaming: true,
        },
    );
    m.insert(
        "gemini-2.0-flash",
        ModelCapability {
            max_output_tokens: 8192,
            context_window: 1_000_000,
            supports_tools: true,
            supports_vision: true,
            supports_streaming: true,
        },
    );

    // GLM models (Zhipu)
    m.insert(
        "glm-4",
        ModelCapability {
            max_output_tokens: 4096,
            context_window: 128_000,
            supports_tools: true,
            supports_vision: false,
            supports_streaming: true,
        },
    );
    m.insert(
        "glm-4-plus",
        ModelCapability {
            max_output_tokens: 4096,
            context_window: 128_000,
            supports_tools: true,
            supports_vision: false,
            supports_streaming: true,
        },
    );

    // Doubao models
    m.insert(
        "doubao-pro-4k",
        ModelCapability {
            max_output_tokens: 4096,
            context_window: 4_000,
            supports_tools: true,
            supports_vision: false,
            supports_streaming: true,
        },
    );
    m.insert(
        "doubao-pro-32k",
        ModelCapability {
            max_output_tokens: 4096,
            context_window: 32_000,
            supports_tools: true,
            supports_vision: false,
            supports_streaming: true,
        },
    );
    m.insert(
        "doubao-pro-128k",
        ModelCapability {
            max_output_tokens: 4096,
            context_window: 128_000,
            supports_tools: true,
            supports_vision: false,
            supports_streaming: true,
        },
    );

    m
});

/// Get the capability for a specific model
///
/// Returns the known capability if the model is recognized,
/// otherwise returns a default capability.
pub fn get_model_capability(model: &str) -> ModelCapability {
    // Try exact match first
    if let Some(cap) = MODEL_CAPABILITIES.get(model) {
        return cap.clone();
    }

    // Try prefix matching for versioned models
    for (known_model, cap) in MODEL_CAPABILITIES.iter() {
        if model.starts_with(known_model) || known_model.starts_with(model) {
            return cap.clone();
        }
    }

    // Return default for unknown models
    ModelCapability::default()
}

/// Get the recommended max_tokens for a model
///
/// Returns a sensible default based on the model's capability,
/// typically half of max_output_tokens to leave room for the model.
pub fn get_recommended_max_tokens(model: &str) -> u32 {
    let cap = get_model_capability(model);
    // Use 75% of max_output_tokens as the recommended default
    let tokens_f64 = cap.max_output_tokens as f64 * 0.75;
    if tokens_f64.is_finite() && tokens_f64 >= 0.0 {
        (tokens_f64 as u32).min(cap.max_output_tokens)
    } else {
        cap.max_output_tokens
    }
}

/// Get the maximum allowed max_tokens for a model
pub fn get_max_output_tokens(model: &str) -> u32 {
    get_model_capability(model).max_output_tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_model() {
        let cap = get_model_capability("claude-sonnet-4-20250514");
        assert_eq!(cap.max_output_tokens, 16384);
        assert!(cap.supports_vision);
    }

    #[test]
    fn test_unknown_model() {
        let cap = get_model_capability("unknown-model-v1");
        assert_eq!(cap.max_output_tokens, 4096); // default
    }

    #[test]
    fn test_prefix_matching() {
        // "gpt-4-0613" should match "gpt-4"
        let cap = get_model_capability("gpt-4-0613");
        assert_eq!(cap.max_output_tokens, 8192);
    }

    #[test]
    fn test_recommended_max_tokens() {
        let tokens = get_recommended_max_tokens("claude-sonnet-4-20250514");
        assert_eq!(tokens, 12288); // 75% of 16384
    }
}
