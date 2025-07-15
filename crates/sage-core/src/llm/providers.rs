//! LLM provider definitions and configurations

use serde::{Deserialize, Serialize};

/// Supported LLM providers
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LLMProvider {
    /// OpenAI (GPT models)
    OpenAI,
    /// Anthropic (Claude models)
    Anthropic,
    /// Google (Gemini models)
    Google,
    /// Azure OpenAI
    Azure,
    /// OpenRouter
    OpenRouter,
    /// Doubao
    Doubao,
    /// Ollama (local models)
    Ollama,
    /// Custom provider
    Custom(String),
}

impl std::fmt::Display for LLMProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LLMProvider::OpenAI => write!(f, "openai"),
            LLMProvider::Anthropic => write!(f, "anthropic"),
            LLMProvider::Google => write!(f, "google"),
            LLMProvider::Azure => write!(f, "azure"),
            LLMProvider::OpenRouter => write!(f, "openrouter"),
            LLMProvider::Doubao => write!(f, "doubao"),
            LLMProvider::Ollama => write!(f, "ollama"),
            LLMProvider::Custom(name) => write!(f, "{}", name),
        }
    }
}

impl std::str::FromStr for LLMProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(LLMProvider::OpenAI),
            "anthropic" => Ok(LLMProvider::Anthropic),
            "google" => Ok(LLMProvider::Google),
            "azure" => Ok(LLMProvider::Azure),
            "openrouter" => Ok(LLMProvider::OpenRouter),
            "doubao" => Ok(LLMProvider::Doubao),
            "ollama" => Ok(LLMProvider::Ollama),
            _ => Ok(LLMProvider::Custom(s.to_string())),
        }
    }
}

// ProviderConfig is now defined in config::provider module

/// Model-specific parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelParameters {
    /// Model name/ID
    pub model: String,
    /// Maximum tokens to generate
    pub max_tokens: Option<u32>,
    /// Temperature (0.0 to 2.0)
    pub temperature: Option<f32>,
    /// Top-p sampling
    pub top_p: Option<f32>,
    /// Top-k sampling (for supported models)
    pub top_k: Option<u32>,
    /// Stop sequences
    pub stop: Option<Vec<String>>,
    /// Whether to enable parallel tool calls
    pub parallel_tool_calls: Option<bool>,
    /// Frequency penalty
    pub frequency_penalty: Option<f32>,
    /// Presence penalty
    pub presence_penalty: Option<f32>,
    /// Seed for deterministic generation
    pub seed: Option<u32>,
}

impl ModelParameters {
    /// Create new model parameters with just the model name
    pub fn new<S: Into<String>>(model: S) -> Self {
        Self {
            model: model.into(),
            max_tokens: None,
            temperature: None,
            top_p: None,
            top_k: None,
            stop: None,
            parallel_tool_calls: None,
            frequency_penalty: None,
            presence_penalty: None,
            seed: None,
        }
    }

    /// Set max tokens
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set top-p
    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    /// Enable parallel tool calls
    pub fn with_parallel_tool_calls(mut self, enabled: bool) -> Self {
        self.parallel_tool_calls = Some(enabled);
        self
    }
}
