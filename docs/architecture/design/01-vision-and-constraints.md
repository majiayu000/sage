# Vision and Constraints Document

> Sage Agent - A Concurrent Asynchronous Code Agent System

## 1. Project Vision

### 1.1 What We Are Building

Sage is a **concurrent, asynchronous Code Agent system** built in Rust, designed to assist developers with software engineering tasks through intelligent automation.

**Core Value Proposition:**
- **Concurrent Execution**: Multiple tools and agents can run in parallel
- **Stream-First Design**: Real-time streaming responses with SSE support
- **Type Safety**: Leveraging Rust's type system for reliability
- **Extensibility**: MCP protocol support and plugin architecture

### 1.2 Differentiation from Claude Code

| Aspect | Claude Code | Sage |
|--------|-------------|------|
| Language | TypeScript/JavaScript | Rust |
| Runtime | Node.js | Native binary |
| Primary Use | CLI tool | CLI + SDK + Embeddable |
| Extensibility | Limited | Full MCP + Plugins |
| Target Users | End users | Developers building agents |

### 1.3 Success Criteria

```
+------------------------------------------------------------------+
|                    Success Metrics                                |
+------------------------------------------------------------------+
| Metric                  | Target           | Measurement          |
|-------------------------|------------------|----------------------|
| First token latency     | < 200ms          | P95 latency          |
| Concurrent tools        | >= 8             | Parallel execution   |
| Memory footprint        | < 500MB          | Peak per session     |
| API coverage            | 100%             | All major LLM APIs   |
| Stream support          | All providers    | SSE implementation   |
| Test coverage           | > 80%            | Unit + Integration   |
+------------------------------------------------------------------+
```

---

## 2. Hard Constraints

### 2.1 Technical Constraints

```yaml
Language:
  required: Rust (stable)
  edition: 2021
  msrv: 1.75.0  # Minimum Supported Rust Version

Runtime:
  async_runtime: tokio (full features)
  http_client: reqwest
  serialization: serde

Platform Support:
  tier1:
    - Linux (x86_64, aarch64)
    - macOS (x86_64, aarch64)
    - Windows (x86_64)
  tier2:
    - FreeBSD
    - WebAssembly (limited)
```

### 2.2 API Compatibility

```
+------------------------------------------------------------------+
|                  API Compatibility Matrix                         |
+------------------------------------------------------------------+
| Provider      | Chat | Stream | Tools | Vision | Required |
|---------------|------|--------|-------|--------|----------|
| Anthropic     |  Y   |   Y    |   Y   |   Y    |    Y     |
| OpenAI        |  Y   |   Y    |   Y   |   Y    |    Y     |
| Google Vertex |  Y   |   Y    |   Y   |   Y    |    N     |
| Azure OpenAI  |  Y   |   Y    |   Y   |   Y    |    N     |
| AWS Bedrock   |  Y   |   Y    |   Y   |   N    |    N     |
| Ollama        |  Y   |   Y    |   Y   |   N    |    N     |
+------------------------------------------------------------------+
```

### 2.3 Backward Compatibility

```rust
// Current public API that MUST be preserved:
// sage-core
pub use agent::{Agent, AgentExecution, AgentState};
pub use tools::{Tool, ToolCall, ToolResult, ToolRegistry};
pub use llm::{LLMClient, LLMProvider, Message};
pub use config::Config;

// sage-sdk
pub use SageAgentSDK;
```

---

## 3. MoSCoW Prioritization

### 3.1 Must Have (P0)

```
+------------------------------------------------------------------+
|                      Must Have Features                           |
+------------------------------------------------------------------+
| Feature                        | Rationale                        |
|--------------------------------|----------------------------------|
| Anthropic streaming            | Core functionality gap           |
| Tool parallel execution        | Performance requirement          |
| Event-driven architecture      | Enables real-time UI             |
| Cancellation propagation       | User control requirement         |
| Basic permission system        | Security baseline                |
+------------------------------------------------------------------+
```

### 3.2 Should Have (P1)

```
+------------------------------------------------------------------+
|                     Should Have Features                          |
+------------------------------------------------------------------+
| Feature                        | Rationale                        |
|--------------------------------|----------------------------------|
| MCP protocol support           | Extensibility                    |
| Specialized agents (Explore)   | Performance optimization         |
| Sandbox execution              | Security hardening               |
| Session persistence            | User experience                  |
| Retry with backoff             | Reliability                      |
+------------------------------------------------------------------+
```

