# Sage Agent Architecture

This directory contains the system architecture documentation for Sage Agent - a concurrent async code agent built in Rust.

## Document Index

| Document | Description |
|----------|-------------|
| [01-overview.md](./01-overview.md) | System overview and high-level architecture |
| [02-core-components.md](./02-core-components.md) | Core components and their responsibilities |
| [03-concurrency.md](./03-concurrency.md) | Concurrency model and async patterns |
| [04-tools-system.md](./04-tools-system.md) | Tool execution and permission system |
| [05-mcp-integration.md](./05-mcp-integration.md) | Model Context Protocol integration |
| [06-agent-lifecycle.md](./06-agent-lifecycle.md) | Agent lifecycle hooks and management |
| [07-error-recovery.md](./07-error-recovery.md) | Error recovery and fault tolerance |
| [08-builder-pattern.md](./08-builder-pattern.md) | SageBuilder and component assembly |

## Architecture Principles

1. **Async-First**: Built on Tokio for non-blocking I/O
2. **Composable**: Small, focused components that compose well
3. **Extensible**: Plugin-friendly architecture with trait-based abstractions
4. **Resilient**: Comprehensive error recovery and graceful degradation
5. **Observable**: Event-driven architecture for monitoring and debugging

## Tech Stack

- **Runtime**: Tokio async runtime
- **Concurrency**: tokio-util CancellationToken, DashMap, Semaphore
- **Serialization**: serde, serde_json
- **HTTP**: reqwest with async
- **CLI**: clap
- **Error Handling**: thiserror, anyhow

## Module Structure

```
sage-core/
├── agent/           # Agent execution engine
│   ├── base.rs      # Base Agent trait
│   ├── lifecycle.rs # Lifecycle hooks
│   └── reactive_agent.rs
├── builder.rs       # SageBuilder
├── cache/           # LLM response caching
├── concurrency/     # Cancellation hierarchy
├── config/          # Configuration management
├── error.rs         # Error types
├── events/          # Event bus system
├── llm/             # LLM client implementations
│   ├── client.rs    # Multi-provider client
│   ├── streaming.rs # SSE streaming
│   └── sse_decoder.rs
├── mcp/             # Model Context Protocol
│   ├── client.rs    # MCP client
│   ├── registry.rs  # Server registry
│   └── transport/   # Stdio/HTTP transports
├── recovery/        # Error recovery
│   ├── backoff.rs   # Backoff strategies
│   ├── retry.rs     # Retry policies
│   ├── circuit_breaker.rs
│   └── supervisor.rs
├── tools/           # Tool system
│   ├── base.rs      # Tool trait
│   ├── executor.rs  # Sequential executor
│   ├── batch_executor.rs
│   ├── parallel_executor.rs
│   └── permission.rs
├── trajectory/      # Execution recording
└── ui/              # Terminal UI
```
