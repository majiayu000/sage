# Unified Timeout Configuration Design

## Executive Summary

This document proposes a unified timeout configuration system for the Sage LLM client that supports:
1. Per-provider default timeouts with sensible fallbacks
2. User configuration at multiple levels (global, provider, request)
3. Request-level overrides for fine-grained control
4. Different timeout types (connection, read, streaming)

## Current State Analysis

### Existing Timeout Configuration

#### 1. HTTP Client Level (`llm/client.rs`)
```rust
// Line 43
Client::builder().timeout(Duration::from_secs(config.timeout.unwrap_or(60)))
```
- Single blanket timeout for all HTTP operations
- No distinction between connection and read timeouts
- Default: 60 seconds

#### 2. Provider Configuration (`config/provider.rs`)
```rust
pub struct ProviderConfig {
    // ...
    pub timeout: Option<u64>,  // Request timeout in seconds
    // ...
}
```
- Provider-specific timeout
- Defaults: 60s (most providers), 120s (Ollama)
- Applied uniformly to all requests from that provider

#### 3. Execution Options (`agent/options.rs`)
```rust
pub struct ExecutionOptions {
    pub execution_timeout: Option<Duration>,  // Total execution time
    pub prompt_timeout: Option<Duration>,     // User prompt timeout
    // ...
}
```
- Only covers execution-level timeouts
- No LLM request timeout configuration

### Gaps in Current Implementation

1. **No timeout type differentiation**: Cannot set different timeouts for:
   - Connection establishment (TCP handshake, TLS negotiation)
   - Read operations (waiting for response data)
   - Streaming vs. non-streaming requests

2. **No request-level overrides**: All requests share provider timeout
   - Cannot extend timeout for complex reasoning tasks
   - Cannot reduce timeout for simple queries

3. **No retry timeout configuration**: Exponential backoff uses fixed intervals
   - Retry delays: 2^attempt seconds (hardcoded in `client.rs:110`)

4. **Streaming timeout limitations**: Same timeout for streaming and non-streaming
   - Streaming may need longer timeouts for slow token generation

## Proposed Design

### 1. Timeout Configuration Hierarchy

```
Global Defaults → Provider Defaults → User Config → Request Override
     (code)          (ProviderDefaults)    (config file)    (runtime)
```

### 2. New Data Structures

#### A. Enhanced Timeout Configuration

```rust
// crates/sage-core/src/llm/timeout.rs

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Granular timeout configuration for LLM requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    /// Total timeout for the entire request (connection + read)
    /// This is the maximum time from request start to completion
    /// Default: 60 seconds
    #[serde(default = "default_total_timeout")]
    #[serde(with = "humantime_serde")]
    pub total: Duration,

    /// Timeout for establishing connection
    /// Default: 10 seconds
    #[serde(default = "default_connect_timeout")]
    #[serde(with = "humantime_serde")]
    pub connect: Duration,

    /// Timeout for reading response data
    /// Applied per-read operation, not total read time
    /// Default: 30 seconds
    #[serde(default = "default_read_timeout")]
    #[serde(with = "humantime_serde")]
    pub read: Duration,

    /// Timeout for streaming requests (if different from regular)
    /// Default: None (uses total timeout)
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "humantime_serde")]
    pub streaming: Option<Duration>,

    /// Timeout for individual retry attempts
    /// Default: None (uses total timeout)
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "humantime_serde")]
    pub retry: Option<Duration>,
}

// Default timeout values
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
    /// Create a new timeout config with custom total timeout
    pub fn new(total: Duration) -> Self {
        Self {
            total,
            ..Default::default()
        }
    }

    /// Create from seconds (for convenience)
    pub fn from_secs(secs: u64) -> Self {
        Self::new(Duration::from_secs(secs))
    }

    /// Builder pattern setters
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

    /// Get effective streaming timeout
    pub fn effective_streaming(&self) -> Duration {
        self.streaming.unwrap_or(self.total)
    }

    /// Get effective retry timeout
    pub fn effective_retry(&self) -> Duration {
        self.retry.unwrap_or(self.total)
    }

    /// Merge with another config (other takes precedence)
    pub fn merge(&self, other: &TimeoutConfig) -> Self {
        Self {
            total: other.total,
            connect: other.connect,
            read: other.read,
            streaming: other.streaming.or(self.streaming),
            retry: other.retry.or(self.retry),
        }
    }

    /// Create a request-specific override
    pub fn override_total(&self, total: Duration) -> Self {
        Self {
            total,
            ..self.clone()
        }
    }
}
```

