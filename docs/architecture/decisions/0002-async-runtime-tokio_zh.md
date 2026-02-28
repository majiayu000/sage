# ADR-0002: Tokio as Async Runtime

## Status

Accepted

## Context

Sage Agent requires asynchronous I/O for:
- LLM API calls (HTTP streaming responses)
- Concurrent tool execution
- File system operations
- Signal handling (Ctrl+C, graceful shutdown)
- Background task management
- Real-time UI updates during streaming

We needed to choose an async runtime that provides:
1. Mature and well-maintained ecosystem
2. Full-featured async I/O (network, filesystem, timers)
3. Multi-threaded task scheduling
4. Stream processing capabilities
5. Signal handling support
6. Good error messages and debugging tools
7. Strong community and documentation

## Decision

We chose **Tokio** as the async runtime with the `full` feature set.

### Configuration

```toml
# Workspace dependencies (Cargo.toml)
tokio = { version = "1.0", features = ["full"] }
tokio-util = "0.7"
tokio-stream = { version = "0.1", features = ["io-util"] }
futures = "0.3"

# Additional async utilities
async-trait = "0.1"
```

### Usage Patterns

1. **Runtime Initialization**:
   ```rust
   #[tokio::main]
   async fn main() {
       // Application entry point
   }
   ```

2. **Concurrent Operations**:
   ```rust
   // Parallel tool execution
   let results = tokio::try_join!(
       tool1.execute(&call1),
       tool2.execute(&call2),
   )?;
   ```

3. **Stream Processing**:
   ```rust
   // LLM streaming responses
   let mut stream = client.chat_stream(messages, tools).await?;
   while let Some(chunk) = stream.next().await {
       // Process streaming chunk
   }
   ```

4. **Timeouts and Cancellation**:
   ```rust
   // Tool execution timeout
   tokio::time::timeout(duration, tool.execute(call)).await??;
   ```

5. **Signal Handling**:
   ```rust
   use signal_hook_tokio::Signals;
   let signals = Signals::new(&[SIGINT, SIGTERM])?;
   tokio::spawn(async move {
       // Handle signals
   });
   ```

## Consequences

### Positive

1. **Ecosystem Maturity**:
   - Most popular Rust async runtime
   - Extensive ecosystem of compatible libraries
   - `reqwest` (HTTP client) builds on Tokio

2. **Full Feature Set**:
   - Multi-threaded scheduler
   - Async file I/O (`tokio::fs`)
   - Timer and interval support
   - Signal handling (`signal-hook-tokio`)
   - Process management

3. **Performance**:
   - Work-stealing scheduler optimizes CPU usage
   - Efficient for I/O-bound workloads (LLM API calls)
   - Supports both single and multi-threaded runtimes

4. **Developer Experience**:
   - Excellent error messages
   - `tokio-console` for runtime introspection
   - Well-documented with many examples
   - Good integration with `tracing` for logging

5. **Stream Processing**:
   - `tokio-stream` and `futures::stream` for composable streaming
   - Essential for LLM response streaming
   - Supports backpressure and buffering

6. **Tool Execution**:
   - Parallel execution of multiple tools
   - Background task management
   - Graceful cancellation on interrupts

### Negative

1. **Binary Size**: Tokio adds ~1MB to binary size with `full` features
2. **Compile Time**: Significant impact on initial compilation
3. **Complexity**: Async programming has steeper learning curve
4. **Runtime Overhead**: Small overhead compared to blocking I/O (acceptable for our use case)

### Migration Considerations

1. **Async All the Way**:
   - All tool execution is async (`#[async_trait]`)
   - LLM providers implement async methods
   - File operations use `tokio::fs` where beneficial

2. **Blocking Operations**:
   - CPU-intensive operations run in `spawn_blocking`
   - Prevents blocking the async runtime

3. **Synchronization**:
   - Use `tokio::sync` primitives (Mutex, RwLock, mpsc channels)
   - `parking_lot` for synchronous code paths

### Alternative Approaches Considered

1. **async-std**:
   - Rejected: Smaller ecosystem, less tooling
   - Pros: Similar API to std library
   - Cons: Less widely adopted, fewer compatible libraries

2. **smol**:
   - Rejected: Lightweight but less feature-complete
   - Pros: Smaller binary size
   - Cons: Less ecosystem support, manual runtime management

3. **Synchronous (blocking) I/O**:
   - Rejected: Poor fit for streaming and concurrent operations
   - Pros: Simpler programming model
   - Cons: Cannot efficiently handle LLM streaming, limits concurrency

4. **Custom runtime**:
   - Rejected: Significant development and maintenance burden
   - Not justified given Tokio's maturity

## Performance Characteristics

- **LLM Streaming**: Tokio's stream processing enables low-latency response rendering
- **Concurrent Tools**: Work-stealing scheduler efficiently distributes parallel tool execution
- **Signal Handling**: Async signal handlers enable graceful shutdown without blocking
- **HTTP Requests**: `reqwest` with Tokio backend provides robust retry and timeout handling

## References

- [Tokio Documentation](https://tokio.rs/)
- `Cargo.toml`: Workspace dependencies
- `crates/sage-core/src/llm/client.rs`: Async LLM client implementation
- `crates/sage-core/src/tools/executor.rs`: Async tool execution
- `crates/sage-cli/src/main.rs`: Tokio runtime initialization
