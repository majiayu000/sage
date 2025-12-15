# Domain Model

> Core entities and relationships in the Sage Agent system

## 1. Domain Overview

```
+=====================================================================+
|                         SAGE DOMAIN MODEL                            |
+=====================================================================+
|                                                                      |
|  +-------------+                                                     |
|  |   Session   |  The root aggregate - represents a user's          |
|  +------+------+  complete interaction session                       |
|         |                                                            |
|         | 1:N                                                        |
|         v                                                            |
|  +-------------+                                                     |
|  |    Agent    |  Autonomous entities that execute tasks             |
|  +------+------+                                                     |
|         |                                                            |
|    +----+----+                                                       |
|    |         |                                                       |
|    v         v                                                       |
| +------+  +------+                                                   |
| | Task |  | Tool |  Work unit and capability                        |
| +------+  +------+                                                   |
|                                                                      |
+======================================================================+
```

---

## 2. Core Entities

### 2.1 Session (Root Aggregate)

```rust
/// Session represents a complete user interaction session.
/// It is the root aggregate that owns all other entities.
///
/// Invariants:
/// - A session has exactly one active message stream
/// - A session can have 0..N agents
/// - Session ID is globally unique
pub struct Session {
    /// Unique identifier for this session
    id: SessionId,

    /// Current session state
    state: SessionState,

    /// Message stream for real-time communication
    message_stream: MessageStream,

    /// Active agents in this session
    agents: Vec<AgentHandle>,

    /// Conversation history
    messages: Vec<Message>,

    /// Session configuration
    config: SessionConfig,

    /// Creation timestamp
    created_at: DateTime<Utc>,

    /// Last activity timestamp
    last_activity: DateTime<Utc>,
}

/// Session states
pub enum SessionState {
    /// Session is initializing
    Initializing,
    /// Session is active and ready
    Active,
    /// Session is processing a request
    Processing,
    /// Session is paused (waiting for user)
    Paused,
    /// Session has ended normally
    Ended,
    /// Session ended due to error
    Error(SessionError),
}

/// Unique session identifier
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SessionId(Uuid);
```

### 2.2 Agent (Core Entity)

```rust
/// Agent is an autonomous entity that executes tasks.
/// Each agent has a specific type that determines its behavior and capabilities.
///
/// Invariants:
/// - An agent belongs to exactly one session
/// - An agent has a fixed type after creation
/// - An agent can execute only one task at a time
pub struct Agent {
    /// Unique identifier
    id: AgentId,

    /// Agent type determines behavior
    agent_type: AgentType,

    /// Reference to parent session
    session_id: SessionId,

    /// LLM model configuration
    model: ModelConfig,

    /// Available tools for this agent
    tools: ToolSet,

    /// Current execution state
    state: AgentState,

    /// Execution context (working directory, environment, etc.)
    context: ExecutionContext,

    /// System prompt
    system_prompt: String,

    /// Cancellation token for this agent
    cancel_token: CancellationToken,
}

/// Agent types with specialized behaviors
pub enum AgentType {
    /// General purpose agent for complex tasks
    GeneralPurpose,

    /// Fast agent for codebase exploration
    /// Uses: Glob, Grep, Read
    /// Model: haiku (fast)
    Explore {
        thoroughness: Thoroughness,
    },

    /// Planning agent for architecture decisions
    /// Uses: All tools
    /// Model: sonnet/opus
    Plan,

    /// Task execution agent for specific implementations
    /// Uses: All tools
    /// Model: sonnet
    Task,

    /// Guide agent for documentation queries
    /// Uses: Read, WebFetch, WebSearch
    /// Model: haiku
    Guide,

    /// Custom agent type
    Custom {
        name: String,
        config: CustomAgentConfig,
    },
}

pub enum Thoroughness {
    Quick,      // 1-2 search rounds
    Medium,     // 3-5 search rounds
    VeryThorough, // Exhaustive search
}

/// Agent states
pub enum AgentState {
    /// Agent is being initialized
    Initializing,
    /// Agent is ready to receive tasks
    Ready,
    /// Agent is thinking (waiting for LLM response)
    Thinking,
    /// Agent is executing tools
    ExecutingTools,
    /// Agent is waiting for permission
    WaitingForPermission,
    /// Agent completed successfully
    Completed,
    /// Agent encountered an error
    Error(AgentError),
    /// Agent was cancelled
    Cancelled,
}
```

