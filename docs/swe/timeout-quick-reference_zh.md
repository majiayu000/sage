# Timeout Configuration Quick Reference

## Timeout Types

| Type | Default | Description | When Applied |
|------|---------|-------------|--------------|
| **total** | 60s | Maximum time from request start to finish | Entire HTTP request |
| **connect** | 10s | Time to establish TCP + TLS connection | Connection phase only |
| **read** | 30s | Time between receiving bytes | Per-read operation |
| **streaming** | 120s | Total time for streaming responses | SSE/streaming requests |
| **retry** | 45s | Timeout for individual retry attempts | Each retry |

## Provider Defaults

```rust
// OpenAI & Anthropic (fast, cloud APIs)
total: 60s, connect: 10s, read: 30s, streaming: 120s

// Google (slower with large context)
total: 90s, connect: 15s, read: 45s, streaming: 180s

// Ollama (local, slow inference)
total: 300s, connect: 5s, read: 60s, streaming: 600s

// Azure (more network latency)
total: 60s, connect: 15s, read: 30s, streaming: 120s
```

## Configuration Examples

### Zero Configuration (Use Defaults)

```json
{
  "model_providers": {
    "anthropic": {
      "api_key": "${ANTHROPIC_API_KEY}"
    }
  }
}
```

Uses Anthropic defaults automatically.

### Custom Provider Timeout

```json
{
  "model_providers": {
    "anthropic": {
      "api_key": "${ANTHROPIC_API_KEY}",
      "timeouts": {
        "total": "90s",
        "streaming": "3m"
      }
    }
  }
}
```

### All Timeout Types

```json
{
  "timeouts": {
    "total": "60s",
    "connect": "10s",
    "read": "30s",
    "streaming": "120s",
    "retry": "45s"
  }
}
```

### Legacy Format (Deprecated)

```json
{
  "timeout": 60
}
```

Still works, converts to `total: 60s`.

## Rust API Examples

### Use Provider Defaults

```rust
let config = ProviderConfig::new("anthropic")
    .with_api_key(api_key);

let client = LLMClient::new(provider, config, params)?;
```

### Custom Provider Timeout

```rust
let timeouts = TimeoutConfig::from_secs(90)
    .with_streaming(Duration::from_secs(180));

let config = ProviderConfig::new("anthropic")
    .with_timeouts(timeouts);
```

### Request-Level Override

```rust
// Quick query - short timeout
let opts = RequestOptions::with_total_timeout(Duration::from_secs(15));
let response = client.chat_with_options(&messages, None, Some(opts)).await?;

// Complex task - long timeout
let opts = RequestOptions::with_total_timeout(Duration::from_secs(300));
let response = client.chat_with_options(&messages, tools, Some(opts)).await?;

// Custom timeout config
let opts = RequestOptions::with_timeout(
    TimeoutConfig::from_secs(120)
        .with_streaming(Duration::from_secs(240))
);
let response = client.chat_with_options(&messages, tools, Some(opts)).await?;
```

### Streaming

```rust
// Uses streaming timeout (120s default for Anthropic)
let stream = client.chat_stream(&messages, tools).await?;

// Custom streaming timeout
let opts = RequestOptions::with_timeout(
    TimeoutConfig::from_secs(60)
        .with_streaming(Duration::from_secs(300))
);
let stream = client.chat_stream_with_options(&messages, tools, Some(opts)).await?;
```

## Timeout Resolution Priority

```
1. RequestOptions.timeout_override   (highest)
2. ProviderConfig.timeouts
3. ProviderConfig.timeout (legacy)
4. TimeoutDefaults::for_provider()
5. TimeoutConfig::default()         (lowest)
```

## Human-Readable Duration Format

```json
{
  "timeouts": {
    "total": "60s",      // 60 seconds
    "connect": "10s",    // 10 seconds
    "streaming": "2m",   // 2 minutes
    "retry": "1m30s"     // 1 minute 30 seconds
  }
}
```

Supported units: `s` (seconds), `m` (minutes), `h` (hours)

## Common Use Cases

### Quick Question

```rust
// Use short timeout for simple queries
let opts = RequestOptions::with_total_timeout(Duration::from_secs(15));
client.chat_with_options(&["What is 2+2?"], None, Some(opts)).await?
```

### Code Generation

```rust
// Use long timeout for complex tasks
let opts = RequestOptions::with_timeout(
    TimeoutConfig::from_secs(300)
        .with_streaming(Duration::from_secs(600))
);
client.chat_with_options(&code_gen_prompt, tools, Some(opts)).await?
```

