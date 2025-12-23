# ADR-0004: LLM Provider Abstraction

## Status

Accepted

## Context

Sage Agent integrates with multiple LLM providers (OpenAI, Anthropic, Google, Azure, etc.). Each provider has:

1. **Different APIs**: REST endpoints, request/response formats, authentication
2. **Different capabilities**: Function calling, streaming, context length, pricing
3. **Different error handling**: Rate limits, timeouts, quota exhaustion
4. **Different message formats**: Anthropic's system messages vs OpenAI's role-based

We needed an abstraction that:
- **Hides provider differences** from the agent execution layer
- **Enables easy addition** of new providers
- **Supports both streaming and non-streaming** modes
- **Handles retries and fallbacks** across providers
- **Preserves provider-specific optimizations** (e.g., prompt caching)
- **Provides unified error handling** with provider context

## Decision

We designed a **two-layer abstraction**: a unified trait with provider-specific implementations wrapped in an enum.

### Layer 1: Unified Provider Trait

```rust
#[async_trait]
pub trait LLMProviderTrait: Send + Sync {
    /// Non-streaming chat completion
    async fn chat(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMResponse>;

    /// Streaming chat completion
    async fn chat_stream(
        &self,
        messages: &[LLMMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LLMStream>;
}
```

### Layer 2: Provider Enum

```rust
pub enum ProviderInstance {
    OpenAI(OpenAIProvider),
    Anthropic(AnthropicProvider),
    Google(GoogleProvider),
    Azure(AzureProvider),
    OpenRouter(OpenRouterProvider),
    Ollama(OllamaProvider),
    Doubao(DoubaoProvider),
    Glm(GlmProvider),
}

#[async_trait]
impl LLMProviderTrait for ProviderInstance {
    async fn chat(...) -> SageResult<LLMResponse> {
        match self {
            Self::OpenAI(p) => p.chat(messages, tools).await,
            Self::Anthropic(p) => p.chat(messages, tools).await,
            // ... other providers
        }
    }
}
```

### Unified Message Format

All providers work with a common message representation:
```rust
pub struct LLMMessage {
    pub role: MessageRole,      // System, User, Assistant
    pub content: Vec<Content>,  // Text, ToolUse, ToolResult
    pub metadata: Option<MessageMetadata>,
}

pub enum Content {
    Text { text: String },
    ToolUse { id: String, name: String, input: Value },
    ToolResult { tool_use_id: String, output: String },
}
```

### Provider Configuration

Each provider is configured via `ProviderConfig`:
```rust
pub struct ProviderConfig {
    pub api_key: String,
    pub base_url: Option<String>,
    pub headers: HashMap<String, String>,
    pub timeouts: TimeoutConfig,
    // Provider-specific fields (Azure deployment, etc.)
}
```

### Client Architecture

The `LLMClient` wraps a `ProviderInstance`:
```rust
pub struct LLMClient {
    provider: LLMProvider,           // Enum identifying the provider
    config: ProviderConfig,          // Configuration
    model_params: ModelParameters,   // Temperature, max_tokens, etc.
    provider_instance: ProviderInstance, // The actual provider
}

impl LLMClient {
    pub async fn chat(&self, ...) -> SageResult<LLMResponse> {
        // Apply rate limiting
        rate_limiter::acquire(&self.provider).await;

        // Delegate to provider instance
        let result = self.provider_instance.chat(messages, tools).await;

        // Handle provider-specific errors (quotas, rate limits)
        match result {
            Err(SageError::QuotaExceeded) => {
                // Fallback logic
            }
            _ => result
        }
    }
}
```

## Consequences

### Positive

1. **Provider Isolation**:
   - Agent code doesn't know which provider it's using
   - Easy to swap providers via configuration
   - Each provider has isolated implementation (no cross-contamination)

2. **Unified Error Handling**:
   - All providers return `SageResult<LLMResponse>`
   - Common error types (rate limit, quota, timeout)
   - Provider-specific context preserved in error messages

3. **Message Format Abstraction**:
   - Agent works with unified `LLMMessage` format
   - Provider-specific conversion happens in provider implementations
   - Anthropic's system messages, OpenAI's roles, etc. are hidden

4. **Streaming Support**:
   - Both streaming and non-streaming share same trait
   - `LLMStream` provides unified async iterator
   - Provider-specific SSE/streaming handled internally

5. **Easy Provider Addition**:
   ```rust
   // Add new provider in 3 steps:
   // 1. Implement LLMProviderTrait
   pub struct NewProvider { ... }

   #[async_trait]
   impl LLMProviderTrait for NewProvider { ... }

   // 2. Add to ProviderInstance enum
   enum ProviderInstance {
       // ...
       New(NewProvider),
   }

   // 3. Add match arm
   ```

6. **Provider-Specific Optimizations**:
   - Anthropic: Prompt caching headers
   - OpenAI: Function calling format
   - Google: Gemini-specific features
   - Each provider controls its own request/response transformation

7. **Retry and Fallback**:
   - `LLMClient` can retry on transient errors
   - Multi-provider fallback for quota exhaustion
   - Exponential backoff with jitter

8. **Cost Tracking**:
   - `LLMResponse` includes token usage
   - Provider-specific pricing calculation
   - Unified interface for cost estimation

### Negative

