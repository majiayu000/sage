# Timeout Configuration Implementation Checklist

## Pre-Implementation

- [ ] Review design document with team
- [ ] Get approval from maintainers
- [ ] Confirm backward compatibility requirements
- [ ] Plan release version (suggested: v0.2.0)

## Phase 1: Core Infrastructure ⏱️ Week 1

### 1.1 Dependencies

- [ ] Add `humantime-serde = "1.1"` to `crates/sage-core/Cargo.toml`
- [ ] Run `cargo update` to fetch dependency
- [ ] Verify dependency compiles: `cargo check -p sage-core`

### 1.2 Create Timeout Module

- [ ] Create file: `crates/sage-core/src/llm/timeout.rs`
- [ ] Implement `TimeoutConfig` struct
  - [ ] Add fields: total, connect, read, streaming, retry
  - [ ] Add `#[serde]` attributes with `humantime_serde`
  - [ ] Implement `Default` trait
  - [ ] Add builder methods: `new()`, `from_secs()`, `with_*()`
  - [ ] Add helper methods: `effective_streaming()`, `effective_retry()`
  - [ ] Add `merge()` method
  - [ ] Add `override_total()` method
- [ ] Implement `TimeoutDefaults` struct
  - [ ] Add `openai()` method
  - [ ] Add `anthropic()` method
  - [ ] Add `google()` method
  - [ ] Add `ollama()` method
  - [ ] Add `azure()` method
  - [ ] Add `glm()` method
  - [ ] Add `doubao()` method
  - [ ] Add `for_provider()` method
- [ ] Add unit tests
  - [ ] Test default values
  - [ ] Test builder pattern
  - [ ] Test effective timeouts
  - [ ] Test provider defaults
  - [ ] Test merge behavior
  - [ ] Test serde serialization/deserialization
- [ ] Run tests: `cargo test -p sage-core llm::timeout`

### 1.3 Create Request Module

- [ ] Create file: `crates/sage-core/src/llm/request.rs`
- [ ] Implement `RequestOptions` struct
  - [ ] Add `timeout_override` field
  - [ ] Add `priority` field
  - [ ] Add `metadata` field
  - [ ] Implement `Default` trait
  - [ ] Add builder methods: `new()`, `with_timeout()`, `with_total_timeout()`
  - [ ] Add `with_metadata()` method
  - [ ] Add `with_priority()` method
- [ ] Add unit tests
  - [ ] Test creation methods
  - [ ] Test builder pattern
  - [ ] Test metadata handling
- [ ] Run tests: `cargo test -p sage-core llm::request`

### 1.4 Update Module Exports

- [ ] Edit `crates/sage-core/src/llm/mod.rs`
  - [ ] Add `pub mod timeout;`
  - [ ] Add `pub mod request;`
  - [ ] Add `pub use timeout::{TimeoutConfig, TimeoutDefaults};`
  - [ ] Add `pub use request::RequestOptions;`
- [ ] Verify exports: `cargo check -p sage-core`

## Phase 2: Configuration Integration ⏱️ Week 1-2

### 2.1 Update ProviderConfig

- [ ] Edit `crates/sage-core/src/config/provider.rs`
  - [ ] Import `TimeoutConfig` and `TimeoutDefaults`
  - [ ] Add `#[deprecated]` attribute to `timeout` field
  - [ ] Add `timeouts: Option<TimeoutConfig>` field
  - [ ] Implement `get_timeouts()` method
    - [ ] Check explicit timeouts first
    - [ ] Fall back to legacy timeout (with warning)
    - [ ] Fall back to provider defaults
  - [ ] Add `with_timeouts()` builder method
  - [ ] Update validation to accept both old and new formats
- [ ] Add unit tests
  - [ ] Test `get_timeouts()` with explicit config
  - [ ] Test legacy timeout conversion
  - [ ] Test provider default fallback
  - [ ] Test deprecation warning in debug mode
- [ ] Run tests: `cargo test -p sage-core config::provider`

### 2.2 Update ProviderDefaults

- [ ] Edit `crates/sage-core/src/config/provider.rs`
  - [ ] Update `ProviderDefaults::openai()` to use `TimeoutDefaults::openai()`
  - [ ] Update `ProviderDefaults::anthropic()` to use `TimeoutDefaults::anthropic()`
  - [ ] Update `ProviderDefaults::google()` to use `TimeoutDefaults::google()`
  - [ ] Update `ProviderDefaults::ollama()` to use `TimeoutDefaults::ollama()`
  - [ ] Update `ProviderDefaults::glm()` to use `TimeoutDefaults::glm()`
  - [ ] Remove hardcoded `timeout` values
