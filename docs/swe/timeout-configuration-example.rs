// Example implementation showing how the unified timeout configuration works
// This is a reference implementation - not meant to compile as-is

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

// ============================================================================
// 1. Core Timeout Configuration Types
// ============================================================================

/// Granular timeout configuration for LLM requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    /// Total timeout for the entire request
    #[serde(default = "default_total_timeout")]
    #[serde(with = "humantime_serde")]
    pub total: Duration,

    /// Timeout for establishing connection
    #[serde(default = "default_connect_timeout")]
    #[serde(with = "humantime_serde")]
    pub connect: Duration,

    /// Timeout for reading response data
    #[serde(default = "default_read_timeout")]
    #[serde(with = "humantime_serde")]
    pub read: Duration,

    /// Timeout for streaming requests
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "humantime_serde")]
    pub streaming: Option<Duration>,

    /// Timeout for retry attempts
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "humantime_serde")]
    pub retry: Option<Duration>,
}

fn default_total_timeout() -> Duration {
    Duration::from_secs(60)
}

fn default_connect_timeout() -> Duration {
    Duration::from_secs(10)
}

fn default_read_timeout() -> Duration {
    Duration::from_secs(30)
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            total: default_total_timeout(),
            connect: default_connect_timeout(),
            read: default_read_timeout(),
            streaming: None,
            retry: None,
        }
    }
}

impl TimeoutConfig {
    pub fn from_secs(secs: u64) -> Self {
        Self::new(Duration::from_secs(secs))
    }

    pub fn new(total: Duration) -> Self {
        Self {
            total,
            ..Default::default()
        }
    }

    pub fn with_connect(mut self, timeout: Duration) -> Self {
        self.connect = timeout;
        self
    }

    pub fn with_read(mut self, timeout: Duration) -> Self {
        self.read = timeout;
        self
    }

    pub fn with_streaming(mut self, timeout: Duration) -> Self {
        self.streaming = Some(timeout);
        self
    }

    pub fn with_retry(mut self, timeout: Duration) -> Self {
        self.retry = Some(timeout);
        self
    }

    pub fn effective_streaming(&self) -> Duration {
        self.streaming.unwrap_or(self.total)
    }

    pub fn effective_retry(&self) -> Duration {
        self.retry.unwrap_or(self.total)
    }
}

// ============================================================================
// 2. Provider-Specific Defaults
// ============================================================================

pub struct TimeoutDefaults;

impl TimeoutDefaults {
    pub fn openai() -> TimeoutConfig {
        TimeoutConfig {
            total: Duration::from_secs(60),
            connect: Duration::from_secs(10),
            read: Duration::from_secs(30),
            streaming: Some(Duration::from_secs(120)),
            retry: Some(Duration::from_secs(45)),
        }
    }

    pub fn anthropic() -> TimeoutConfig {
        TimeoutConfig {
            total: Duration::from_secs(60),
            connect: Duration::from_secs(10),
            read: Duration::from_secs(30),
            streaming: Some(Duration::from_secs(120)),
            retry: Some(Duration::from_secs(45)),
        }
    }

    pub fn google() -> TimeoutConfig {
        TimeoutConfig {
            total: Duration::from_secs(90),
            connect: Duration::from_secs(15),
            read: Duration::from_secs(45),
            streaming: Some(Duration::from_secs(180)),
            retry: Some(Duration::from_secs(60)),
        }
    }

    pub fn ollama() -> TimeoutConfig {
        TimeoutConfig {
            total: Duration::from_secs(300),
            connect: Duration::from_secs(5),
            read: Duration::from_secs(60),
            streaming: Some(Duration::from_secs(600)),
            retry: Some(Duration::from_secs(180)),
        }
    }

    pub fn for_provider(provider: &str) -> TimeoutConfig {
        match provider {
            "openai" => Self::openai(),
            "anthropic" => Self::anthropic(),
            "google" => Self::google(),
            "ollama" => Self::ollama(),
            _ => TimeoutConfig::default(),
        }
    }
}

