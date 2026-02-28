# Timeout Configuration for LLM Providers

## Overview

Sage Agent now provides unified timeout handling across all LLM providers through the `TimeoutConfig` struct. This ensures consistent behavior and fine-grained control over connection and request timeouts.

## Configuration Structure

```rust
pub struct TimeoutConfig {
    /// Connection timeout in seconds (default: 30s)
    pub connection_timeout_secs: u64,

    /// Request timeout in seconds (default: 60s)
    pub request_timeout_secs: u64,
}
```

## Timeout Types

### Connection Timeout
Maximum time allowed to establish a TCP connection to the API server.
- **Default**: 30 seconds
- **Purpose**: Prevents hanging on unresponsive or slow-to-connect servers
- **Applies to**: Initial TCP handshake and TLS negotiation

### Request Timeout
Maximum time allowed for the complete request/response cycle.
- **Default**: 60 seconds
- **Purpose**: Ensures requests complete in reasonable time
- **Applies to**: Connection establishment + sending request + receiving response

## Usage Examples

### Basic Usage (Defaults)

```rust
use sage_core::config::provider::ProviderConfig;

// Uses default timeouts (30s connection, 60s request)
let config = ProviderConfig::new("openai");
```

### Custom Timeouts

```rust
use sage_core::config::provider::ProviderConfig;
use sage_core::llm::TimeoutConfig;

// Custom timeout configuration
let timeouts = TimeoutConfig::new()
    .with_connection_timeout_secs(15)
    .with_request_timeout_secs(120);

let config = ProviderConfig::new("anthropic")
    .with_timeouts(timeouts);
```

### Preset Configurations

```rust
use sage_core::llm::TimeoutConfig;

// Quick timeouts for fast local models
let quick = TimeoutConfig::quick();
// Connection: 5s, Request: 30s

// Relaxed timeouts for slow connections or large requests
let relaxed = TimeoutConfig::relaxed();
// Connection: 60s, Request: 300s (5 minutes)
```

## Provider-Specific Defaults

Different providers have different default timeout configurations optimized for their characteristics:

| Provider | Connection Timeout | Request Timeout | Notes |
|----------|-------------------|-----------------|-------|
| OpenAI | 30s | 60s | Standard cloud API |
| Anthropic | 30s | 60s | Standard cloud API |
| Google | 30s | 60s | Standard cloud API |
| Azure | 30s | 60s | Standard cloud API |
| OpenRouter | 30s | 90s | Longer due to routing overhead |
| Doubao | 30s | 60s | Standard cloud API |
| GLM | 30s | 60s | Standard cloud API |
| Ollama | 10s | 120s | Optimized for local models |

## JSON Configuration

### New Format (Recommended)

```json
{
  "providers": {
    "openai": {
      "api_key": "${OPENAI_API_KEY}",
      "timeouts": {
        "connection_timeout_secs": 30,
        "request_timeout_secs": 60
      }
    }
  }
}
```

### Legacy Format (Still Supported)

For backward compatibility, the old `timeout` field is still supported:

```json
{
  "providers": {
    "openai": {
      "api_key": "${OPENAI_API_KEY}",
      "timeout": 60
    }
  }
}
```

**Note**: If both `timeout` and `timeouts` are specified, the legacy `timeout` field will override the `request_timeout_secs` in `timeouts`.

## Validation Rules

The timeout configuration enforces the following rules:

1. **Connection timeout must be > 0**
2. **Request timeout must be > 0**
3. **Request timeout must be >= connection timeout**

Invalid configurations will be rejected during validation with descriptive error messages.

## Timeout Behavior

### Successful Request
```
[Connection] -> [Send Request] -> [Receive Response] -> [Success]
     ↓                                      ↓
  < 30s                                  < 60s total
```

### Connection Timeout
```
[Attempting Connection...] -> TIMEOUT (30s) -> Error
```

### Request Timeout
```
[Connected] -> [Request Sent] -> [Waiting...] -> TIMEOUT (60s) -> Error
```

## Retry Behavior

When a timeout occurs, the request is automatically retried with exponential backoff:

1. **Timeout detected** → Classified as retryable error
2. **Exponential backoff** → Wait 2^attempt seconds + jitter
3. **Retry attempt** → New request with same timeout configuration
4. **Max retries** → Configurable per provider (default: 3)

## Best Practices

### 1. Choose Appropriate Timeouts

```rust
// For fast, synchronous operations
let timeouts = TimeoutConfig::quick();

// For long-running generation tasks
let timeouts = TimeoutConfig::relaxed();

// For most use cases
let timeouts = TimeoutConfig::default();
```

### 2. Consider Network Conditions

```rust
// Slow or unreliable network
let timeouts = TimeoutConfig::new()
    .with_connection_timeout_secs(60)
    .with_request_timeout_secs(180);
```

### 3. Match Provider Characteristics

```rust
// Local Ollama instance - fast connection, slow generation
let ollama_timeouts = TimeoutConfig::new()
    .with_connection_timeout_secs(5)
    .with_request_timeout_secs(300);

// Cloud API with routing - slower connection, standard generation
let router_timeouts = TimeoutConfig::new()
    .with_connection_timeout_secs(45)
    .with_request_timeout_secs(120);
```

### 4. Monitor and Adjust

Use debug logging to monitor timeout effectiveness:

```
Created LLM client for provider 'openai' with timeouts: connection=30s, request=60s
```

Adjust timeouts based on observed behavior in your environment.

## Migration Guide

### From Legacy `timeout` Field

**Before:**
```rust
let config = ProviderConfig::new("openai")
    .with_timeout(120);  // Deprecated
```

**After:**
```rust
let config = ProviderConfig::new("openai")
    .with_timeouts(
        TimeoutConfig::new()
            .with_request_timeout_secs(120)
    );
```

### From Hardcoded Values

**Before:**
```rust
let client = Client::builder()
    .timeout(Duration::from_secs(60))
    .build()?;
```

**After:**
```rust
let timeouts = config.get_effective_timeouts();
let client = Client::builder()
    .connect_timeout(timeouts.connection_timeout())
    .timeout(timeouts.request_timeout())
    .build()?;
```

## Troubleshooting

### Requests Timing Out Too Early

Increase the request timeout:
```rust
.with_timeouts(
    TimeoutConfig::new()
        .with_request_timeout_secs(180)
)
```

### Slow to Detect Failed Connections

Decrease the connection timeout:
```rust
.with_timeouts(
    TimeoutConfig::new()
        .with_connection_timeout_secs(10)
)
```

### "Request timeout must be >= connection timeout" Error

Ensure request timeout is at least as large as connection timeout:
```rust
// ❌ Invalid
TimeoutConfig::new()
    .with_connection_timeout_secs(60)
    .with_request_timeout_secs(30)  // Error!

// ✅ Valid
TimeoutConfig::new()
    .with_connection_timeout_secs(30)
    .with_request_timeout_secs(60)
```

## Performance Considerations

- **Lower timeouts** = Faster failure detection, but may abort legitimate slow requests
- **Higher timeouts** = More tolerance for slow responses, but slower failure detection
- **Connection timeout** should be low enough to quickly detect network issues
- **Request timeout** should accommodate the slowest expected legitimate response

## Security Implications

Proper timeout configuration helps prevent:
- **Denial of Service**: Prevents resource exhaustion from hanging connections
- **Resource Leaks**: Ensures connections are released in reasonable time
- **Deadlocks**: Prevents infinite waits on unresponsive servers