### 2.3 Task (Value Object)

```rust
/// Task represents a unit of work for an agent to execute.
/// Tasks are immutable once created.
///
/// Invariants:
/// - A task belongs to exactly one agent
/// - A task has a clear description
/// - A task produces a trajectory of steps
pub struct Task {
    /// Unique identifier
    id: TaskId,

    /// Human-readable description
    description: String,

    /// Task metadata
    metadata: TaskMetadata,

    /// Execution steps (built during execution)
    steps: Vec<Step>,

    /// Execution trajectory for replay/analysis
    trajectory: Trajectory,

    /// Task result (set on completion)
    result: Option<TaskResult>,
}

pub struct TaskMetadata {
    /// Priority level
    priority: Priority,

    /// Maximum execution time
    timeout: Duration,

    /// Maximum number of steps
    max_steps: usize,

    /// Parent task (for subtasks)
    parent_id: Option<TaskId>,
}

pub enum Priority {
    Low,
    Normal,
    High,
    Critical,
}
```

### 2.4 Tool (Entity)

```rust
/// Tool represents a capability that an agent can use.
/// Tools are registered in a ToolRegistry and can be shared across agents.
///
/// Invariants:
/// - Tool names are unique within a registry
/// - Tools are stateless (state passed via ToolCall)
/// - Tools must declare their concurrency mode
pub trait Tool: Send + Sync {
    /// Unique name for this tool
    fn name(&self) -> &str;

    /// Human-readable description
    fn description(&self) -> &str;

    /// JSON Schema for input validation
    fn schema(&self) -> ToolSchema;

    /// Check if tool can be executed with given input
    fn validate(&self, input: &Value) -> Result<(), ValidationError>;

    /// Check permissions before execution
    fn check_permission(
        &self,
        call: &ToolCall,
        context: &ToolContext,
    ) -> impl Future<Output = PermissionResult> + Send;

    /// Execute the tool
    fn execute(
        &self,
        call: ToolCall,
    ) -> impl Stream<Item = ToolProgress> + Send;

    /// Concurrency mode
    fn concurrency_mode(&self) -> ConcurrencyMode {
        ConcurrencyMode::Parallel
    }

    /// Maximum execution time (None = use default)
    fn max_execution_time(&self) -> Option<Duration> {
        None
    }

    /// Whether this tool is read-only
    fn is_read_only(&self) -> bool {
        false
    }
}

pub enum ConcurrencyMode {
    /// Can run in parallel with other tools
    Parallel,
    /// Must run sequentially
    Sequential,
    /// Can run in parallel but limited count
    Limited(usize),
}
```

### 2.5 ToolCall (Value Object)

```rust
/// ToolCall represents a single invocation of a tool.
/// It is immutable and contains all information needed for execution.
pub struct ToolCall {
    /// Unique identifier
    id: ToolCallId,

    /// Name of the tool to invoke
    tool_name: String,

    /// Input parameters (validated against schema)
    input: Value,

    /// Invocation context
    context: ToolCallContext,

    /// When this call was created
    created_at: DateTime<Utc>,
}

pub struct ToolCallContext {
    /// Agent that initiated this call
    agent_id: AgentId,

    /// Task this call belongs to
    task_id: TaskId,

    /// Working directory
    working_dir: PathBuf,

    /// Environment variables
    env: HashMap<String, String>,
}
```

### 2.6 ToolResult (Value Object)

```rust
/// ToolResult represents the outcome of a tool execution.
pub struct ToolResult {
    /// Reference to the tool call
    call_id: ToolCallId,

    /// Execution status
    status: ToolStatus,

    /// Output content
    output: ToolOutput,

    /// Execution duration
    duration: Duration,

    /// Resource usage
    resources: ResourceUsage,
}

pub enum ToolStatus {
    Success,
    Error(ToolError),
    Timeout,
    Cancelled,
    PermissionDenied,
}

pub enum ToolOutput {
    /// Text output
    Text(String),

    /// Structured data
    Json(Value),

    /// Binary data (e.g., image)
    Binary {
        mime_type: String,
        data: Bytes,
    },

    /// Multiple outputs
    Multiple(Vec<ToolOutput>),

    /// No output
    Empty,
}
```

---

## 3. Value Objects

### 3.1 Message

