# Timeout Configuration Implementation Guide

## Overview

This guide provides step-by-step instructions for implementing the unified timeout configuration system in the Sage LLM client.

## Prerequisites

- Rust 2024 edition
- Familiarity with Sage codebase structure
- Understanding of HTTP timeouts (connect, read, total)

## Implementation Steps

### Phase 1: Core Infrastructure (Week 1)

#### Step 1.1: Add Dependencies

**File:** `crates/sage-core/Cargo.toml`

```toml
[dependencies]
# Existing dependencies...
humantime-serde = "1.1"  # For human-readable duration parsing (e.g., "60s", "2m")
```

**Why:** Allows users to write "60s" instead of numeric seconds in config files.

#### Step 1.2: Create Timeout Module

**File:** `crates/sage-core/src/llm/timeout.rs`

```rust
//! Timeout configuration for LLM requests
//!
//! This module provides granular timeout configuration supporting:
//! - Connection timeouts (TCP + TLS handshake)
//! - Read timeouts (between receiving data)
//! - Total request timeouts
//! - Streaming-specific timeouts
//! - Retry attempt timeouts

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Granular timeout configuration for LLM requests
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimeoutConfig {
    // ... (copy from design doc)
}

impl Default for TimeoutConfig {
    // ... (copy from design doc)
}

impl TimeoutConfig {
    // ... (copy builder methods from design doc)
}

/// Provider-specific timeout defaults
pub struct TimeoutDefaults;

impl TimeoutDefaults {
    // ... (copy provider defaults from design doc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_timeout_config() {
        let config = TimeoutConfig::default();
        assert_eq!(config.total, Duration::from_secs(60));
        assert_eq!(config.connect, Duration::from_secs(10));
        assert_eq!(config.read, Duration::from_secs(30));
    }

    #[test]
    fn test_provider_defaults() {
        let openai = TimeoutDefaults::openai();
        assert_eq!(openai.total, Duration::from_secs(60));

        let ollama = TimeoutDefaults::ollama();
        assert_eq!(ollama.total, Duration::from_secs(300));
    }

    #[test]
    fn test_builder_pattern() {
        let config = TimeoutConfig::from_secs(90)
            .with_connect(Duration::from_secs(15))
            .with_streaming(Duration::from_secs(180));

        assert_eq!(config.total, Duration::from_secs(90));
        assert_eq!(config.connect, Duration::from_secs(15));
        assert_eq!(config.streaming, Some(Duration::from_secs(180)));
    }

    #[test]
    fn test_effective_timeouts() {
        let config = TimeoutConfig::default();

        // No streaming timeout set - should use total
        assert_eq!(config.effective_streaming(), config.total);

        let config_with_streaming = config
            .with_streaming(Duration::from_secs(120));
        assert_eq!(config_with_streaming.effective_streaming(), Duration::from_secs(120));
    }

    #[test]
    fn test_merge() {
        let base = TimeoutConfig::from_secs(60);
        let override_config = TimeoutConfig::from_secs(90)
            .with_streaming(Duration::from_secs(180));

        let merged = base.merge(&override_config);
        assert_eq!(merged.total, Duration::from_secs(90));
        assert_eq!(merged.streaming, Some(Duration::from_secs(180)));
    }
}
```

**Testing:**
```bash
cargo test -p sage-core llm::timeout
```

#### Step 1.3: Create Request Options Module

**File:** `crates/sage-core/src/llm/request.rs`

```rust
//! Request-level options for LLM calls
//!
//! Provides fine-grained control over individual requests,
//! including timeout overrides, priority, and metadata.

use crate::llm::timeout::TimeoutConfig;
use std::collections::HashMap;
use std::time::Duration;

/// Options for individual LLM requests
#[derive(Debug, Clone, Default)]
pub struct RequestOptions {
    /// Override timeout for this specific request
    pub timeout_override: Option<TimeoutConfig>,

    /// Request priority (for rate limiting)
    pub priority: Option<u32>,

    /// Custom request metadata
    pub metadata: HashMap<String, String>,
}

impl RequestOptions {
    /// Create new request options
    pub fn new() -> Self {
        Self::default()
    }

    /// Create request options with timeout override
    pub fn with_timeout(timeout: TimeoutConfig) -> Self {
        Self {
            timeout_override: Some(timeout),
            priority: None,
            metadata: HashMap::new(),
        }
    }

    /// Create request options with just total timeout override
    pub fn with_total_timeout(duration: Duration) -> Self {
        Self::with_timeout(TimeoutConfig::new(duration))
    }

    /// Add custom metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = Some(priority);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_options_creation() {
        let opts = RequestOptions::new();
        assert!(opts.timeout_override.is_none());

        let opts = RequestOptions::with_total_timeout(Duration::from_secs(120));
        assert!(opts.timeout_override.is_some());
        assert_eq!(
            opts.timeout_override.unwrap().total,
            Duration::from_secs(120)
        );
    }

    #[test]
    fn test_builder_pattern() {
        let opts = RequestOptions::new()
            .with_priority(1)
            .with_metadata("request_type", "complex_reasoning");

        assert_eq!(opts.priority, Some(1));
        assert_eq!(opts.metadata.get("request_type"), Some(&"complex_reasoning".to_string()));
    }
}
```

