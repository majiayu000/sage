# Timeout Configuration Diagrams

## 1. Timeout Resolution Hierarchy

```mermaid
graph TD
    A[Request Made] --> B{Request Options<br/>with timeout?}
    B -->|Yes| C[Use Request<br/>Timeout Override]
    B -->|No| D{Provider Config<br/>has timeouts?}
    D -->|Yes| E[Use Provider<br/>Config Timeouts]
    D -->|No| F{Legacy timeout<br/>field set?}
    F -->|Yes| G[Convert Legacy<br/>Timeout to TimeoutConfig]
    F -->|No| H{Provider-Specific<br/>Defaults Exist?}
    H -->|Yes| I[Use Provider<br/>Defaults]
    H -->|No| J[Use Global<br/>Defaults]

    C --> K[Execute Request]
    E --> K
    G --> K
    I --> K
    J --> K

    style C fill:#90EE90
    style E fill:#87CEEB
    style G fill:#FFE4B5
    style I fill:#DDA0DD
    style J fill:#F0E68C
```

## 2. Timeout Types and Application

```mermaid
graph LR
    A[HTTP Request] --> B[Connection Phase]
    B --> C[Request Sent]
    C --> D[Response Headers]
    D --> E[Response Body]

    B -.->|connect timeout| B1[10s]
    C -.->|read timeout| C1[30s per read]
    E -.->|total timeout| E1[60s overall]

    F[Streaming Request] --> G[Connection Phase]
    G --> H[First Chunk]
    H --> I[Subsequent Chunks]
    I --> J[Stream Complete]

    G -.->|connect timeout| G1[10s]
    I -.->|streaming timeout| I1[120s total]

    style B1 fill:#FFB6C1
    style C1 fill:#FFB6C1
    style E1 fill:#FFB6C1
    style G1 fill:#98FB98
    style I1 fill:#98FB98
```

## 3. Configuration Sources Flow

```mermaid
flowchart TD
    A[User Writes Config] --> B[sage_config.json]
    B --> C[ConfigLoader]
    C --> D[ProviderConfig]

    D --> E{timeouts field<br/>present?}
    E -->|Yes| F[TimeoutConfig]
    E -->|No| G{timeout field<br/>present?}

    G -->|Yes| H[Legacy Conversion]
    H --> F
    G -->|No| I[TimeoutDefaults::for_provider]
    I --> F

    J[Runtime Request] --> K[RequestOptions]
    K --> L{timeout_override?}
    L -->|Yes| M[Override TimeoutConfig]
    L -->|No| F

    M --> N[LLMClient]
    F --> N
    N --> O[HTTP Client]

    style B fill:#E6E6FA
    style F fill:#98FB98
    style M fill:#FFD700
    style N fill:#87CEEB
```

## 4. Provider-Specific Defaults

```mermaid
graph TB
    A[Provider Type] --> B[OpenAI]
    A --> C[Anthropic]
    A --> D[Google]
    A --> E[Ollama]

    B --> B1[Total: 60s<br/>Connect: 10s<br/>Read: 30s<br/>Streaming: 120s]
    C --> C1[Total: 60s<br/>Connect: 10s<br/>Read: 30s<br/>Streaming: 120s]
    D --> D1[Total: 90s<br/>Connect: 15s<br/>Read: 45s<br/>Streaming: 180s]
    E --> E1[Total: 300s<br/>Connect: 5s<br/>Read: 60s<br/>Streaming: 600s]

    style B1 fill:#FFE4E1
    style C1 fill:#E0FFE0
    style D1 fill:#E0F0FF
    style E1 fill:#FFF0E0
```

## 5. Request Execution with Timeouts

```mermaid
sequenceDiagram
    participant User
    participant Agent
    participant LLMClient
    participant HTTPClient
    participant Provider

    User->>Agent: execute_task()
    Agent->>LLMClient: chat(messages, tools)
    Note over LLMClient: Resolve timeout config
    LLMClient->>LLMClient: get_timeouts()

    alt Request has timeout override
        LLMClient->>LLMClient: Use RequestOptions.timeout_override
    else Provider has explicit config
        LLMClient->>LLMClient: Use ProviderConfig.timeouts
    else Legacy timeout set
        LLMClient->>LLMClient: Convert legacy timeout
    else Use defaults
        LLMClient->>LLMClient: Use TimeoutDefaults::for_provider()
    end

    LLMClient->>HTTPClient: POST with timeouts
    activate HTTPClient

    HTTPClient->>Provider: Connect (10s timeout)
    Provider-->>HTTPClient: Connection established
    HTTPClient->>Provider: Send request
    Provider-->>HTTPClient: Stream response (30s read timeout)

    deactivate HTTPClient
    HTTPClient-->>LLMClient: Response
    LLMClient-->>Agent: LLMResponse
    Agent-->>User: Result
```

## 6. Timeout Failure and Retry Flow

```mermaid
stateDiagram-v2
    [*] --> SendRequest
    SendRequest --> Connecting: Start

    Connecting --> Connected: Success
    Connecting --> ConnectTimeout: timeout (10s)

    Connected --> SendingData
    SendingData --> WaitingResponse

    WaitingResponse --> ReceivingData: Headers received
    WaitingResponse --> ReadTimeout: timeout (30s)

    ReceivingData --> Success: Complete
    ReceivingData --> ReadTimeout: timeout (30s)
    ReceivingData --> TotalTimeout: timeout (60s)

    ConnectTimeout --> Retry: attempt < max_retries
    ReadTimeout --> Retry: attempt < max_retries
    TotalTimeout --> Retry: attempt < max_retries

    ConnectTimeout --> Failed: attempts exhausted
    ReadTimeout --> Failed: attempts exhausted
    TotalTimeout --> Failed: attempts exhausted

    Retry --> Wait: exponential backoff
    Wait --> SendRequest: 2^attempt seconds

    Success --> [*]
    Failed --> [*]

    note right of Retry
        Retry timeout: 45s
        (shorter than total)
    end note
```