```rust
/// Message represents a single message in the conversation.
pub struct Message {
    /// Unique identifier
    id: MessageId,

    /// Message role
    role: Role,

    /// Message content (can be multimodal)
    content: Vec<ContentBlock>,

    /// Timestamp
    timestamp: DateTime<Utc>,

    /// Optional metadata
    metadata: Option<MessageMetadata>,
}

pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

pub enum ContentBlock {
    Text {
        text: String,
    },
    Image {
        source: ImageSource,
        media_type: String,
    },
    ToolUse {
        id: ToolCallId,
        name: String,
        input: Value,
    },
    ToolResult {
        tool_use_id: ToolCallId,
        content: ToolOutput,
        is_error: bool,
    },
    Thinking {
        thinking: String,
        signature: Option<String>,
    },
}
```

### 3.2 Event

```rust
/// Event represents a state change in the system.
/// Events are broadcast to all subscribers via the event bus.
pub enum Event {
    // === Session Events ===
    SessionStarted {
        session_id: SessionId,
        config: SessionConfig,
    },
    SessionEnded {
        session_id: SessionId,
        reason: EndReason,
    },

    // === Stream Events ===
    StreamConnected,
    StreamDisconnected {
        reason: DisconnectReason,
    },

    // === Message Events ===
    MessageStart {
        message: Message,
    },
    ContentBlockStart {
        index: usize,
        block_type: BlockType,
    },
    ContentBlockDelta {
        index: usize,
        delta: Delta,
    },
    ContentBlockStop {
        index: usize,
    },
    MessageStop {
        stop_reason: StopReason,
    },

    // === Agent Events ===
    AgentSpawned {
        agent_id: AgentId,
        agent_type: AgentType,
    },
    AgentStateChanged {
        agent_id: AgentId,
        old_state: AgentState,
        new_state: AgentState,
    },
    AgentCompleted {
        agent_id: AgentId,
        result: AgentResult,
    },

    // === Tool Events ===
    ToolCallStart {
        call: ToolCall,
    },
    ToolCallProgress {
        call_id: ToolCallId,
        progress: Progress,
    },
    ToolCallComplete {
        call_id: ToolCallId,
        result: ToolResult,
    },

    // === Permission Events ===
    PermissionRequested {
        request_id: RequestId,
        tool_call: ToolCall,
        reason: String,
    },
    PermissionGranted {
        request_id: RequestId,
    },
    PermissionDenied {
        request_id: RequestId,
        reason: String,
    },

    // === Error Events ===
    Error {
        error: SageError,
        context: ErrorContext,
    },
}

pub enum Delta {
    TextDelta {
        text: String,
    },
    InputJsonDelta {
        partial_json: String,
    },
    ThinkingDelta {
        thinking: String,
    },
}
```

---

## 4. Aggregates

### 4.1 Session Aggregate

```
+------------------------------------------------------------------+
|                      Session Aggregate                            |
+------------------------------------------------------------------+
|                                                                   |
|  Session (Aggregate Root)                                         |
|  +-------------------------------------------------------------+  |
|  |                                                             |  |
|  |  +-------------+     +---------------+     +-----------+   |  |
|  |  |  Messages   |     | MessageStream |     |  Agents   |   |  |
|  |  | (Value Obj) |     |   (Entity)    |     | (Handles) |   |  |
|  |  +-------------+     +---------------+     +-----------+   |  |
|  |                                                             |  |
|  +-------------------------------------------------------------+  |
|                                                                   |
|  Invariants:                                                      |
|  - Only one active message stream per session                    |
|  - Messages are append-only                                      |
|  - Agent handles are managed by session                          |
|                                                                   |
+------------------------------------------------------------------+
```

### 4.2 Agent Aggregate

```
+------------------------------------------------------------------+
|                       Agent Aggregate                             |
+------------------------------------------------------------------+
|                                                                   |
|  Agent (Aggregate Root)                                          |
|  +-------------------------------------------------------------+  |
|  |                                                             |  |
|  |  +---------+     +----------+     +-----------------+      |  |
|  |  | ToolSet |     |  Tasks   |     | ExecutionContext |      |  |
|  |  | (Refs)  |     | (Entity) |     |   (Value Obj)   |      |  |
|  |  +---------+     +----------+     +-----------------+      |  |
|  |                       |                                     |  |
|  |                       v                                     |  |
|  |                  +---------+                               |  |
|  |                  |  Steps  |                               |  |
|  |                  +---------+                               |  |
|  |                                                             |  |
|  +-------------------------------------------------------------+  |
|                                                                   |
|  Invariants:                                                      |
|  - Only one task active at a time                                |
|  - Tools are immutable references                                |
|  - Context is scoped to agent lifetime                           |
|                                                                   |
+------------------------------------------------------------------+
```

