//! Model identity information for system prompts

use crate::config::model::{Config, ModelParameters};

/// Model identity information for system prompt
#[derive(Debug, Clone)]
#[allow(dead_code)] // Reserved for future system prompt customization
pub(super) struct ModelIdentity {
    pub base_model_info: String,
    pub model_name: String,
}

/// Get model identity information based on configuration
pub(super) fn get_model_identity(config: &Config) -> ModelIdentity {
    let default_provider = config.get_default_provider();
    let default_params = ModelParameters::default();
    let model_params = config
        .default_model_parameters()
        .unwrap_or(&default_params);

    match default_provider {
        "anthropic" => {
            let base_model_info = match model_params.model.as_str() {
                "claude-3-sonnet-20240229" => "The base model is Claude 3 Sonnet by Anthropic.",
                "claude-3-opus-20240229" => "The base model is Claude 3 Opus by Anthropic.",
                "claude-3-haiku-20240307" => "The base model is Claude 3 Haiku by Anthropic.",
                "claude-sonnet-4-20250514" => "The base model is Claude Sonnet 4 by Anthropic.",
                _ => "The base model is Claude by Anthropic.",
            };
            ModelIdentity {
                base_model_info: base_model_info.to_string(),
                model_name: format!("{} by Anthropic", model_params.model),
            }
        }
        "openai" => {
            let base_model_info = match model_params.model.as_str() {
                "gpt-4" => "The base model is GPT-4 by OpenAI.",
                "gpt-4-turbo" => "The base model is GPT-4 Turbo by OpenAI.",
                "gpt-3.5-turbo" => "The base model is GPT-3.5 Turbo by OpenAI.",
                _ => "The base model is GPT by OpenAI.",
            };
            ModelIdentity {
                base_model_info: base_model_info.to_string(),
                model_name: format!("{} by OpenAI", model_params.model),
            }
        }
        "google" => {
            let base_model_info = match model_params.model.as_str() {
                "gemini-2.5-pro" => "The base model is Gemini 2.5 Pro by Google.",
                "gemini-1.5-pro" => "The base model is Gemini 1.5 Pro by Google.",
                "gemini-1.0-pro" => "The base model is Gemini 1.0 Pro by Google.",
                _ => "The base model is Gemini by Google.",
            };
            ModelIdentity {
                base_model_info: base_model_info.to_string(),
                model_name: format!("{} by Google", model_params.model),
            }
        }
        _ => ModelIdentity {
            base_model_info: "The base model information is not available.".to_string(),
            model_name: model_params.model.clone(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_identity_display() {
        let identity = ModelIdentity {
            base_model_info: "Test model info".to_string(),
            model_name: "test-model".to_string(),
        };

        assert_eq!(identity.base_model_info, "Test model info");
        assert_eq!(identity.model_name, "test-model");
    }
}