## 7. Configuration File Structure

```mermaid
classDiagram
    class Config {
        +String default_provider
        +HashMap~String, ProviderConfig~ model_providers
        +ToolConfig tools
        +TrajectoryConfig trajectory
    }

    class ProviderConfig {
        +String name
        +Option~String~ api_key
        +Option~u64~ timeout [deprecated]
        +Option~TimeoutConfig~ timeouts
        +Option~u32~ max_retries
    }

    class TimeoutConfig {
        +Duration total
        +Duration connect
        +Duration read
        +Option~Duration~ streaming
        +Option~Duration~ retry
        +effective_streaming() Duration
        +effective_retry() Duration
    }

    class RequestOptions {
        +Option~TimeoutConfig~ timeout_override
        +Option~u32~ priority
        +HashMap~String, String~ metadata
    }

    class LLMClient {
        +LLMProvider provider
        +ProviderConfig config
        +Client http_client
        +chat(messages, tools) LLMResponse
        +chat_with_options(messages, tools, options) LLMResponse
    }

    Config "1" --> "*" ProviderConfig
    ProviderConfig "1" --> "0..1" TimeoutConfig
    LLMClient "1" --> "1" ProviderConfig
    LLMClient ..> RequestOptions: uses
    RequestOptions "1" --> "0..1" TimeoutConfig: overrides
```

## 8. Migration Path (Legacy to New)

```mermaid
graph LR
    A[Old Config] --> B{Has 'timeout'<br/>field?}
    B -->|Yes| C[Read timeout value]
    C --> D[Create TimeoutConfig<br/>with total=timeout]
    D --> E[Set other fields<br/>to defaults]
    E --> F[New TimeoutConfig]

    B -->|No| G[Use provider<br/>defaults]
    G --> F

    H[New Config] --> I[Define timeouts<br/>object]
    I --> J[Set granular<br/>timeouts]
    J --> F

    style A fill:#FFE4B5
    style H fill:#90EE90
    style F fill:#87CEEB
```

## 9. Timeout Decision Tree

```mermaid
graph TD
    A[Need to make LLM request] --> B{What type?}

    B -->|Simple query| C[Short timeout<br/>15-30s]
    B -->|Normal operation| D[Standard timeout<br/>60s]
    B -->|Complex reasoning| E[Extended timeout<br/>120-300s]
    B -->|Streaming| F[Streaming timeout<br/>120-600s]

    C --> G{Provider?}
    D --> G
    E --> G
    F --> G

    G -->|Cloud API| H[Use configured<br/>or provider default]
    G -->|Local Model| I[Use extended<br/>timeout]

    H --> J[Create RequestOptions<br/>if override needed]
    I --> J

    J --> K[Execute with<br/>resolved timeout]

    style C fill:#FFB6C1
    style D fill:#87CEEB
    style E fill:#FFA500
    style F fill:#9370DB
```

## 10. Backward Compatibility

```mermaid
flowchart TD
    A[Existing Config] --> B{Check version}

    B -->|v0.1.x| C[Has 'timeout' field]
    B -->|v0.2.x+| D[Has 'timeouts' object]

    C --> E[Automatic conversion]
    E --> F{Deprecation warning}
    F -->|Development| G[Log warning]
    F -->|Production| H[Silent conversion]

    G --> I[Use converted config]
    H --> I

    D --> J[Direct usage]
    J --> I

    I --> K[Runtime TimeoutConfig]

    style C fill:#FFE4B5
    style D fill:#90EE90
    style K fill:#87CEEB
```

## Legend

- 🟢 Green: Recommended/New approach
- 🔵 Blue: Standard/Normal flow
- 🟡 Yellow: Legacy/Deprecated
- 🟣 Purple: Special cases
- 🔴 Red: Error/Failed state

## Quick Reference Table

| Timeout Type | Default | OpenAI | Anthropic | Google | Ollama | When Applied |
|--------------|---------|--------|-----------|--------|--------|--------------|
| **Total**    | 60s     | 60s    | 60s       | 90s    | 300s   | Entire request |
| **Connect**  | 10s     | 10s    | 10s       | 15s    | 5s     | TCP + TLS handshake |
| **Read**     | 30s     | 30s    | 30s       | 45s    | 60s    | Between receiving bytes |
| **Streaming**| 120s    | 120s   | 120s      | 180s   | 600s   | SSE/streaming responses |
| **Retry**    | 45s     | 45s    | 45s       | 60s    | 180s   | Individual retry attempt |

## Example Scenarios

### Scenario 1: Quick Question
```
User: "What is 2+2?"
Timeout: 15s total (request override)
Rationale: Simple query, expect fast response
```

### Scenario 2: Code Analysis
```
User: "Analyze this 1000-line codebase"
Timeout: 120s total, 180s streaming
Rationale: Complex task, may need extended processing
```

### Scenario 3: Local Model
```
Provider: Ollama
Timeout: 300s total, 600s streaming
Rationale: Local inference is slower, needs more time
```

### Scenario 4: Production API
```
Provider: Anthropic/OpenAI
Timeout: 60s total (default)
Rationale: Production APIs should be fast and reliable
```