**Testing:**
```bash
cargo test -p sage-core llm::request
```

#### Step 1.4: Update Module Exports

**File:** `crates/sage-core/src/llm/mod.rs`

```rust
//! LLM client and message types

pub mod client;
pub mod converters;
pub mod fallback;
pub mod messages;
pub mod parsers;
pub mod providers;
pub mod rate_limiter;
pub mod request;      // NEW
pub mod sse_decoder;
pub mod streaming;
pub mod timeout;      // NEW

pub use client::LLMClient;
pub use fallback::{
    FallbackChain, FallbackChainBuilder, FallbackEvent, FallbackReason, ModelConfig,
    ModelStats as FallbackModelStats, anthropic_fallback_chain, openai_fallback_chain,
};
pub use messages::{CacheControl, LLMMessage, LLMResponse, MessageRole};
pub use providers::LLMProvider;
pub use rate_limiter::{RateLimitConfig, RateLimiter};
pub use request::RequestOptions;   // NEW
pub use sse_decoder::{SSEDecoder, SSEEvent};
pub use streaming::{LLMStream, StreamChunk, StreamingLLMClient};
pub use timeout::{TimeoutConfig, TimeoutDefaults};  // NEW
```

### Phase 2: Configuration Integration (Week 1-2)

#### Step 2.1: Update ProviderConfig

**File:** `crates/sage-core/src/config/provider.rs`

Add the new field and update methods:

```rust
use crate::llm::timeout::{TimeoutConfig, TimeoutDefaults};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub api_version: Option<String>,
    pub organization: Option<String>,
    pub project_id: Option<String>,
    pub headers: HashMap<String, String>,

    /// DEPRECATED: Use timeouts.total instead
    /// Kept for backward compatibility - will be removed in v0.3.0
    #[serde(skip_serializing_if = "Option::is_none")]
    #[deprecated(since = "0.2.0", note = "Use timeouts field instead")]
    pub timeout: Option<u64>,

    /// Granular timeout configuration
    /// Falls back to provider defaults if not specified
    #[serde(default)]
    pub timeouts: Option<TimeoutConfig>,

    pub max_retries: Option<u32>,
    pub rate_limit: Option<RateLimitConfig>,
}

impl ProviderConfig {
    // ... existing methods ...

    /// Get effective timeout configuration
    /// Priority: explicit timeouts > legacy timeout > provider defaults
    pub fn get_timeouts(&self) -> TimeoutConfig {
        // 1. Use explicit timeouts if provided
        if let Some(ref timeouts) = self.timeouts {
            return timeouts.clone();
        }

        // 2. Fall back to legacy timeout field
        #[allow(deprecated)]
        if let Some(timeout_secs) = self.timeout {
            #[cfg(debug_assertions)]
            tracing::warn!(
                "Provider '{}' uses deprecated 'timeout' field. \
                 Please migrate to 'timeouts' object for granular control.",
                self.name
            );
            return TimeoutConfig::from_secs(timeout_secs);
        }

        // 3. Use provider-specific defaults
        TimeoutDefaults::for_provider(&self.name)
    }

    /// Set timeout configuration (builder pattern)
    pub fn with_timeouts(mut self, timeouts: TimeoutConfig) -> Self {
        self.timeouts = Some(timeouts);
        self
    }
}
```

**Testing:**
```bash
cargo test -p sage-core config::provider
```

#### Step 2.2: Update ProviderDefaults

**File:** `crates/sage-core/src/config/provider.rs`

Update the default configurations to use `TimeoutConfig`:

