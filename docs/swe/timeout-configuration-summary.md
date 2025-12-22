# Unified Timeout Configuration - Executive Summary

## Quick Overview

This proposal introduces a comprehensive timeout configuration system for the Sage LLM client that supports three levels of configuration:

1. **Per-Provider Defaults**: Sensible defaults for each LLM provider
2. **User Configuration**: Granular control via config files
3. **Request-Level Overrides**: Runtime timeout adjustments

## Why This Matters

### Current Problems

1. **One-size-fits-all**: Single 60s timeout for all requests and providers
2. **No granularity**: Cannot distinguish between connection, read, and streaming timeouts
3. **Inflexible**: Cannot override timeout for specific requests (e.g., complex reasoning tasks)
4. **Provider-agnostic**: Ollama (local) gets same timeout as OpenAI (cloud)

### Proposed Solutions

1. **Granular timeouts**: Separate connection, read, total, streaming, and retry timeouts
2. **Provider-specific defaults**: Ollama gets 5min, OpenAI gets 60s
3. **Runtime overrides**: Extend timeout for specific complex requests
4. **Backward compatible**: Legacy `timeout` field continues to work

## Configuration Hierarchy

```
Request Override → User Config → Legacy Timeout → Provider Defaults → Global Defaults
   (highest)                                                           (lowest)
```

## Key Features

### 1. TimeoutConfig Structure

```rust
pub struct TimeoutConfig {
    pub total: Duration,           // Total request timeout (60s default)
    pub connect: Duration,         // TCP + TLS handshake (10s default)
    pub read: Duration,            // Between reading bytes (30s default)
    pub streaming: Option<Duration>, // For streaming (120s default)
    pub retry: Option<Duration>,   // Per retry attempt (45s default)
}
```

### 2. Provider Defaults

| Provider   | Total | Connect | Read | Streaming | Notes                    |
|------------|-------|---------|------|-----------|--------------------------|
| OpenAI     | 60s   | 10s     | 30s  | 120s      | Fast, consistent         |
| Anthropic  | 60s   | 10s     | 30s  | 120s      | Similar to OpenAI        |
| Google     | 90s   | 15s     | 45s  | 180s      | Slower with large context|
| Ollama     | 300s  | 5s      | 60s  | 600s      | Local, slow inference    |
| Azure      | 60s   | 15s     | 30s  | 120s      | More network latency     |

### 3. Configuration Examples

#### Provider-Level Configuration

```json
{
  "model_providers": {
    "anthropic": {
      "timeouts": {
        "total": "60s",
        "connect": "10s",
        "streaming": "2m"
      }
    }
  }
}
```

#### Request-Level Override

```rust
// Extend timeout for complex reasoning
let options = RequestOptions::with_total_timeout(Duration::from_secs(300));
let response = client.chat_with_options(&messages, tools, Some(options)).await?;
```

## Implementation Roadmap

### Phase 1: Core Infrastructure (Week 1)
- [ ] Create `timeout.rs` module with `TimeoutConfig` and `TimeoutDefaults`
- [ ] Create `request.rs` module with `RequestOptions`
- [ ] Add `humantime-serde` dependency
- [ ] Write unit tests

### Phase 2: Configuration Integration (Week 1-2)
- [ ] Update `ProviderConfig` with `timeouts` field
- [ ] Implement `get_timeouts()` method with fallback logic
- [ ] Update `ProviderDefaults` to use `TimeoutConfig`
- [ ] Add deprecation warning for legacy `timeout` field

### Phase 3: LLMClient Integration (Week 2)
- [ ] Update `LLMClient::new()` to use granular timeouts
- [ ] Add `chat_with_options()` method
- [ ] Update streaming methods to respect streaming timeout
- [ ] Ensure retry logic uses retry timeout

### Phase 4: Testing & Documentation (Week 2-3)
- [ ] Integration tests for all timeout scenarios
- [ ] Update example configurations
- [ ] Write migration guide
- [ ] Update API documentation

## Migration Path

### Backward Compatibility

The legacy `timeout` field is **deprecated but still works**:

```json
// OLD (still works, but deprecated)
{
  "timeout": 60
}

// NEW (recommended)
{
  "timeouts": {
    "total": "60s",
    "streaming": "120s"
  }
}
```

### Migration Strategy

1. **Phase 1** (v0.2.0): Add new config, keep legacy working
2. **Phase 2** (v0.2.x): Deprecation warnings in debug mode
3. **Phase 3** (v0.3.0): Remove legacy field (breaking change)

## Usage Examples

### Example 1: Use Provider Defaults (Zero Configuration)

```rust
let config = ProviderConfig::new("anthropic")
    .with_api_key(api_key);

let client = LLMClient::new(provider, config, params)?;

// Uses Anthropic defaults: 60s total, 120s streaming
let response = client.chat(&messages, tools).await?;
```

### Example 2: Custom Provider Configuration

