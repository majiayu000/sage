# Performance Issues Analysis

**Generated:** 2026-01-21
**Codebase:** Sage Agent

## Summary

This report identifies potential performance bottlenecks and optimization opportunities in the Sage Agent codebase.

---

## 1. Clone Operations (High Impact)

### Issue
Extensive use of `.clone()` throughout the codebase, particularly in hot paths.

### Locations
- `crates/sage-core/src/llm/` - Message cloning in LLM client
- `crates/sage-core/src/agent/` - State cloning in agent execution
- `crates/sage-core/src/tools/` - Tool call cloning

### Recommendations
1. Use `Arc<T>` for shared ownership instead of cloning large structures
2. Consider `Cow<'a, T>` for conditionally-owned data
3. Implement `Clone` only for types that genuinely need value semantics

---

## 2. String Allocations (Medium Impact)

### Issue
Frequent `.to_string()` and `.to_owned()` calls create unnecessary heap allocations.

### Key Patterns Found
```rust
// Inefficient
format!("Error: {}", msg).to_string()

// Should be
format!("Error: {}", msg)
```

### Locations
- Error handling code throughout
- Tool parameter extraction in `sage-tools`
- Configuration parsing in `sage-core/src/config/`

### Recommendations
1. Use `&str` references where possible
2. Leverage `String::from()` over `.to_string()` for literals
3. Consider `compact_str` crate for small strings

---

## 3. Synchronous File I/O (High Impact)

### Issue
Some file operations use `std::fs` instead of `tokio::fs`, blocking the async runtime.

### Locations
- `crates/sage-core/src/settings/loader.rs`
- Some tool implementations in `sage-tools`

### Recommendations
1. Replace `std::fs::*` with `tokio::fs::*`
2. Use `spawn_blocking` for unavoidable sync operations
3. Implement file operation batching

---

## 4. Lock Contention (Medium Impact)

### Issue
Use of `Mutex` and `RwLock` in shared state can cause contention in async code.

### Locations
- `crates/sage-core/src/tools/background_registry.rs`
- `crates/sage-core/src/session/` - Session management
- `crates/sage-core/src/mcp/registry.rs`

### Recommendations
1. Consider `parking_lot` crate for better mutex performance
2. Use `tokio::sync::RwLock` for async-aware locks
3. Minimize lock hold time
4. Consider lock-free data structures (e.g., `dashmap`)

---

## 5. Memory Allocations in Loops (Medium Impact)

### Issue
Vectors and strings created inside loops without pre-allocation.

### Example Pattern
```rust
// Inefficient
for item in items {
    let mut result = Vec::new();  // Reallocates each iteration
    // ...
}
```

### Recommendations
1. Use `Vec::with_capacity()` when size is known
2. Move allocations outside loops where possible
3. Reuse buffers across iterations

---

## 6. Streaming Response Handling (Medium Impact)

### Issue
SSE decoder and streaming clients may buffer excessively.

### Locations
- `crates/sage-core/src/llm/sse_decoder.rs`
- `crates/sage-core/src/llm/streaming.rs`

### Recommendations
1. Implement backpressure handling
2. Use bounded channels for streaming
3. Consider chunk-based processing

---

## 7. JSON Serialization (Low Impact)

### Issue
Frequent JSON serialization/deserialization in hot paths.

### Locations
- LLM request/response handling
- Tool execution results
- Session storage

### Recommendations
1. Use `simd-json` for faster parsing
2. Cache serialized representations where appropriate
3. Consider binary formats (e.g., MessagePack) for internal storage

---

## Priority Matrix

| Issue | Impact | Effort | Priority |
|-------|--------|--------|----------|
| Clone Operations | High | Medium | P1 |
| Sync File I/O | High | Low | P1 |
| Lock Contention | Medium | Medium | P2 |
| String Allocations | Medium | Low | P2 |
| Memory in Loops | Medium | Low | P2 |
| Streaming Handling | Medium | High | P3 |
| JSON Serialization | Low | Medium | P3 |

---

## Next Steps

1. Profile the codebase with `cargo-flamegraph`
2. Add benchmarks for critical paths
3. Address P1 issues first
4. Monitor with `tracing` spans for production profiling