- [ ] Verify compilation: `cargo check -p sage-core`

## Phase 3: LLMClient Integration ⏱️ Week 2

### 3.1 Update LLMClient Constructor

- [ ] Edit `crates/sage-core/src/llm/client.rs`
  - [ ] Import `RequestOptions` and `TimeoutConfig`
  - [ ] Update `new()` method
    - [ ] Call `config.get_timeouts()`
    - [ ] Use `connect_timeout()` on HTTP client builder
    - [ ] Use `timeout()` on HTTP client builder
  - [ ] Add debug logging for applied timeouts
- [ ] Test with different provider configs

### 3.2 Add chat_with_options Method

- [ ] Edit `crates/sage-core/src/llm/client.rs`
  - [ ] Add `chat_with_options()` method signature
  - [ ] Implement timeout resolution logic
    - [ ] Check for request override
    - [ ] Fall back to provider config
  - [ ] Create new HTTP client if override present
  - [ ] Add debug logging
  - [ ] Refactor existing `chat()` to call `chat_with_options()`
  - [ ] Update provider-specific methods to accept HTTP client
- [ ] Add unit/integration tests

### 3.3 Update Streaming Support

- [ ] Edit `crates/sage-core/src/llm/client.rs`
  - [ ] Add `chat_stream_with_options()` method
  - [ ] Use `effective_streaming()` timeout
  - [ ] Create HTTP client with streaming timeout
  - [ ] Refactor `chat_stream()` to call `chat_stream_with_options()`
  - [ ] Update provider streaming methods
- [ ] Test streaming with custom timeouts

### 3.4 Update Retry Logic

- [ ] Edit `crates/sage-core/src/llm/client.rs`
  - [ ] Update `execute_with_retry()` to use retry timeout
  - [ ] Apply timeout to individual retry attempts
  - [ ] Add logging for retry timeouts
- [ ] Test retry behavior

## Phase 4: Configuration Files ⏱️ Week 2

### 4.1 Update Example Configurations

- [ ] Edit `sage_config.json.example`
  - [ ] Add `timeouts` objects to all providers
  - [ ] Use human-readable format ("60s", "2m")
  - [ ] Add comments explaining each timeout type
  - [ ] Keep one example with legacy format (commented out)
- [ ] Verify config parses: `cargo run --bin sage -- --config sage_config.json.example --help`

### 4.2 Create Migration Examples

- [ ] Create `configs/timeout-migration-example.json`
  - [ ] Show before/after examples
  - [ ] Demonstrate all timeout types
  - [ ] Include edge cases
- [ ] Test parsing both old and new formats

## Phase 5: Testing ⏱️ Week 2-3

### 5.1 Unit Tests

- [ ] Test `TimeoutConfig` thoroughly
  - [ ] All constructor methods
  - [ ] Builder pattern
  - [ ] Effective timeout calculation
  - [ ] Merge behavior
  - [ ] Serialization/deserialization
- [ ] Test `RequestOptions`
  - [ ] Creation methods
  - [ ] Builder pattern
- [ ] Test provider config integration
  - [ ] Legacy conversion
  - [ ] Provider defaults
  - [ ] Explicit config
- [ ] Run: `cargo test -p sage-core`

### 5.2 Integration Tests

- [ ] Create `crates/sage-core/tests/timeout_integration_test.rs`
  - [ ] Test end-to-end timeout application
  - [ ] Test request override
  - [ ] Test streaming timeout
  - [ ] Test retry timeout
  - [ ] Test timeout failure scenarios
- [ ] Run: `cargo test -p sage-core --test timeout_integration_test`

### 5.3 Manual Testing

- [ ] Test with real OpenAI API
  - [ ] Normal request with default timeout
  - [ ] Request with extended timeout
  - [ ] Streaming with custom timeout
- [ ] Test with real Anthropic API
  - [ ] Default timeouts
  - [ ] Custom timeouts
- [ ] Test with Ollama (if available)
  - [ ] Verify longer timeouts work
- [ ] Test timeout failure scenarios
  - [ ] Verify timeout errors are caught
  - [ ] Verify retry logic works

### 5.4 Performance Testing

- [ ] Benchmark timeout resolution overhead
- [ ] Benchmark HTTP client creation overhead
- [ ] Verify no regression in request latency
- [ ] Run: `cargo bench` (if benchmarks exist)

## Phase 6: Documentation ⏱️ Week 3

### 6.1 API Documentation

- [ ] Add rustdoc comments to `TimeoutConfig`
  - [ ] Struct documentation
  - [ ] Field documentation
  - [ ] Method documentation
  - [ ] Usage examples