```rust
impl ProviderDefaults {
    pub fn openai() -> ProviderConfig {
        ProviderConfig::new("openai")
            .with_base_url("https://api.openai.com/v1")
            .with_timeouts(TimeoutDefaults::openai())  // CHANGED
            .with_max_retries(3)
            .with_rate_limit(RateLimitConfig {
                requests_per_minute: Some(60),
                tokens_per_minute: Some(100_000),
                max_concurrent_requests: Some(10),
            })
    }

    pub fn anthropic() -> ProviderConfig {
        ProviderConfig::new("anthropic")
            .with_base_url("https://api.anthropic.com")
            .with_api_version("2023-06-01")
            .with_timeouts(TimeoutDefaults::anthropic())  // CHANGED
            .with_max_retries(3)
            .with_rate_limit(RateLimitConfig {
                requests_per_minute: Some(50),
                tokens_per_minute: Some(80_000),
                max_concurrent_requests: Some(5),
            })
    }

    // ... update other providers similarly ...
}
```

### Phase 3: LLMClient Integration (Week 2)

#### Step 3.1: Update LLMClient Constructor

**File:** `crates/sage-core/src/llm/client.rs`

```rust
use crate::llm::request::RequestOptions;
use crate::llm::timeout::TimeoutConfig;

impl LLMClient {
    pub fn new(
        provider: LLMProvider,
        config: ProviderConfig,
        model_params: ModelParameters,
    ) -> SageResult<Self> {
        config.validate()?;

        // Get effective timeout configuration
        let timeouts = config.get_timeouts();

        // Create HTTP client with granular timeouts
        let http_client = Client::builder()
            .connect_timeout(timeouts.connect)
            .timeout(timeouts.total)
            // Note: reqwest doesn't have separate read_timeout in stable API
            // The timeout() sets the total timeout which includes reads
            .build()
            .map_err(|e| SageError::llm(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            provider,
            config,
            model_params,
            http_client,
        })
    }
}
```

#### Step 3.2: Add chat_with_options Method

**File:** `crates/sage-core/src/llm/client.rs`

```rust
impl LLMClient {
    /// Send a chat completion request with custom options
    pub async fn chat_with_options(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[ToolSchema]>,
        options: Option<RequestOptions>,
    ) -> SageResult<LLMResponse> {
        // Apply rate limiting
        let provider_name = self.provider.name();
        let limiter = rate_limiter::get_rate_limiter(provider_name).await;

        if let Some(wait_duration) = limiter.acquire().await {
            debug!(
                "Rate limited for provider '{}', waited {:.2}s",
                provider_name,
                wait_duration.as_secs_f64()
            );
        }

        // Get effective timeout for this request
        let effective_timeout = if let Some(ref opts) = options {
            if let Some(ref timeout_override) = opts.timeout_override {
                debug!(
                    "Request using timeout override: total={:?}, streaming={:?}",
                    timeout_override.total,
                    timeout_override.streaming
                );
                timeout_override.clone()
            } else {
                self.config.get_timeouts()
            }
        } else {
            self.config.get_timeouts()
        };

        // If timeout differs from default, create a new HTTP client for this request
        let http_client = if let Some(ref opts) = options {
            if opts.timeout_override.is_some() {
                Client::builder()
                    .connect_timeout(effective_timeout.connect)
                    .timeout(effective_timeout.total)
                    .build()
                    .map_err(|e| SageError::llm(format!("Failed to create HTTP client: {}", e)))?
            } else {
                self.http_client.clone()
            }
        } else {
            self.http_client.clone()
        };

        // Execute the request with retry logic using the effective timeout
        self.execute_with_retry_and_client(&http_client, || {
            self.execute_request_internal(&http_client, messages, tools)
        })
        .await
    }

    /// Send a chat completion request (convenience method)
    pub async fn chat(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMResponse> {
        self.chat_with_options(messages, tools, None).await
    }

    // Helper method for internal request execution
    async fn execute_request_internal(
        &self,
        http_client: &Client,
        messages: &[LLMMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMResponse> {
        match &self.provider {
            LLMProvider::OpenAI => self.openai_chat_internal(http_client, messages, tools).await,
            LLMProvider::Anthropic => self.anthropic_chat_internal(http_client, messages, tools).await,
            // ... other providers
            _ => Err(SageError::llm("Provider not implemented")),
        }
    }
}
```

#### Step 3.3: Update Streaming Support

**File:** `crates/sage-core/src/llm/client.rs`