1. **Enum Dispatch Overhead**:
   - `match` statement for each provider call
   - Could use trait objects (`Box<dyn LLMProviderTrait>`) but enum is clearer
   - Negligible overhead for I/O-bound LLM calls

2. **Feature Parity Challenges**:
   - Not all providers support all features (e.g., Ollama doesn't support function calling)
   - Need runtime capability detection
   - Some features require provider-specific handling

3. **Message Format Conversion**:
   - Each provider must convert `LLMMessage` to its native format
   - Potential information loss if provider doesn't support all content types
   - Need careful testing for each provider

4. **Maintenance Burden**:
   - New LLM features require updating all providers
   - API changes in one provider require isolated fixes
   - Need to keep up with 8+ provider APIs

5. **Abstraction Leaks**:
   - Provider-specific rate limits visible in errors
   - Some features only work with certain providers (e.g., Anthropic's caching)
   - Configuration needs provider-specific fields

### Design Patterns

1. **Strategy Pattern**:
   - Different providers implement the same interface
   - Runtime selection via configuration

2. **Adapter Pattern**:
   - Each provider adapts its native API to `LLMProviderTrait`
   - Converts between unified and provider-specific formats

3. **Facade Pattern**:
   - `LLMClient` provides simple interface to complex provider ecosystem
   - Hides retry logic, rate limiting, fallbacks

### Provider-Specific Implementations

#### Anthropic
```rust
impl AnthropicProvider {
    async fn chat(&self, ...) -> SageResult<LLMResponse> {
        // Convert LLMMessage to Anthropic format
        let (system, messages) = self.convert_messages(messages);

        // Add prompt caching headers
        let headers = self.build_headers_with_caching();

        // Make API request
        let response = self.http_client
            .post(&self.config.base_url)
            .headers(headers)
            .json(&request)
            .send()
            .await?;

        // Parse response
        self.parse_response(response).await
    }
}
```

#### OpenAI
```rust
impl OpenAIProvider {
    async fn chat(&self, ...) -> SageResult<LLMResponse> {
        // Convert to OpenAI message format
        let openai_messages = self.convert_to_openai_format(messages);

        // Handle function calling
        let functions = tools.map(|t| self.convert_tools_to_functions(t));

        // Make API request
        // ...
    }
}
```

### Streaming Implementation

```rust
pub struct LLMStream {
    inner: Pin<Box<dyn Stream<Item = SageResult<StreamChunk>> + Send>>,
}

// Provider-specific streaming
impl AnthropicProvider {
    async fn chat_stream(&self, ...) -> SageResult<LLMStream> {
        let response = self.http_client.post(...).send().await?;
        let stream = response.bytes_stream();

        // Parse SSE events
        let parsed_stream = stream.map(|chunk| {
            self.parse_sse_event(chunk)
        });

        Ok(LLMStream::new(parsed_stream))
    }
}
```

### Alternative Approaches Considered

1. **Trait Objects Only**:
   ```rust
   pub struct LLMClient {
       provider: Box<dyn LLMProviderTrait>,
   }
   ```
   - Rejected: Less clear which provider is in use
   - Harder to match on provider for fallback logic

2. **No Abstraction (Direct Provider Calls)**:
   ```rust
   match provider_type {
       "openai" => openai_client.chat(...),
       "anthropic" => anthropic_client.chat(...),
   }
   ```
   - Rejected: Agent code tied to specific providers
   - Duplicates retry/fallback logic

3. **Single Provider Only**:
   - Rejected: Limits user choice
   - No fallback on quota/rate limits
   - Can't leverage different providers' strengths

4. **External Adapter Service**:
   - Run a proxy that normalizes all providers
   - Rejected: Adds deployment complexity, latency
   - Current approach is more efficient

5. **Code Generation from OpenAPI Specs**:
   - Auto-generate provider clients from API specs
   - Rejected: Specs don't capture all nuances
   - Hand-written adapters give more control

## Migration and Compatibility

### Adding a New Provider

1. Create provider module: `crates/sage-core/src/llm/providers/new_provider.rs`
2. Implement `LLMProviderTrait`
3. Handle message format conversion
4. Add to `ProviderInstance` enum
5. Update `LLMClient::new()` to construct provider
6. Add configuration example to docs

### Upgrading Provider APIs

When a provider changes its API:
1. Update only that provider's implementation
2. Other providers unaffected
3. Add tests for new API behavior
4. Update docs if new features available

## Performance Characteristics

- **HTTP Client Reuse**: Each provider holds a `reqwest::Client` instance
- **Connection Pooling**: `reqwest` handles connection reuse
- **Timeout Configuration**: Per-provider timeouts via `TimeoutConfig`
- **Rate Limiting**: Global rate limiter prevents quota exhaustion
- **Retries**: Exponential backoff with configurable max attempts

## References

- `crates/sage-core/src/llm/providers/provider_trait.rs`: Trait definition
- `crates/sage-core/src/llm/client.rs`: LLM client implementation
- `crates/sage-core/src/llm/providers/`: Individual provider implementations
- `crates/sage-core/src/llm/messages.rs`: Unified message format
- `crates/sage-core/src/llm/streaming.rs`: Streaming abstraction
- `crates/sage-core/src/llm/provider_fallback.rs`: Multi-provider fallback logic