#### B. Per-Provider Defaults

```rust
// crates/sage-core/src/llm/timeout.rs (continued)

/// Provider-specific timeout defaults
pub struct TimeoutDefaults;

impl TimeoutDefaults {
    /// OpenAI defaults - fast response expected
    pub fn openai() -> TimeoutConfig {
        TimeoutConfig {
            total: Duration::from_secs(60),
            connect: Duration::from_secs(10),
            read: Duration::from_secs(30),
            streaming: Some(Duration::from_secs(120)), // Longer for streaming
            retry: Some(Duration::from_secs(45)),
        }
    }

    /// Anthropic defaults - similar to OpenAI
    pub fn anthropic() -> TimeoutConfig {
        TimeoutConfig {
            total: Duration::from_secs(60),
            connect: Duration::from_secs(10),
            read: Duration::from_secs(30),
            streaming: Some(Duration::from_secs(120)),
            retry: Some(Duration::from_secs(45)),
        }
    }

    /// Google defaults - can be slower for large context
    pub fn google() -> TimeoutConfig {
        TimeoutConfig {
            total: Duration::from_secs(90),
            connect: Duration::from_secs(15),
            read: Duration::from_secs(45),
            streaming: Some(Duration::from_secs(180)),
            retry: Some(Duration::from_secs(60)),
        }
    }

    /// Ollama defaults - local model, much slower
    pub fn ollama() -> TimeoutConfig {
        TimeoutConfig {
            total: Duration::from_secs(300), // 5 minutes
            connect: Duration::from_secs(5),  // Local, fast connect
            read: Duration::from_secs(60),    // Slower inference
            streaming: Some(Duration::from_secs(600)), // 10 minutes for streaming
            retry: Some(Duration::from_secs(180)),
        }
    }

    /// Azure defaults - similar to OpenAI
    pub fn azure() -> TimeoutConfig {
        TimeoutConfig {
            total: Duration::from_secs(60),
            connect: Duration::from_secs(15), // May have more latency
            read: Duration::from_secs(30),
            streaming: Some(Duration::from_secs(120)),
            retry: Some(Duration::from_secs(45)),
        }
    }

    /// GLM defaults
    pub fn glm() -> TimeoutConfig {
        TimeoutConfig {
            total: Duration::from_secs(60),
            connect: Duration::from_secs(10),
            read: Duration::from_secs(30),
            streaming: Some(Duration::from_secs(120)),
            retry: Some(Duration::from_secs(45)),
        }
    }

    /// Get default timeout config for a provider
    pub fn for_provider(provider: &str) -> TimeoutConfig {
        match provider {
            "openai" => Self::openai(),
            "anthropic" => Self::anthropic(),
            "google" => Self::google(),
            "ollama" => Self::ollama(),
            "azure" => Self::azure(),
            "glm" | "zhipu" => Self::glm(),
            _ => TimeoutConfig::default(),
        }
    }
}
```

#### C. Updated ProviderConfig

```rust
// crates/sage-core/src/config/provider.rs

use crate::llm::timeout::TimeoutConfig;

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,

    /// Granular timeout configuration
    /// Falls back to provider defaults if not specified
    #[serde(default)]
    pub timeouts: Option<TimeoutConfig>,

    pub max_retries: Option<u32>,
    pub rate_limit: Option<RateLimitConfig>,
}

impl ProviderConfig {
    /// Get effective timeout configuration
    pub fn get_timeouts(&self) -> TimeoutConfig {
        // Priority: explicit timeouts > legacy timeout > provider defaults
        if let Some(ref timeouts) = self.timeouts {
            return timeouts.clone();
        }

        // Fallback to legacy timeout field
        if let Some(timeout_secs) = self.timeout {
            return TimeoutConfig::from_secs(timeout_secs);
        }

        // Use provider-specific defaults
        TimeoutDefaults::for_provider(&self.name)
    }

    // Builder methods
    pub fn with_timeouts(mut self, timeouts: TimeoutConfig) -> Self {
        self.timeouts = Some(timeouts);
        self
    }
}
```