```rust
#[async_trait]
impl StreamingLLMClient for LLMClient {
    async fn chat_stream(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMStream> {
        self.chat_stream_with_options(messages, tools, None).await
    }
}

impl LLMClient {
    /// Send a streaming chat completion request with options
    pub async fn chat_stream_with_options(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[ToolSchema]>,
        options: Option<RequestOptions>,
    ) -> SageResult<LLMStream> {
        // Apply rate limiting
        let provider_name = self.provider.name();
        let limiter = rate_limiter::get_rate_limiter(provider_name).await;

        if let Some(wait_duration) = limiter.acquire().await {
            debug!(
                "Rate limited for provider '{}' (streaming), waited {:.2}s",
                provider_name,
                wait_duration.as_secs_f64()
            );
        }

        // Get effective timeout - use streaming timeout if available
        let effective_timeout = if let Some(ref opts) = options {
            if let Some(ref timeout_override) = opts.timeout_override {
                timeout_override.clone()
            } else {
                self.config.get_timeouts()
            }
        } else {
            self.config.get_timeouts()
        };

        // Create HTTP client with streaming timeout
        let streaming_timeout = effective_timeout.effective_streaming();
        let http_client = Client::builder()
            .connect_timeout(effective_timeout.connect)
            .timeout(streaming_timeout)
            .build()
            .map_err(|e| SageError::llm(format!("Failed to create streaming HTTP client: {}", e)))?;

        debug!(
            "Streaming request using timeout: connect={:?}, total={:?}",
            effective_timeout.connect,
            streaming_timeout
        );

        // Execute streaming request
        match &self.provider {
            LLMProvider::OpenAI => self.openai_chat_stream_internal(&http_client, messages, tools).await,
            LLMProvider::Anthropic => self.anthropic_chat_stream_internal(&http_client, messages, tools).await,
            // ... other providers
            _ => Err(SageError::llm("Streaming not supported for this provider")),
        }
    }
}
```

### Phase 4: Configuration File Updates (Week 2)

#### Step 4.1: Update Example Configuration

**File:** `sage_config.json.example`

```json
{
  "default_provider": "anthropic",
  "max_steps": 20,
  "total_token_budget": 100000,

  "model_providers": {
    "anthropic": {
      "api_key": "${ANTHROPIC_API_KEY}",
      "model": "claude-3-sonnet-20240229",
      "max_tokens": 4096,
      "temperature": 0.7,

      "timeouts": {
        "total": "60s",
        "connect": "10s",
        "read": "30s",
        "streaming": "120s",
        "retry": "45s"
      },

      "max_retries": 3
    },

    "google": {
      "api_key": "${GOOGLE_API_KEY}",
      "model": "gemini-2.5-pro",
      "max_tokens": 120000,
      "temperature": 0.7,

      "timeouts": {
        "total": "90s",
        "connect": "15s",
        "read": "45s",
        "streaming": "180s"
      },

      "max_retries": 3
    },

    "ollama": {
      "base_url": "http://localhost:11434",
      "model": "llama3",
      "temperature": 0.7,

      "timeouts": {
        "total": "5m",
        "connect": "5s",
        "read": "60s",
        "streaming": "10m"
      },

      "max_retries": 1
    }
  },

  "tools": {
    "enabled_tools": ["bash", "str_replace_based_edit_tool"],
    "max_execution_time": 300
  }
}
```

### Phase 5: Testing (Week 2-3)

#### Step 5.1: Unit Tests

Create comprehensive unit tests:

```bash
# Test timeout configuration
cargo test -p sage-core timeout

# Test provider config integration
cargo test -p sage-core provider::tests

# Test request options
cargo test -p sage-core request::tests
```

#### Step 5.2: Integration Tests

**File:** `crates/sage-core/tests/timeout_integration_test.rs`

```rust
#[cfg(test)]
mod timeout_integration_tests {
    use sage_core::config::provider::ProviderConfig;
    use sage_core::llm::{LLMClient, RequestOptions, TimeoutConfig};
    use std::time::Duration;

    #[test]
    fn test_provider_default_timeouts() {
        let config = ProviderConfig::new("anthropic");
        let timeouts = config.get_timeouts();

        assert_eq!(timeouts.total, Duration::from_secs(60));
        assert_eq!(timeouts.streaming, Some(Duration::from_secs(120)));
    }

    #[test]
    fn test_legacy_timeout_conversion() {
        #[allow(deprecated)]
        let mut config = ProviderConfig::new("openai");
        config.timeout = Some(90);

        let timeouts = config.get_timeouts();
        assert_eq!(timeouts.total, Duration::from_secs(90));
    }

    #[test]
    fn test_explicit_timeout_override() {
        let custom_timeouts = TimeoutConfig::from_secs(120)
            .with_streaming(Duration::from_secs(240));

        let config = ProviderConfig::new("anthropic")
            .with_timeouts(custom_timeouts);

        let timeouts = config.get_timeouts();
        assert_eq!(timeouts.total, Duration::from_secs(120));
        assert_eq!(timeouts.streaming, Some(Duration::from_secs(240)));
    }

    #[tokio::test]
    async fn test_request_timeout_override() {
        // This would require a real API call or mock
        // Shown here for completeness

        let config = ProviderConfig::new("anthropic");
        // let client = LLMClient::new(...);

        // Normal request - uses provider default
        // let response = client.chat(&messages, None).await;

        // Extended timeout for complex task
        let options = RequestOptions::with_total_timeout(Duration::from_secs(300));
        // let response = client.chat_with_options(&messages, None, Some(options)).await;
    }
}
```

