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
    m.insert(
        "claude-opus-4-7",
        ModelCapability {
            max_output_tokens: 128_000,
            context_window: 1_000_000,
            supports_tools: true,
            supports_vision: true,
            supports_streaming: true,
        },
    );
    m.insert(
        "claude-sonnet-4-6",
        ModelCapability {
            max_output_tokens: 64_000,
            context_window: 1_000_000,
            supports_tools: true,
            supports_vision: true,
            supports_streaming: true,
        },
    );
    m.insert(
        "claude-haiku-4-5",
        ModelCapability {
            max_output_tokens: 64_000,
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
    m.insert(
        "gpt-5.4",
        ModelCapability {
            max_output_tokens: 128_000,
            context_window: 1_050_000,
            supports_tools: true,
            supports_vision: true,
            supports_streaming: true,
        },
    );
    m.insert(
        "gpt-5.4-mini",
        ModelCapability {
            max_output_tokens: 128_000,
            context_window: 400_000,
            supports_tools: true,
            supports_vision: true,
            supports_streaming: true,
        },
    );
    m.insert(
        "gpt-5.4-nano",
        ModelCapability {
            max_output_tokens: 128_000,
            context_window: 400_000,
            supports_tools: true,
            supports_vision: true,
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
    m.insert(
        "gemini-2.5-pro",
        ModelCapability {
            max_output_tokens: 65_536,
            context_window: 1_048_576,
            supports_tools: true,
            supports_vision: true,
            supports_streaming: true,
        },
    );
    m.insert(
        "gemini-2.5-flash",
        ModelCapability {
            max_output_tokens: 65_536,
            context_window: 1_048_576,
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
    m.insert(
        "glm-4.7",
        ModelCapability {
            max_output_tokens: 131_072,
            context_window: 204_800,
            supports_tools: true,
            supports_vision: false,
            supports_streaming: true,
        },
    );
    m.insert(
        "glm-5.1",
        ModelCapability {
            max_output_tokens: 131_072,
            context_window: 204_800,
            supports_tools: true,
            supports_vision: false,
            supports_streaming: true,
        },
    );
    m.insert(
        "kimi-k2.6",
        ModelCapability {
            max_output_tokens: 32_768,
            context_window: 256_000,
            supports_tools: true,
            supports_vision: true,
            supports_streaming: true,
        },
    );
    m.insert(
        "kimi-k2.5",
        ModelCapability {
            max_output_tokens: 32_768,
            context_window: 256_000,
            supports_tools: true,
            supports_vision: true,
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

/// Get the capability for a specific model.
///
/// Resolution order:
///
/// 1. Exact match on the registered key.
/// 2. Longest registered prefix of `model`. For example, given the
///    registered keys `"claude"` (hypothetical family default) and
///    `"claude-sonnet-4-6"`, the input `"claude-sonnet-4-6-preview"`
///    resolves to the more specific `"claude-sonnet-4-6"` entry.
/// 3. `ModelCapability::default()` for unknown models.
///
/// Picking the *longest* prefix is what makes the lookup deterministic.
/// Iterating `MODEL_CAPABILITIES` in `HashMap` order would otherwise
/// return whichever matching key the iterator visited first, which is
/// randomized per process and can flip the result between runs.
pub(crate) fn get_static_model_capability(model: &str) -> ModelCapability {
    // Try exact match first.
    if let Some(cap) = MODEL_CAPABILITIES.get(model) {
        return cap.clone();
    }

    // Fall back to the longest registered prefix of `model`.
    let mut best: Option<(&&str, &ModelCapability)> = None;
    for (known_model, cap) in MODEL_CAPABILITIES.iter() {
        if model.starts_with(*known_model) && best.is_none_or(|(b, _)| known_model.len() > b.len())
        {
            best = Some((known_model, cap));
        }
    }

    if let Some((_, cap)) = best {
        return cap.clone();
    }

    ModelCapability::default()
}

pub fn get_model_capability(model: &str) -> ModelCapability {
    crate::llm::CapabilityManager::default().capability(model)
}

/// Get the recommended max_tokens for a model
///
/// Returns a sensible default based on the model's capability,
/// typically half of max_output_tokens to leave room for the model.
pub fn get_recommended_max_tokens(model: &str) -> u32 {
    crate::llm::CapabilityManager::default().recommended_max_tokens(model)
}

/// Get the maximum allowed max_tokens for a model
pub fn get_max_output_tokens(model: &str) -> u32 {
    crate::llm::CapabilityManager::default().max_output_tokens(model)
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

    #[test]
    fn test_longest_prefix_wins() {
        // `claude-sonnet-4-6-preview` is longer than any registered key, so the
        // lookup must pick the most-specific registered prefix.
        // `claude-sonnet-4-6` is registered, `claude-sonnet-4` is not, and
        // older keys like `claude-3-5-sonnet-20241022` are not prefixes here.
        // The most-specific match must be `claude-sonnet-4-6`.
        let cap = get_model_capability("claude-sonnet-4-6-preview");
        let registered = get_model_capability("claude-sonnet-4-6");
        assert_eq!(cap.max_output_tokens, registered.max_output_tokens);
        assert_eq!(cap.context_window, registered.context_window);
    }

    #[test]
    fn test_short_input_does_not_match_longer_registered_keys() {
        // Before the fix, `get_model_capability("gpt")` matched both
        // `"gpt-4"` and `"gpt-5.4"` via the bidirectional `starts_with`
        // and returned whichever the HashMap iterator visited first.
        // After the fix, `"gpt"` is not itself a registered key and is
        // also not a prefix of any *registered* key in the prefix
        // direction we care about (we now only allow `model.starts_with(known)`),
        // so it must fall through to the default capability deterministically.
        let cap = get_model_capability("gpt");
        let default_cap = ModelCapability::default();
        assert_eq!(cap.max_output_tokens, default_cap.max_output_tokens);
        assert_eq!(cap.context_window, default_cap.context_window);
    }

    #[test]
    fn test_lookup_is_deterministic_across_calls() {
        // Sanity check: regardless of HashMap iteration order, the same
        // input always returns the same capability.
        let inputs = [
            "gpt-4-0613",
            "gpt-4o-2024-08-06",
            "claude-3-5-sonnet-20241022",
            "claude-sonnet-4-6-preview",
            "gemini-2.5-pro-exp",
            "definitely-not-a-real-model",
        ];
        for input in inputs {
            let first = get_model_capability(input);
            for _ in 0..16 {
                let again = get_model_capability(input);
                assert_eq!(first.max_output_tokens, again.max_output_tokens);
                assert_eq!(first.context_window, again.context_window);
                assert_eq!(first.supports_tools, again.supports_tools);
                assert_eq!(first.supports_vision, again.supports_vision);
                assert_eq!(first.supports_streaming, again.supports_streaming);
            }
        }
    }
}