```json
{
  "model_providers": {
    "anthropic": {
      "timeouts": {
        "total": "90s",
        "streaming": "3m"
      }
    }
  }
}
```

### Example 3: Request-Level Override

```rust
// Quick query - reduce timeout
let options = RequestOptions::with_total_timeout(Duration::from_secs(15));
let response = client.chat_with_options(&quick_query, None, Some(options)).await?;

// Complex reasoning - extend timeout
let options = RequestOptions::with_total_timeout(Duration::from_secs(300));
let response = client.chat_with_options(&complex_task, tools, Some(options)).await?;
```

### Example 4: Streaming with Custom Timeout

```rust
let timeouts = TimeoutConfig::from_secs(60)
    .with_streaming(Duration::from_secs(300)); // 5 minutes for streaming

let config = ProviderConfig::new("anthropic")
    .with_timeouts(timeouts);

let stream = client.chat_stream(&messages, tools).await?;
```

## Benefits

### 1. Flexibility
- Configure at global, provider, or request level
- Different timeouts for different operations

### 2. Sensible Defaults
- Provider-specific defaults work out of the box
- No configuration needed for common cases

### 3. Backward Compatible
- Legacy configurations continue to work
- Smooth migration path

### 4. Developer Experience
- Human-readable durations ("60s", "2m", "1h")
- Type-safe with Rust's `Duration`
- Clear error messages

### 5. Performance Optimized
- Appropriate timeouts per provider
- Streaming gets longer timeouts
- Local models get much longer timeouts

## Files Modified/Created

### New Files
- `crates/sage-core/src/llm/timeout.rs` (new)
- `crates/sage-core/src/llm/request.rs` (new)
- `docs/swe/timeout-configuration-design.md` (new)
- `docs/swe/timeout-configuration-example.rs` (new)
- `docs/swe/timeout-configuration-diagrams.md` (new)
- `docs/swe/timeout-implementation-guide.md` (new)

### Modified Files
- `crates/sage-core/src/llm/mod.rs` (exports)
- `crates/sage-core/src/llm/client.rs` (use new timeouts)
- `crates/sage-core/src/config/provider.rs` (add timeouts field)
- `crates/sage-core/Cargo.toml` (add humantime-serde)
- `sage_config.json.example` (update examples)

### Dependencies Added
- `humantime-serde = "1.1"` (for duration parsing)

## Testing Strategy

### Unit Tests
- `TimeoutConfig` creation and builders
- Provider defaults verification
- Request options creation
- Legacy timeout conversion

### Integration Tests
- End-to-end timeout application
- Request override behavior
- Streaming timeout handling
- Retry timeout validation

### Manual Testing
- Real API calls with different timeouts
- Timeout failure scenarios
- Provider-specific timeout behavior

## Success Metrics

- [ ] All providers have appropriate default timeouts
- [ ] Zero configuration for 90% of use cases
- [ ] Request overrides work for remaining 10%
- [ ] No performance degradation
- [ ] Clear documentation and examples
- [ ] Positive user feedback

## Risks and Mitigations

### Risk 1: Breaking Changes
**Mitigation**: Maintain backward compatibility for 1-2 minor versions

### Risk 2: Complexity
**Mitigation**: Provide clear examples and sensible defaults

### Risk 3: Performance Overhead
**Mitigation**: Minimal impact (config cloning is cheap)

### Risk 4: User Confusion
**Mitigation**: Comprehensive documentation and migration guide

## Next Steps

1. **Review & Approve**: Get team feedback on design
2. **Implement Core**: Start with Phase 1 (timeout module)
3. **Integrate**: Add to ProviderConfig and LLMClient
4. **Test**: Comprehensive testing with real providers
5. **Document**: Update all documentation
6. **Release**: Ship as v0.2.0 with backward compatibility

## Questions and Answers

### Q: Why separate connection and read timeouts?
**A**: Connection failures (network issues) vs. slow responses (server processing) have different characteristics and optimal timeouts.

### Q: Why provider-specific defaults?
**A**: Ollama (local) is much slower than OpenAI (cloud), requires different timeouts.

### Q: Will this break existing configs?
**A**: No, legacy `timeout` field continues to work with deprecation warning.

### Q: How do I extend timeout for one complex request?
**A**: Use `RequestOptions::with_total_timeout(duration)` for that request.

### Q: What happens if I set total < connect?
**A**: HTTP client uses the most restrictive timeout. Best to keep total >= connect.

## References

- Design Doc: `docs/swe/timeout-configuration-design.md`
- Implementation Guide: `docs/swe/timeout-implementation-guide.md`
- Diagrams: `docs/swe/timeout-configuration-diagrams.md`
- Example Code: `docs/swe/timeout-configuration-example.rs`

---

**Document Version**: 1.0
**Last Updated**: 2025-12-22
**Author**: Claude (Sonnet 4.5)
**Status**: Proposal - Ready for Review