### 3.3 Could Have (P2)

```
+------------------------------------------------------------------+
|                      Could Have Features                          |
+------------------------------------------------------------------+
| Feature                        | Rationale                        |
|--------------------------------|----------------------------------|
| Plugin hot-reload              | Developer experience             |
| Distributed agents             | Scalability                      |
| Custom model fine-tuning       | Advanced customization           |
| Web UI                         | Alternative interface            |
+------------------------------------------------------------------+
```

### 3.4 Won't Have (Out of Scope)

```
- Proprietary protocol support
- Real-time collaboration (multi-user)
- IDE integration (separate project)
- Model training/fine-tuning infrastructure
```

---

## 4. Stakeholders

### 4.1 Primary Users

```
+------------------------------------------------------------------+
|                        User Personas                              |
+------------------------------------------------------------------+
| Persona              | Needs                    | Interface       |
|----------------------|--------------------------|-----------------|
| CLI User             | Quick code assistance    | sage-cli        |
| SDK Developer        | Build custom agents      | sage-sdk        |
| Platform Integrator  | Embed in applications    | sage-core       |
| Tool Author          | Extend capabilities      | Tool trait      |
+------------------------------------------------------------------+
```

### 4.2 Development Team

```
Maintainers:
  - Core team: Architecture decisions, code review
  - Contributors: Feature implementation, bug fixes

Communication:
  - GitHub Issues: Bug reports, feature requests
  - Discussions: Design proposals, Q&A
```

---

## 5. Quality Attributes

### 5.1 Performance

```rust
// Performance requirements expressed as tests
#[test]
fn first_token_latency_under_200ms() {
    // P95 latency from request to first token < 200ms
}

#[test]
fn supports_8_concurrent_tools() {
    // Must handle 8 parallel tool executions
}

#[test]
fn memory_under_500mb_per_session() {
    // Peak memory usage < 500MB for single session
}
```

### 5.2 Reliability

```
+------------------------------------------------------------------+
|                    Reliability Requirements                       |
+------------------------------------------------------------------+
| Requirement              | Implementation                        |
|--------------------------|---------------------------------------|
| Graceful degradation     | Single tool failure doesn't crash     |
| Automatic retry          | 3 retries with exponential backoff    |
| State recovery           | Session can be restored after crash   |
| Error isolation          | Agent errors don't affect others      |
+------------------------------------------------------------------+
```

### 5.3 Security

```
+------------------------------------------------------------------+
|                     Security Requirements                         |
+------------------------------------------------------------------+
| Threat                   | Mitigation                            |
|--------------------------|---------------------------------------|
| Command injection        | Input validation + sandbox            |
| Path traversal           | Path canonicalization + whitelist     |
| API key exposure         | Memory encryption, no disk storage    |
| Malicious tool output    | Output sanitization                   |
+------------------------------------------------------------------+
```

### 5.4 Observability

```yaml
Logging:
  format: structured JSON
  library: tracing
  levels: [ERROR, WARN, INFO, DEBUG, TRACE]

Metrics:
  format: Prometheus
  endpoints:
    - /metrics

Tracing:
  format: OpenTelemetry
  exporters:
    - Jaeger
    - OTLP
```

---

## 6. Constraints Summary

```
+------------------------------------------------------------------+
|                      Constraints Matrix                           |
+------------------------------------------------------------------+
|                                                                   |
|  MUST                           | MUST NOT                        |
|  -------------------------------|--------------------------------|
|  - Use Rust stable              | - Break existing public API    |
|  - Support Anthropic streaming  | - Use unsafe without audit     |
|  - Implement event bus          | - Block async runtime          |
|  - Preserve Tool trait API      | - Expose secrets in logs       |
|                                 | - Skip permission checks       |
|                                                                   |
|  SHOULD                         | SHOULD NOT                      |
|  -------------------------------|--------------------------------|
|  - Support all major LLMs       | - Over-engineer solutions      |
|  - Implement MCP protocol       | - Add unnecessary dependencies |
|  - Add specialized agents       | - Couple components tightly    |
|  - Include comprehensive tests  | - Ignore backward compat       |
|                                                                   |
+------------------------------------------------------------------+
```

---

## 7. Document Control

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2024-01 | Architecture Team | Initial version |

---

## Next Steps

1. Review and approve this vision document
2. Proceed to Domain Model design
3. Design Concurrency Model (critical path)
4. Define Core Interfaces