---

## 5. Domain Services

### 5.1 AgentFactory

```rust
/// Creates agents with appropriate configuration
pub struct AgentFactory {
    tool_registry: Arc<ToolRegistry>,
    llm_client: Arc<LLMClient>,
    default_config: AgentConfig,
}

impl AgentFactory {
    pub fn create(&self, agent_type: AgentType) -> Box<dyn Agent>;
    pub fn create_with_config(&self, agent_type: AgentType, config: AgentConfig) -> Box<dyn Agent>;
}
```

### 5.2 ToolExecutor

```rust
/// Executes tool calls with proper isolation and concurrency control
pub struct ToolExecutor {
    registry: Arc<ToolRegistry>,
    semaphore: Arc<Semaphore>,
    sandbox: Option<Arc<Sandbox>>,
}

impl ToolExecutor {
    pub fn execute(&self, call: ToolCall) -> impl Stream<Item = ToolProgress>;
    pub fn execute_batch(&self, calls: Vec<ToolCall>) -> impl Stream<Item = BatchProgress>;
}
```

### 5.3 MessageStreamHandler

```rust
/// Handles bidirectional message streaming
pub struct MessageStreamHandler {
    llm_client: Arc<LLMClient>,
    event_bus: broadcast::Sender<Event>,
}

impl MessageStreamHandler {
    pub fn send(&self, message: Message) -> impl Stream<Item = Event>;
    pub fn subscribe(&self) -> broadcast::Receiver<Event>;
}
```

---

## 6. Repository Interfaces

```rust
/// Session persistence
#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn save(&self, session: &Session) -> Result<()>;
    async fn load(&self, id: SessionId) -> Result<Option<Session>>;
    async fn delete(&self, id: SessionId) -> Result<()>;
    async fn list(&self, filter: SessionFilter) -> Result<Vec<SessionSummary>>;
}

/// Trajectory persistence
#[async_trait]
pub trait TrajectoryRepository: Send + Sync {
    async fn save(&self, trajectory: &Trajectory) -> Result<()>;
    async fn load(&self, task_id: TaskId) -> Result<Option<Trajectory>>;
    async fn query(&self, filter: TrajectoryFilter) -> Result<Vec<Trajectory>>;
}
```

---

## 7. Entity Relationship Diagram

```
+------------------------------------------------------------------+
|                    Entity Relationships                           |
+------------------------------------------------------------------+
|                                                                   |
|  Session                                                          |
|     |                                                             |
|     +--< MessageStream (1:1)                                     |
|     |                                                             |
|     +--< Agent (1:N)                                             |
|     |       |                                                     |
|     |       +--< Task (1:N)                                      |
|     |       |       |                                             |
|     |       |       +--< Step (1:N)                              |
|     |       |       |                                             |
|     |       |       +--< ToolCall (1:N)                          |
|     |       |               |                                     |
|     |       |               +--< ToolResult (1:1)                |
|     |       |                                                     |
|     |       +--< ToolSet (1:1, references)                       |
|     |                                                             |
|     +--< Message (1:N)                                           |
|             |                                                     |
|             +--< ContentBlock (1:N)                              |
|                                                                   |
|  ToolRegistry (Singleton)                                         |
|     |                                                             |
|     +--< Tool (1:N)                                              |
|                                                                   |
|  EventBus (Singleton)                                             |
|     |                                                             |
|     +--< Event (N:M with subscribers)                            |
|                                                                   |
+------------------------------------------------------------------+
```

---

## 8. Glossary

| Term | Definition |
|------|------------|
| **Session** | Complete user interaction context |
| **Agent** | Autonomous entity executing tasks |
| **Task** | Unit of work with defined outcome |
| **Tool** | Capability that agents can invoke |
| **ToolCall** | Single invocation of a tool |
| **Event** | State change notification |
| **MessageStream** | Real-time bidirectional communication |
| **Trajectory** | Record of execution steps |
| **Aggregate** | Cluster of entities with consistency boundary |
| **Value Object** | Immutable object defined by attributes |