### Local Model

```json
{
  "model_providers": {
    "ollama": {
      "timeouts": {
        "total": "5m",
        "streaming": "10m"
      }
    }
  }
}
```

### Production API (Strict Limits)

```json
{
  "model_providers": {
    "openai": {
      "timeouts": {
        "total": "30s",
        "retry": "20s"
      }
    }
  }
}
```

## Debugging

### Enable Debug Logging

```bash
RUST_LOG=debug sage run
```

Logs will show:
- Applied timeouts for each request
- Timeout overrides
- Deprecation warnings (for legacy config)

### Check Effective Timeout

```rust
let timeouts = config.get_timeouts();
println!("Total: {:?}", timeouts.total);
println!("Streaming: {:?}", timeouts.effective_streaming());
```

### Test Timeout Behavior

```rust
// Set very short timeout to trigger failure
let opts = RequestOptions::with_total_timeout(Duration::from_millis(1));
match client.chat_with_options(&messages, None, Some(opts)).await {
    Err(e) if e.to_string().contains("timeout") => {
        println!("Timeout triggered as expected");
    }
    _ => panic!("Should have timed out"),
}
```

## Migration Cheat Sheet

### Old → New

```json
// Before (v0.1.x)
{
  "timeout": 60
}

// After (v0.2.x)
{
  "timeouts": {
    "total": "60s"
  }
}
```

### Add Streaming Timeout

```json
// Before
{
  "timeout": 60
}

// After (with streaming support)
{
  "timeouts": {
    "total": "60s",
    "streaming": "120s"
  }
}
```

## Performance Tips

### Minimize Client Creation

```rust
// BAD: Creates new client for each request
for query in queries {
    let opts = RequestOptions::with_total_timeout(Duration::from_secs(120));
    client.chat_with_options(&query, None, Some(opts)).await?;
}

// GOOD: Reuse same options if timeout is constant
let opts = RequestOptions::with_total_timeout(Duration::from_secs(120));
for query in queries {
    client.chat_with_options(&query, None, Some(opts.clone())).await?;
}

// BEST: Set at provider level if all requests need same timeout
let config = ProviderConfig::new("anthropic")
    .with_timeouts(TimeoutConfig::from_secs(120));
let client = LLMClient::new(provider, config, params)?;

for query in queries {
    client.chat(&query, None).await?;  // Uses provider config
}
```

## Troubleshooting

### "Request timed out"

1. Check if timeout is appropriate for request complexity
2. Try increasing timeout for complex requests
3. Check network latency to provider
4. Verify provider service status

### "Connection timeout"

1. Check network connectivity
2. Verify base_url is correct
3. Check firewall/proxy settings
4. Try increasing connect timeout

### "Deprecation warning: timeout field"

Update config from:
```json
{"timeout": 60}
```
to:
```json
{"timeouts": {"total": "60s"}}
```

## Best Practices

1. **Use defaults first**: Provider defaults work for 90% of cases
2. **Configure at provider level**: For consistent behavior
3. **Override at request level**: Only for exceptional cases
4. **Set streaming timeout**: Higher than regular timeout
5. **Local models need longer**: Ollama/local need 5-10x longer
6. **Monitor timeout failures**: Track and adjust based on metrics
7. **Test with real APIs**: Verify timeouts in development

## Limits and Constraints

- **Minimum timeout**: 1 second (enforced by validation)
- **Maximum timeout**: No hard limit, but consider:
  - Cloud provider billing (longer = more cost)
  - User experience (>5min is too long)
  - Resource utilization
- **Timeout granularity**: Millisecond precision, but 1-second minimum recommended
- **Retry timeout**: Should be ≤ total timeout

## Quick Decision Tree

```
Is this a quick query?
├─ Yes → Use 15-30s timeout
└─ No
   └─ Is this using a cloud API?
      ├─ Yes → Use default (60s) or 90s for complex
      └─ No (local model)
         └─ Use 300s+ timeout
```

## Resources

- Full Design: `docs/swe/timeout-configuration-design.md`
- Implementation Guide: `docs/swe/timeout-implementation-guide.md`
- Diagrams: `docs/swe/timeout-configuration-diagrams.md`
- Examples: `docs/swe/timeout-configuration-example.rs`

---

**Version**: 1.0
**For**: Sage Agent v0.2.0+
**Last Updated**: 2025-12-22