// ============================================================================
// 3. Request Options (for runtime overrides)
// ============================================================================

#[derive(Debug, Clone, Default)]
pub struct RequestOptions {
    /// Override timeout for this specific request
    pub timeout_override: Option<TimeoutConfig>,

    /// Request priority
    pub priority: Option<u32>,

    /// Custom metadata
    pub metadata: HashMap<String, String>,
}

impl RequestOptions {
    pub fn with_timeout(timeout: TimeoutConfig) -> Self {
        Self {
            timeout_override: Some(timeout),
            ..Default::default()
        }
    }

    pub fn with_total_timeout(duration: Duration) -> Self {
        Self::with_timeout(TimeoutConfig::new(duration))
    }
}

// ============================================================================
// 4. Updated ProviderConfig
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,

    /// DEPRECATED: Use timeouts.total instead
    #[serde(skip_serializing_if = "Option::is_none")]
    #[deprecated(since = "0.2.0", note = "Use timeouts field instead")]
    pub timeout: Option<u64>,

    /// Granular timeout configuration
    #[serde(default)]
    pub timeouts: Option<TimeoutConfig>,

    pub max_retries: Option<u32>,
}

impl ProviderConfig {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            api_key: None,
            base_url: None,
            timeout: None,
            timeouts: None,
            max_retries: Some(3),
        }
    }

    /// Get effective timeout configuration
    /// Priority: explicit timeouts > legacy timeout > provider defaults
    pub fn get_timeouts(&self) -> TimeoutConfig {
        if let Some(ref timeouts) = self.timeouts {
            return timeouts.clone();
        }

        #[allow(deprecated)]
        if let Some(timeout_secs) = self.timeout {
            return TimeoutConfig::from_secs(timeout_secs);
        }

        TimeoutDefaults::for_provider(&self.name)
    }

    pub fn with_timeouts(mut self, timeouts: TimeoutConfig) -> Self {
        self.timeouts = Some(timeouts);
        self
    }

    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }
}

// ============================================================================
// 5. Usage Examples
// ============================================================================

#[cfg(test)]
mod examples {
    use super::*;

    /// Example 1: Using provider defaults (no configuration needed)
    fn example_provider_defaults() {
        let config = ProviderConfig::new("anthropic")
            .with_api_key("sk-ant-...");

        let timeouts = config.get_timeouts();

        // Will use Anthropic defaults:
        // - total: 60s
        // - connect: 10s
        // - read: 30s
        // - streaming: 120s
        // - retry: 45s

        println!("Anthropic default total timeout: {:?}", timeouts.total);
    }

    /// Example 2: Custom provider-level timeouts
    fn example_custom_provider_timeouts() {
        let custom_timeouts = TimeoutConfig::default()
            .with_connect(Duration::from_secs(15))
            .with_streaming(Duration::from_secs(180));

        let config = ProviderConfig::new("anthropic")
            .with_api_key("sk-ant-...")
            .with_timeouts(custom_timeouts);

        let timeouts = config.get_timeouts();
        println!("Custom streaming timeout: {:?}", timeouts.streaming);
    }

