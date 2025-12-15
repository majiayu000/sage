# Core Components

## Component Overview

```
┌────────────────────────────────────────────────────────────┐
│                      sage-core                              │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                 Agent Layer                          │   │
│  │  ┌───────────┐  ┌────────────┐  ┌────────────────┐  │   │
│  │  │ BaseAgent │  │ Reactive   │  │  Lifecycle     │  │   │
│  │  │           │  │   Agent    │  │   Manager      │  │   │
│  │  └───────────┘  └────────────┘  └────────────────┘  │   │
│  └─────────────────────────────────────────────────────┘   │
│                            │                                │
│  ┌─────────────────────────▼───────────────────────────┐   │
│  │               Execution Layer                        │   │
│  │  ┌───────────┐  ┌────────────┐  ┌────────────────┐  │   │
│  │  │   LLM     │  │   Tool     │  │    Event       │  │   │
│  │  │  Client   │  │  Executor  │  │     Bus        │  │   │
│  │  └───────────┘  └────────────┘  └────────────────┘  │   │
│  └─────────────────────────────────────────────────────┘   │
│                            │                                │
│  ┌─────────────────────────▼───────────────────────────┐   │
│  │               Infrastructure Layer                   │   │
│  │  ┌───────────┐  ┌────────────┐  ┌────────────────┐  │   │
│  │  │Cancellation│ │   MCP      │  │    Error       │  │   │
│  │  │ Hierarchy │  │  Registry  │  │   Recovery     │  │   │
│  │  └───────────┘  └────────────┘  └────────────────┘  │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
└────────────────────────────────────────────────────────────┘
```

## Agent Layer

### BaseAgent
The core agent implementation that orchestrates task execution.

```rust
#[async_trait]
pub trait Agent: Send + Sync {
    async fn execute_task(&mut self, task: TaskMetadata) -> SageResult<AgentExecution>;
    async fn continue_execution(&mut self, execution: &mut AgentExecution, user_message: &str) -> SageResult<()>;
    fn config(&self) -> &Config;
    fn id(&self) -> Id;
}
```

**Responsibilities:**
- Task lifecycle management
- LLM interaction orchestration
- Tool execution coordination
- State tracking

### ReactiveAgent
Claude Code style lightweight agent for response-driven execution.

```rust
#[async_trait]
pub trait ReactiveAgent: Send + Sync {
    async fn process_request(&mut self, request: &str, context: Option<TaskMetadata>) -> SageResult<ReactiveResponse>;
    async fn continue_conversation(&mut self, previous: &ReactiveResponse, additional_input: &str) -> SageResult<ReactiveResponse>;
    fn config(&self) -> &Config;
}
```

**Responsibilities:**
- Single request-response cycles
- Conversation state management
- Batch tool execution

### LifecycleManager
Coordinates agent lifecycle events and hooks.

```rust
pub struct LifecycleManager {
    registry: Arc<LifecycleHookRegistry>,
    state: RwLock<AgentState>,
    initialized: RwLock<bool>,
}
```

**Responsibilities:**
- Hook registration and execution
- State transition validation
- Event notification

## Execution Layer

### LLMClient
Multi-provider LLM client with streaming support.

```rust
pub struct LLMClient {
    provider: LLMProvider,
    config: ProviderConfig,
    model_params: LLMModelParameters,
    http_client: reqwest::Client,
}
```

**Supported Providers:**
- OpenAI (GPT-4, GPT-3.5)
- Anthropic (Claude 3, Claude Sonnet 4)
- Google (Gemini)

**Features:**
- Streaming responses (SSE)
- Tool calling support
- Retry with backoff

### ToolExecutor
Sequential tool execution engine.

```rust
pub struct ToolExecutor {
    tools: HashMap<String, Arc<dyn Tool>>,
}
```

### BatchToolExecutor
Batch tool execution with concurrent execution support.

```rust
pub struct BatchToolExecutor {
    tools: DashMap<String, Arc<dyn Tool>>,
}
```

### ParallelToolExecutor
Full parallel execution with semaphore-based concurrency control.

```rust
pub struct ParallelToolExecutor {
    tools: DashMap<String, Arc<dyn Tool>>,
    config: ParallelExecutorConfig,
    global_semaphore: Arc<Semaphore>,
    type_semaphores: DashMap<String, Arc<Semaphore>>,
    sequential_lock: Arc<Mutex<()>>,
}
```

**Concurrency Modes:**
- `Parallel` - Full concurrent execution
- `Sequential` - One at a time
- `Limited(n)` - Limited concurrency
- `ExclusiveByType` - One per tool type

### EventBus
Pub/sub event distribution system.

```rust
pub struct EventBus {
    sender: broadcast::Sender<Event>,
}

pub enum Event {
    StreamConnected,
    StreamDisconnected,
    TextDelta(String),
    ToolCallStart { id: String, name: String },
    ToolCallComplete { id: String, result: ToolResult },
    AgentStateChanged { from: AgentState, to: AgentState },
    Error(String),
}
```

## Infrastructure Layer

### CancellationHierarchy
Hierarchical cancellation token management.

```rust
pub struct CancellationHierarchy {
    root: CancellationToken,
    sessions: DashMap<SessionId, CancellationToken>,
    agents: DashMap<AgentId, CancellationToken>,
    tools: DashMap<ToolCallId, CancellationToken>,
}
```

**Hierarchy:**
```
Root Token
    └── Session Token
            └── Agent Token
                    └── Tool Call Token
```

### McpRegistry
Registry for MCP server connections.

```rust
pub struct McpRegistry {
    clients: DashMap<String, Arc<McpClient>>,
    tool_mapping: DashMap<String, String>,
    resource_mapping: DashMap<String, String>,
    prompt_mapping: DashMap<String, String>,
}
```

**Capabilities:**
- Server lifecycle management
- Tool discovery and mapping
- Resource access
- Prompt templates

### Error Recovery

#### RetryPolicy
```rust
pub struct RetryPolicy {
    config: RetryConfig,
    backoff: Box<dyn BackoffStrategy>,
}
```

#### CircuitBreaker
```rust
pub struct CircuitBreaker {
    state: RwLock<CircuitState>,
    failure_count: AtomicU32,
    success_count: AtomicU32,
    config: CircuitBreakerConfig,
}
```

#### TaskSupervisor
```rust
pub struct TaskSupervisor {
    policy: SupervisionPolicy,
    restart_count: AtomicU32,
    window_start: RwLock<Instant>,
}
```