#### D. Request-Level Timeout Override

```rust
// crates/sage-core/src/llm/request.rs

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
    /// Create request options with timeout override
    pub fn with_timeout(timeout: TimeoutConfig) -> Self {
        Self {
            timeout_override: Some(timeout),
            ..Default::default()
        }
    }

    /// Create request options with just total timeout override
    pub fn with_total_timeout(duration: Duration) -> Self {
        Self::with_timeout(TimeoutConfig::new(duration))
    }
}
```

#### E. Updated LLMClient

```rust
// crates/sage-core/src/llm/client.rs

impl LLMClient {
    /// Create a new LLM client with timeout configuration
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
            .read_timeout(timeouts.read)
            .build()
            .map_err(|e| SageError::llm(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            provider,
            config,
            model_params,
            http_client,
        })
    }

    /// Send a chat completion request with optional timeout override
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
            debug!("Rate limited for provider '{}', waited {:.2}s",
                   provider_name, wait_duration.as_secs_f64());
        }

        // Get effective timeout for this request
        let effective_timeout = if let Some(ref opts) = options {
            if let Some(ref timeout_override) = opts.timeout_override {
                timeout_override.clone()
            } else {
                self.config.get_timeouts()
            }
        } else {
            self.config.get_timeouts()
        };

        // Store timeout in request context for provider methods to use
        // ... execute request ...
    }

    /// Send a chat completion request (convenience method)
    pub async fn chat(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMResponse> {
        self.chat_with_options(messages, tools, None).await
    }
}
```

### 3. Configuration File Format

#### Example `sage_config.json`

```json
{
  "default_provider": "anthropic",
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

      "timeouts": {
        "total": "90s",
        "connect": "15s",
        "read": "45s",
        "streaming": "180s"
      }
    },

    "ollama": {
      "base_url": "http://localhost:11434",
      "model": "llama3",

      "timeouts": {
        "total": "300s",
        "connect": "5s",
        "read": "60s",
        "streaming": "600s"
      },

      "max_retries": 1
    }
  }
}
```

### 4. Migration Strategy

#### Phase 1: Add New Configuration (Backward Compatible)
1. Add `TimeoutConfig` struct with defaults
2. Add `timeouts` field to `ProviderConfig` (optional)
3. Keep legacy `timeout` field working
4. Update `LLMClient` to use `get_timeouts()` helper

#### Phase 2: Deprecation Warnings
1. Add deprecation warning when `timeout` field is used
2. Document migration path in warnings
3. Update example configs to use new format

#### Phase 3: Remove Legacy (Breaking Change)
1. Remove `timeout` field from `ProviderConfig`
2. Update all example configs
3. Bump major version

### 5. Usage Examples

#### A. Using Provider Defaults
```rust
// No timeout config needed - uses provider-specific defaults
let config = ProviderConfig::new("anthropic")
    .with_api_key(api_key);

let client = LLMClient::new(
    LLMProvider::Anthropic,
    config,
    model_params,
)?;
```

#### B. Custom Provider-Level Timeouts
```rust
let timeouts = TimeoutConfig::default()
    .with_connect(Duration::from_secs(15))
    .with_streaming(Duration::from_secs(180));

let config = ProviderConfig::new("anthropic")
    .with_api_key(api_key)
    .with_timeouts(timeouts);
```

#### C. Request-Level Override
```rust
// Normal request - uses provider timeout
let response = client.chat(&messages, Some(&tools)).await?;

// Complex reasoning - extend timeout
let options = RequestOptions::with_total_timeout(Duration::from_secs(300));
let response = client.chat_with_options(&messages, Some(&tools), Some(options)).await?;

// Quick query - reduce timeout
let options = RequestOptions::with_total_timeout(Duration::from_secs(15));
let response = client.chat_with_options(&messages, Some(&tools), Some(options)).await?;
```