### Phase 6: Documentation (Week 3)

#### Step 6.1: Update User Documentation

Create or update user guide:

**File:** `docs/user-guide/configuration.md`

Add section on timeout configuration with examples.

#### Step 6.2: Update API Documentation

Add rustdoc comments to all public APIs.

#### Step 6.3: Create Migration Guide

**File:** `docs/migration/timeout-config-migration.md`

```markdown
# Migrating to New Timeout Configuration

## Overview

Version 0.2.0 introduces granular timeout configuration. The old `timeout` field
is deprecated but still works for backward compatibility.

## Migration Steps

### Step 1: Identify Current Configuration

**Old format:**
```json
{
  "model_providers": {
    "anthropic": {
      "timeout": 60
    }
  }
}
```

### Step 2: Update to New Format

**New format:**
```json
{
  "model_providers": {
    "anthropic": {
      "timeouts": {
        "total": "60s",
        "streaming": "120s"
      }
    }
  }
}
```

### Step 3: Test Configuration

Run with debug logging to ensure no deprecation warnings:
```bash
RUST_LOG=debug sage run --config sage_config.json
```
```

### Phase 7: Rollout and Monitoring (Week 3-4)

#### Step 7.1: Create Feature Flag (Optional)

If needed, add a feature flag to allow gradual rollout.

#### Step 7.2: Monitor Timeout Metrics

Add telemetry to track:
- Timeout occurrences by provider
- Average request duration
- Timeout override usage

#### Step 7.3: Gather Feedback

Collect user feedback on:
- Default timeout values
- Timeout configuration UX
- Real-world timeout needs

## Testing Checklist

- [ ] All unit tests pass
- [ ] Integration tests pass
- [ ] Backward compatibility verified (legacy `timeout` field works)
- [ ] Default timeouts work for all providers
- [ ] Custom timeouts override defaults correctly
- [ ] Request-level overrides work
- [ ] Streaming timeouts apply correctly
- [ ] Retry timeouts work as expected
- [ ] Configuration file parsing handles all formats
- [ ] Deprecation warnings appear in debug mode
- [ ] Documentation is complete and accurate

## Performance Considerations

1. **HTTP Client Creation**: Request-level timeout overrides create new HTTP clients.
   - **Impact**: Minor overhead per override
   - **Mitigation**: Cache clients for common timeout values if needed

2. **Configuration Cloning**: Timeout configs are cloned frequently.
   - **Impact**: Negligible (small struct)
   - **Mitigation**: None needed

3. **Timeout Resolution**: Happens on every request.
   - **Impact**: Minimal (simple priority check)
   - **Mitigation**: Already optimized

## Troubleshooting

### Common Issues

#### Issue: Deprecation warnings in production

**Solution:** Set config to only warn in debug mode:
```rust
#[cfg(debug_assertions)]
tracing::warn!("...");
```

#### Issue: Timeout too short for complex queries

**Solution:** Use request-level override:
```rust
let options = RequestOptions::with_total_timeout(Duration::from_secs(300));
client.chat_with_options(&messages, tools, Some(options)).await
```

#### Issue: Streaming requests timing out

**Solution:** Set streaming timeout in provider config:
```json
{
  "timeouts": {
    "streaming": "5m"
  }
}
```

## Future Enhancements

1. **Adaptive Timeouts**: Learn optimal timeouts from historical data
2. **Percentile-Based**: Use P95/P99 latencies for dynamic adjustment
3. **Circuit Breaker Integration**: Coordinate timeout with circuit breaker
4. **Telemetry Dashboard**: Visualize timeout patterns
5. **Auto-tuning**: Automatically adjust based on performance

## Success Criteria

- ✅ All providers have sensible default timeouts
- ✅ Users can override at provider and request level
- ✅ Legacy configurations continue to work
- ✅ No performance regression
- ✅ Clear documentation and migration path
- ✅ User feedback is positive