- [ ] Add rustdoc comments to `TimeoutDefaults`
- [ ] Add rustdoc comments to `RequestOptions`
- [ ] Add rustdoc comments to updated `ProviderConfig` methods
- [ ] Add rustdoc comments to new `LLMClient` methods
- [ ] Generate docs: `cargo doc --open -p sage-core`

### 6.2 User Documentation

- [ ] Update `docs/user-guide/configuration.md`
  - [ ] Add timeout configuration section
  - [ ] Explain timeout types
  - [ ] Show configuration examples
  - [ ] Explain provider defaults
- [ ] Create `docs/user-guide/timeout-tuning.md`
  - [ ] When to customize timeouts
  - [ ] How to choose timeout values
  - [ ] Common scenarios and recommendations

### 6.3 Migration Guide

- [ ] Create `docs/migration/v0.1-to-v0.2.md`
  - [ ] Explain changes
  - [ ] Show migration steps
  - [ ] Provide before/after examples
  - [ ] List breaking changes (if any)
  - [ ] Deprecation timeline

### 6.4 README Updates

- [ ] Update main `README.md` if needed
- [ ] Update `CHANGELOG.md`
  - [ ] Add v0.2.0 section
  - [ ] List new features
  - [ ] List deprecations
  - [ ] Note backward compatibility

## Phase 7: Code Review & Polish ⏱️ Week 3

### 7.1 Code Quality

- [ ] Run `cargo clippy` and fix warnings
- [ ] Run `cargo fmt` to format code
- [ ] Check for unwraps and handle errors
- [ ] Add `#[must_use]` where appropriate
- [ ] Verify all public APIs are documented

### 7.2 Review Checklist

- [ ] All code follows project style guide
- [ ] No unwrap() in production code
- [ ] Error messages are clear and actionable
- [ ] Logging is appropriate (debug for verbose, warn for issues)
- [ ] No unnecessary allocations
- [ ] Thread safety considered (if applicable)

### 7.3 Security Review

- [ ] No sensitive data in logs
- [ ] Timeout values validated (no zero or negative)
- [ ] No integer overflow in duration calculations
- [ ] Config parsing is safe

## Phase 8: Release Preparation ⏱️ Week 3-4

### 8.1 Version Bump

- [ ] Update version in `Cargo.toml` to 0.2.0
- [ ] Update dependency versions if needed
- [ ] Run `cargo update` to update lockfile

### 8.2 Release Notes

- [ ] Write release notes for v0.2.0
  - [ ] New features
  - [ ] Improvements
  - [ ] Deprecations
  - [ ] Migration guide link
  - [ ] Breaking changes (if any)
- [ ] Update `CHANGELOG.md`

### 8.3 Pre-Release Testing

- [ ] Run full test suite: `cargo test --all`
- [ ] Test example configs
- [ ] Test CLI with various providers
- [ ] Verify backward compatibility
- [ ] Test on different platforms (if applicable)

### 8.4 Release

- [ ] Create Git tag: `v0.2.0`
- [ ] Push tag to repository
- [ ] Create GitHub release
- [ ] Publish to crates.io (if applicable)
- [ ] Announce release

## Post-Release

### Monitor & Support

- [ ] Monitor for bug reports
- [ ] Respond to user questions
- [ ] Track timeout-related issues
- [ ] Gather feedback on default values

### Future Improvements

- [ ] Consider adaptive timeouts based on history
- [ ] Add timeout telemetry/metrics
- [ ] Implement percentile-based timeout adjustment
- [ ] Add timeout profiles (quick/normal/extended)

## Rollback Plan

If issues are found:

1. **Minor Issues**
   - [ ] Create hotfix branch
   - [ ] Fix issue
   - [ ] Release v0.2.1

2. **Major Issues**
   - [ ] Revert to v0.1.x
   - [ ] Document issues
   - [ ] Re-plan implementation
   - [ ] Release fixed version as v0.3.0

## Success Criteria

- [ ] All tests pass
- [ ] No performance regression
- [ ] Documentation is complete
- [ ] Migration path is clear
- [ ] Backward compatibility maintained
- [ ] User feedback is positive
- [ ] Timeout failures reduced in production

## Notes

- **Estimated Total Time**: 3-4 weeks
- **Critical Path**: Phases 1-3 (core implementation)
- **Risk Areas**: Backward compatibility, streaming timeout handling
- **Testing Focus**: Integration tests with real providers

## Sign-Off

- [ ] Developer approval
- [ ] Code review completed
- [ ] QA testing passed
- [ ] Documentation reviewed
- [ ] Release approved

---

**Last Updated**: 2025-12-22
**Implementation Status**: Not Started
**Target Release**: v0.2.0