    /// Example 3: Request-level override
    async fn example_request_override() {
        // Simulated client
        struct Client {
            config: ProviderConfig,
        }

        impl Client {
            async fn chat_with_options(
                &self,
                messages: Vec<String>,
                options: Option<RequestOptions>,
            ) -> Result<String, String> {
                let effective_timeout = if let Some(ref opts) = options {
                    if let Some(ref override_timeout) = opts.timeout_override {
                        override_timeout.clone()
                    } else {
                        self.config.get_timeouts()
                    }
                } else {
                    self.config.get_timeouts()
                };

                println!("Using timeout: {:?}", effective_timeout.total);
                Ok("response".to_string())
            }
        }

        let client = Client {
            config: ProviderConfig::new("anthropic"),
        };

        // Normal request - uses provider timeout (60s)
        let _ = client.chat_with_options(
            vec!["Hello".to_string()],
            None,
        ).await;

        // Complex reasoning - extend timeout to 5 minutes
        let options = RequestOptions::with_total_timeout(Duration::from_secs(300));
        let _ = client.chat_with_options(
            vec!["Analyze this complex problem...".to_string()],
            Some(options),
        ).await;

        // Quick query - reduce timeout to 15 seconds
        let options = RequestOptions::with_total_timeout(Duration::from_secs(15));
        let _ = client.chat_with_options(
            vec!["What is 2+2?".to_string()],
            Some(options),
        ).await;
    }

    /// Example 4: Configuration file format
    fn example_config_file_format() {
        // This is what the JSON config would look like:
        let json_example = r#"
{
  "model_providers": {
    "anthropic": {
      "api_key": "${ANTHROPIC_API_KEY}",
      "model": "claude-3-sonnet-20240229",
      "timeouts": {
        "total": "60s",
        "connect": "10s",
        "read": "30s",
        "streaming": "2m",
        "retry": "45s"
      }
    },

    "google": {
      "api_key": "${GOOGLE_API_KEY}",
      "model": "gemini-2.5-pro",
      "timeouts": {
        "total": "90s",
        "streaming": "3m"
      }
    },

    "ollama": {
      "base_url": "http://localhost:11434",
      "model": "llama3",
      "timeouts": {
        "total": "5m",
        "streaming": "10m"
      }
    }
  }
}
"#;

        println!("{}", json_example);
    }

    /// Example 5: Different timeout strategies for different scenarios
    fn example_timeout_strategies() {
        // Strategy 1: Fast responses for simple queries
        let quick_timeout = TimeoutConfig::from_secs(15);

        // Strategy 2: Standard timeout for normal operations
        let standard_timeout = TimeoutConfig::from_secs(60);

        // Strategy 3: Extended timeout for complex reasoning
        let extended_timeout = TimeoutConfig::from_secs(300);

        // Strategy 4: Very long timeout for code generation
        let code_gen_timeout = TimeoutConfig::from_secs(600)
            .with_streaming(Duration::from_secs(1200));

        println!("Quick: {:?}", quick_timeout.total);
        println!("Standard: {:?}", standard_timeout.total);
        println!("Extended: {:?}", extended_timeout.total);
        println!("Code gen: {:?}", code_gen_timeout.total);
    }

    /// Example 6: Migrating from legacy timeout field
    fn example_legacy_migration() {
        // OLD WAY (deprecated but still works):
        #[allow(deprecated)]
        let old_config = ProviderConfig {
            name: "openai".to_string(),
            api_key: Some("sk-...".to_string()),
            base_url: None,
            timeout: Some(90), // Legacy field
            timeouts: None,
            max_retries: Some(3),
        };

        // Still works - converts to TimeoutConfig automatically
        let timeouts = old_config.get_timeouts();
        assert_eq!(timeouts.total, Duration::from_secs(90));

        // NEW WAY (recommended):
        let new_config = ProviderConfig::new("openai")
            .with_api_key("sk-...")
            .with_timeouts(
                TimeoutConfig::from_secs(90)
                    .with_streaming(Duration::from_secs(180))
            );

        let timeouts = new_config.get_timeouts();
        assert_eq!(timeouts.total, Duration::from_secs(90));
        assert_eq!(timeouts.streaming, Some(Duration::from_secs(180)));
    }