#### D. Configuration File
```json
{
  "model_providers": {
    "anthropic": {
      "timeouts": {
        "total": "60s",
        "streaming": "2m"
      }
    }
  }
}
```

### 6. Implementation Checklist

- [ ] Create `crates/sage-core/src/llm/timeout.rs`
  - [ ] Define `TimeoutConfig` struct
  - [ ] Implement `TimeoutDefaults`
  - [ ] Add builder methods
  - [ ] Add serde support with `humantime_serde`

- [ ] Update `crates/sage-core/src/llm/request.rs`
  - [ ] Create new file if doesn't exist
  - [ ] Define `RequestOptions` struct

- [ ] Update `crates/sage-core/src/config/provider.rs`
  - [ ] Add `timeouts: Option<TimeoutConfig>` field
  - [ ] Implement `get_timeouts()` method
  - [ ] Add deprecation warning for `timeout` field
  - [ ] Update `ProviderDefaults` to use `TimeoutConfig`

- [ ] Update `crates/sage-core/src/llm/client.rs`
  - [ ] Modify `new()` to use granular timeouts
  - [ ] Add `chat_with_options()` method
  - [ ] Update retry logic to respect retry timeout
  - [ ] Apply streaming timeout for streaming requests

- [ ] Update `crates/sage-core/src/llm/mod.rs`
  - [ ] Export timeout types
  - [ ] Export request types

- [ ] Add dependencies to `Cargo.toml`
  - [ ] Add `humantime-serde` for duration parsing

- [ ] Update configuration files
  - [ ] Update `sage_config.json.example`
  - [ ] Update documentation

- [ ] Add tests
  - [ ] Unit tests for `TimeoutConfig`
  - [ ] Integration tests for timeout behavior
  - [ ] Migration tests (legacy → new format)

### 7. Benefits

1. **Flexibility**: Configure timeouts at multiple levels
2. **Sensible Defaults**: Provider-specific defaults work out of the box
3. **Backward Compatible**: Legacy `timeout` field still works
4. **Fine-Grained Control**: Different timeouts for different operations
5. **Human-Readable**: Use "60s", "2m", "1h" in config files
6. **Request-Specific**: Override timeout for specific requests
7. **Type-Safe**: Compile-time checking with Rust types

### 8. Future Enhancements

1. **Adaptive Timeouts**: Learn optimal timeouts from usage patterns
2. **Percentile-Based**: Set timeout based on P95/P99 latencies
3. **Circuit Breaker Integration**: Coordinate with circuit breaker
4. **Telemetry**: Track timeout occurrences and patterns
5. **Dynamic Adjustment**: Adjust based on load/performance

## Appendix

### A. HTTP Timeout Types Explained

- **Connection Timeout**: Time to establish TCP connection + TLS handshake
- **Read Timeout**: Time between receiving bytes (not total read time)
- **Total Timeout**: Maximum time from request start to completion
- **Streaming Timeout**: Special handling for SSE streams (longer TTL)

### B. Provider Timeout Recommendations

| Provider   | Total | Connect | Read | Streaming | Notes                    |
|------------|-------|---------|------|-----------|--------------------------|
| OpenAI     | 60s   | 10s     | 30s  | 120s      | Fast, consistent         |
| Anthropic  | 60s   | 10s     | 30s  | 120s      | Similar to OpenAI        |
| Google     | 90s   | 15s     | 45s  | 180s      | Slower with large context|
| Ollama     | 300s  | 5s      | 60s  | 600s      | Local, slow inference    |
| Azure      | 60s   | 15s     | 30s  | 120s      | More network latency     |

### C. Related Files

- `crates/sage-core/src/llm/client.rs` - HTTP client setup
- `crates/sage-core/src/config/provider.rs` - Provider configuration
- `crates/sage-core/src/llm/fallback.rs` - Fallback on timeout
- `crates/sage-core/src/agent/options.rs` - Execution timeouts