    /// Example 7: Provider-specific optimizations
    fn example_provider_optimizations() {
        // OpenAI - fast, reliable API
        let openai_timeouts = TimeoutDefaults::openai();
        assert_eq!(openai_timeouts.total, Duration::from_secs(60));

        // Google - may need more time for large context
        let google_timeouts = TimeoutDefaults::google();
        assert_eq!(google_timeouts.total, Duration::from_secs(90));

        // Ollama - local model, much slower inference
        let ollama_timeouts = TimeoutDefaults::ollama();
        assert_eq!(ollama_timeouts.total, Duration::from_secs(300));

        // Different streaming timeouts
        assert_eq!(openai_timeouts.streaming, Some(Duration::from_secs(120)));
        assert_eq!(google_timeouts.streaming, Some(Duration::from_secs(180)));
        assert_eq!(ollama_timeouts.streaming, Some(Duration::from_secs(600)));
    }

    /// Example 8: Dynamic timeout adjustment based on context
    fn example_dynamic_timeouts() {
        fn choose_timeout(message_length: usize, has_tools: bool) -> TimeoutConfig {
            match (message_length, has_tools) {
                // Very short query, no tools
                (0..=100, false) => TimeoutConfig::from_secs(15),

                // Normal query, no tools
                (101..=1000, false) => TimeoutConfig::from_secs(60),

                // Long context or has tools
                (_, true) | (1001.., false) => TimeoutConfig::from_secs(120)
                    .with_streaming(Duration::from_secs(240)),

                _ => TimeoutConfig::default(),
            }
        }

        // Short query
        let timeout1 = choose_timeout(50, false);
        assert_eq!(timeout1.total, Duration::from_secs(15));

        // Normal query
        let timeout2 = choose_timeout(500, false);
        assert_eq!(timeout2.total, Duration::from_secs(60));

        // Complex query with tools
        let timeout3 = choose_timeout(2000, true);
        assert_eq!(timeout3.total, Duration::from_secs(120));
    }
}

// ============================================================================
// 6. Configuration Resolution Flow
// ============================================================================

/// This shows the complete timeout resolution flow:
///
/// 1. Request options override (if provided)
///    └─> Highest priority
///
/// 2. Provider config timeouts (if configured)
///    └─> User's explicit configuration
///
/// 3. Legacy timeout field (if set)
///    └─> Backward compatibility
///
/// 4. Provider defaults
///    └─> Sensible defaults per provider
///
/// 5. Global defaults
///    └─> Last resort fallback
///
pub fn resolve_timeout(
    provider_config: &ProviderConfig,
    request_options: Option<&RequestOptions>,
) -> TimeoutConfig {
    // 1. Check request-level override
    if let Some(opts) = request_options {
        if let Some(ref timeout_override) = opts.timeout_override {
            return timeout_override.clone();
        }
    }

    // 2-4. Provider config handles: explicit timeouts, legacy timeout, provider defaults
    provider_config.get_timeouts()

    // 5. Global defaults are handled by TimeoutConfig::default() if nothing else applies
}

// ============================================================================
// 7. HTTP Client Integration Example
// ============================================================================

/// Shows how to apply timeout config to reqwest HTTP client
fn create_http_client_with_timeouts(config: &ProviderConfig) -> Result<reqwest::Client, String> {
    let timeouts = config.get_timeouts();

    let client = reqwest::Client::builder()
        // Connection timeout - time to establish TCP + TLS
        .connect_timeout(timeouts.connect)

        // Total request timeout - from start to finish
        .timeout(timeouts.total)

        // Read timeout - time between receiving bytes
        .read_timeout(timeouts.read)

        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    Ok(client)
}

// For streaming requests, we would override the timeout:
async fn make_streaming_request(
    client: &reqwest::Client,
    config: &ProviderConfig,
) -> Result<String, String> {
    let timeouts = config.get_timeouts();
    let streaming_timeout = timeouts.effective_streaming();

    // Clone client with streaming timeout
    let streaming_client = reqwest::Client::builder()
        .timeout(streaming_timeout)
        .build()
        .map_err(|e| format!("Failed to create streaming client: {}", e))?;

    // Make streaming request...
    Ok("response".to_string())
}
