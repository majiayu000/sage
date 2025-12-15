# Core Interfaces Design

> Trait definitions and API contracts that components must implement

## 1. Design Principles

```
+------------------------------------------------------------------+
|                    INTERFACE DESIGN PRINCIPLES                    |
+------------------------------------------------------------------+
|                                                                   |
|  1. ASYNC BY DEFAULT                                             |
|     All I/O operations are async                                 |
|     Use #[async_trait] for trait methods                         |
|                                                                   |
|  2. STREAM OVER COLLECT                                          |
|     Return impl Stream instead of Vec where possible             |
|     Enables backpressure and early termination                   |
|                                                                   |
|  3. ERROR IN TYPE                                                |
|     Use Result<T, E> for fallible operations                     |
|     Use specific error types per domain                          |
|                                                                   |
|  4. EXTENSIBILITY VIA TRAITS                                     |
|     Core functionality defined as traits                         |
|     Default implementations where sensible                       |
|                                                                   |
|  5. BUILDER FOR CONFIGURATION                                    |
|     Complex objects constructed via builders                     |
|     Required vs optional parameters clear                        |
|                                                                   |
+------------------------------------------------------------------+
```

---

## 2. Core Traits

### 2.1 Agent Trait

```rust
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Core trait that all agents must implement.
///
/// Agents are autonomous entities that can execute tasks using tools
/// and LLM capabilities. Each agent has a specific type that determines
/// its behavior, available tools, and model configuration.
#[async_trait]
pub trait Agent: Send + Sync {
    /// Returns the unique identifier for this agent instance.
    fn id(&self) -> AgentId;

    /// Returns the type of this agent.
    fn agent_type(&self) -> AgentType;

    /// Returns the agent's configuration.
    fn config(&self) -> &AgentConfig;

    /// Returns the tools available to this agent.
    fn tools(&self) -> &[Arc<dyn Tool>];

    /// Executes a task and returns a stream of events.
    ///
    /// The returned stream yields events as the agent processes the task,
    /// including thinking, tool calls, and results.
    ///
    /// # Arguments
    /// * `task` - The task to execute
    ///
    /// # Returns
    /// A stream of events representing the execution progress
    fn execute(
        &self,
        task: Task,
    ) -> Pin<Box<dyn Stream<Item = Result<Event, AgentError>> + Send + '_>>;

    /// Continues execution after receiving user input.
    ///
    /// # Arguments
    /// * `input` - The user's input to continue with
    fn resume(
        &self,
        input: UserInput,
    ) -> Pin<Box<dyn Stream<Item = Result<Event, AgentError>> + Send + '_>>;

    /// Requests cancellation of the current execution.
    fn cancel(&self);

    /// Returns the current state of the agent.
    fn state(&self) -> AgentState;

    /// Returns the cancellation token for this agent.
    fn cancellation_token(&self) -> CancellationToken;
}

/// Extension trait for agent lifecycle management.
#[async_trait]
pub trait AgentLifecycle: Agent {
    /// Called when the agent is initialized.
    async fn on_init(&mut self) -> Result<(), AgentError> {
        Ok(())
    }

    /// Called before each task execution.
    async fn on_task_start(&mut self, task: &Task) -> Result<(), AgentError> {
        Ok(())
    }

    /// Called after task completion.
    async fn on_task_complete(
        &mut self,
        task: &Task,
        result: &TaskResult,
    ) -> Result<(), AgentError> {
        Ok(())
    }

    /// Called when the agent is being shut down.
    async fn on_shutdown(&mut self) -> Result<(), AgentError> {
        Ok(())
    }
}
```

### 2.2 Tool Trait

```rust
use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use std::time::Duration;

/// Core trait that all tools must implement.
///
/// Tools are capabilities that agents can use to interact with the
/// environment. Each tool has a schema for validation, permission
/// checking, and execution logic.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Returns the unique name of this tool.
    ///
    /// Tool names must be unique within a registry and should follow
    /// the pattern: lowercase with underscores (e.g., "read_file").
    fn name(&self) -> &str;

    /// Returns a human-readable description of what this tool does.
    ///
    /// This description is included in the system prompt to help the
    /// LLM understand when to use this tool.
    fn description(&self) -> &str;

    /// Returns the JSON Schema for this tool's input parameters.
    fn schema(&self) -> ToolSchema;

    /// Validates the input against the schema.
    ///
    /// # Arguments
    /// * `input` - The input to validate
    ///
    /// # Returns
    /// Ok(()) if valid, Err with validation details if invalid
    fn validate(&self, input: &Value) -> Result<(), ValidationError> {
        // Default implementation uses JSON Schema validation
        self.schema().validate(input)
    }

    /// Checks if the tool call is permitted in the current context.
    ///
    /// This method is called before execution to determine if the
    /// operation should be allowed, denied, or requires user approval.
    ///
    /// # Arguments
    /// * `call` - The tool call to check
    /// * `context` - The execution context
    ///
    /// # Returns
    /// Permission decision
    async fn check_permission(
        &self,
        call: &ToolCall,
        context: &ToolContext,
    ) -> PermissionResult {
        PermissionResult::Allow
    }

    /// Executes the tool and returns a stream of progress updates.
    ///
    /// # Arguments
    /// * `call` - The tool call to execute
    ///
    /// # Returns
    /// A stream of progress updates, ending with the final result
    fn execute(
        &self,
        call: ToolCall,
    ) -> Pin<Box<dyn Stream<Item = ToolProgress> + Send + '_>>;

    /// Returns the concurrency mode for this tool.
    ///
    /// This determines whether multiple instances of this tool can
    /// run in parallel.
    fn concurrency_mode(&self) -> ConcurrencyMode {
        ConcurrencyMode::Parallel
    }

    /// Returns the maximum execution time for this tool.
    ///
    /// If None, the default timeout from configuration is used.
    fn max_execution_time(&self) -> Option<Duration> {
        None
    }

    /// Returns whether this tool only reads data (no side effects).
    fn is_read_only(&self) -> bool {
        false
    }

    /// Renders the tool call for display to the user.
    fn render_call(&self, call: &ToolCall) -> String {
        format!("{}({})", self.name(), call.input)
    }

    /// Renders the tool result for display to the user.
    fn render_result(&self, result: &ToolResult) -> String {
        match &result.output {
            ToolOutput::Text(s) => s.clone(),
            ToolOutput::Json(v) => serde_json::to_string_pretty(v).unwrap_or_default(),
            _ => format!("{:?}", result.output),
        }
    }
}

/// Concurrency mode for tool execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConcurrencyMode {
    /// Tool can run in parallel with any other tool.
    Parallel,

    /// Tool must run sequentially (one at a time globally).
    Sequential,

    /// Tool can run in parallel but with a maximum count.
    Limited(usize),

    /// Tool can run in parallel but not with tools of the same type.
    ExclusiveByType,
}

/// Permission check result.
#[derive(Debug, Clone)]
pub enum PermissionResult {
    /// Allow execution to proceed.
    Allow,

    /// Deny execution with reason.
    Deny { reason: String },

    /// Ask user for permission.
    Ask {
        question: String,
        default: bool,
    },

    /// Transform the input before execution.
    Transform { new_input: Value },
}

/// Progress update from tool execution.
#[derive(Debug, Clone)]
pub enum ToolProgress {
    /// Execution started.
    Started,

    /// Progress update with percentage.
    Progress {
        percent: f32,
        message: Option<String>,
    },

    /// Intermediate output.
    Output(ToolOutput),

    /// Execution completed successfully.
    Completed(ToolResult),

    /// Execution failed.
    Failed(ToolError),
}
```

### 2.3 LLM Client Trait

```rust
use async_trait::async_trait;
use futures::Stream;

/// Trait for LLM client implementations.
///
/// Each LLM provider (Anthropic, OpenAI, etc.) implements this trait
/// to provide a unified interface for chat and streaming.
#[async_trait]
pub trait LLMClient: Send + Sync {
    /// Returns the provider name.
    fn provider(&self) -> LLMProvider;

    /// Sends a chat request and waits for the complete response.
    ///
    /// # Arguments
    /// * `request` - The chat request
    ///
    /// # Returns
    /// The complete response
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, LLMError>;

    /// Sends a chat request and returns a stream of events.
    ///
    /// # Arguments
    /// * `request` - The chat request
    ///
    /// # Returns
    /// A stream of SSE events
    fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamEvent, LLMError>> + Send + '_>>;

    /// Counts tokens for the given messages.
    ///
    /// # Arguments
    /// * `messages` - The messages to count
    ///
    /// # Returns
    /// Token count
    async fn count_tokens(&self, messages: &[Message]) -> Result<usize, LLMError>;

    /// Returns the maximum context length for the model.
    fn max_context_length(&self) -> usize;

    /// Returns supported features for this client.
    fn capabilities(&self) -> LLMCapabilities;
}

/// Chat request structure.
#[derive(Debug, Clone)]
pub struct ChatRequest {
    /// Model to use.
    pub model: String,

    /// System prompt.
    pub system: Option<String>,

    /// Conversation messages.
    pub messages: Vec<Message>,

    /// Available tools.
    pub tools: Vec<ToolSchema>,

    /// Maximum tokens to generate.
    pub max_tokens: usize,

    /// Temperature for sampling.
    pub temperature: Option<f32>,

    /// Stop sequences.
    pub stop_sequences: Option<Vec<String>>,

    /// Whether to stream the response.
    pub stream: bool,
}

/// LLM client capabilities.
#[derive(Debug, Clone)]
pub struct LLMCapabilities {
    /// Supports streaming responses.
    pub streaming: bool,

    /// Supports tool/function calling.
    pub tools: bool,

    /// Supports vision (image input).
    pub vision: bool,

    /// Supports extended thinking.
    pub thinking: bool,

    /// Supports JSON mode.
    pub json_mode: bool,
}
```

### 2.4 MCP Client Trait

```rust
use async_trait::async_trait;

/// Model Context Protocol client interface.
///
/// MCP allows extending agent capabilities by connecting to
/// external tool servers.
#[async_trait]
pub trait MCPClient: Send + Sync {
    /// Initializes the connection to the MCP server.
    async fn initialize(&mut self) -> Result<ServerInfo, MCPError>;

    /// Lists available tools from the server.
    async fn list_tools(&self) -> Result<Vec<MCPTool>, MCPError>;

    /// Calls a tool on the server.
    ///
    /// # Arguments
    /// * `name` - Tool name
    /// * `arguments` - Tool arguments
    ///
    /// # Returns
    /// Tool result
    async fn call_tool(
        &self,
        name: &str,
        arguments: Value,
    ) -> Result<MCPToolResult, MCPError>;

    /// Lists available resources.
    async fn list_resources(&self) -> Result<Vec<MCPResource>, MCPError>;

    /// Reads a resource.
    ///
    /// # Arguments
    /// * `uri` - Resource URI
    ///
    /// # Returns
    /// Resource contents
    async fn read_resource(&self, uri: &str) -> Result<ResourceContents, MCPError>;

    /// Lists available prompts.
    async fn list_prompts(&self) -> Result<Vec<MCPPrompt>, MCPError>;

    /// Gets a prompt with arguments.
    ///
    /// # Arguments
    /// * `name` - Prompt name
    /// * `arguments` - Prompt arguments
    ///
    /// # Returns
    /// Rendered prompt messages
    async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<Value>,
    ) -> Result<Vec<Message>, MCPError>;

    /// Closes the connection.
    async fn close(&mut self) -> Result<(), MCPError>;
}
```

---

## 3. Service Interfaces

### 3.1 Session Service

```rust
/// Session management service.
pub trait SessionService: Send + Sync {
    /// Creates a new session.
    fn create_session(&self, config: SessionConfig) -> Result<Session, SessionError>;

    /// Gets an existing session by ID.
    fn get_session(&self, id: SessionId) -> Option<&Session>;

    /// Lists all active sessions.
    fn list_sessions(&self) -> Vec<SessionSummary>;

    /// Ends a session.
    fn end_session(&self, id: SessionId) -> Result<(), SessionError>;
}
```

### 3.2 Tool Registry

```rust
/// Registry for tool discovery and management.
pub trait ToolRegistry: Send + Sync {
    /// Registers a tool.
    fn register(&mut self, tool: Arc<dyn Tool>) -> Result<(), RegistryError>;

    /// Unregisters a tool by name.
    fn unregister(&mut self, name: &str) -> Result<(), RegistryError>;

    /// Gets a tool by name.
    fn get(&self, name: &str) -> Option<Arc<dyn Tool>>;

    /// Lists all registered tools.
    fn list(&self) -> Vec<Arc<dyn Tool>>;

    /// Gets schemas for all tools.
    fn schemas(&self) -> Vec<ToolSchema>;

    /// Filters tools by predicate.
    fn filter<F>(&self, predicate: F) -> Vec<Arc<dyn Tool>>
    where
        F: Fn(&dyn Tool) -> bool;
}
```

### 3.3 Event Bus

```rust
use tokio::sync::broadcast;

/// Event distribution service.
pub trait EventBus: Send + Sync {
    /// Publishes an event to all subscribers.
    fn publish(&self, event: Event);

    /// Subscribes to events.
    fn subscribe(&self) -> broadcast::Receiver<Event>;

    /// Gets the number of active subscribers.
    fn subscriber_count(&self) -> usize;
}
```

---

## 4. Builder Interfaces

### 4.1 SageBuilder

```rust
/// Builder for constructing Sage agent instances.
pub struct SageBuilder {
    config: Option<Config>,
    llm_client: Option<Arc<dyn LLMClient>>,
    tools: Vec<Arc<dyn Tool>>,
    mcp_servers: Vec<MCPServerConfig>,
    event_handlers: Vec<Box<dyn EventHandler>>,
}

impl SageBuilder {
    /// Creates a new builder with default settings.
    pub fn new() -> Self {
        Self {
            config: None,
            llm_client: None,
            tools: Vec::new(),
            mcp_servers: Vec::new(),
            event_handlers: Vec::new(),
        }
    }

    /// Sets the configuration.
    pub fn with_config(mut self, config: Config) -> Self {
        self.config = Some(config);
        self
    }

    /// Loads configuration from a file.
    pub fn with_config_file(mut self, path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let config = Config::from_file(path)?;
        self.config = Some(config);
        Ok(self)
    }

    /// Sets the LLM client.
    pub fn with_llm_client(mut self, client: Arc<dyn LLMClient>) -> Self {
        self.llm_client = Some(client);
        self
    }

    /// Configures LLM from provider and API key.
    pub fn with_llm(
        mut self,
        provider: LLMProvider,
        api_key: impl Into<String>,
    ) -> Self {
        let client = create_llm_client(provider, api_key.into());
        self.llm_client = Some(Arc::new(client));
        self
    }

    /// Adds a tool.
    pub fn with_tool(mut self, tool: Arc<dyn Tool>) -> Self {
        self.tools.push(tool);
        self
    }

    /// Adds multiple tools.
    pub fn with_tools(mut self, tools: Vec<Arc<dyn Tool>>) -> Self {
        self.tools.extend(tools);
        self
    }

    /// Adds default tools.
    pub fn with_default_tools(mut self) -> Self {
        self.tools.extend(get_default_tools());
        self
    }

    /// Adds an MCP server.
    pub fn with_mcp_server(mut self, config: MCPServerConfig) -> Self {
        self.mcp_servers.push(config);
        self
    }

    /// Adds an event handler.
    pub fn on_event<H: EventHandler + 'static>(mut self, handler: H) -> Self {
        self.event_handlers.push(Box::new(handler));
        self
    }

    /// Builds the Sage instance.
    pub async fn build(self) -> Result<Sage, BuildError> {
        let config = self.config.unwrap_or_default();
        let llm_client = self.llm_client
            .ok_or(BuildError::MissingLLMClient)?;

        // Initialize MCP clients
        let mcp_clients = self.initialize_mcp_clients().await?;

        // Build tool registry
        let mut registry = ToolRegistry::new();
        for tool in self.tools {
            registry.register(tool)?;
        }

        // Add MCP tools
        for client in &mcp_clients {
            let tools = client.list_tools().await?;
            for tool in tools {
                registry.register(Arc::new(MCPToolWrapper::new(tool, client.clone())))?;
            }
        }

        Ok(Sage {
            config,
            llm_client,
            tool_registry: Arc::new(registry),
            mcp_clients,
            event_handlers: self.event_handlers,
        })
    }
}
```

### 4.2 AgentBuilder

```rust
/// Builder for constructing agent instances.
pub struct AgentBuilder {
    agent_type: AgentType,
    model: Option<String>,
    system_prompt: Option<String>,
    tools: Option<Vec<Arc<dyn Tool>>>,
    config: AgentConfig,
}

impl AgentBuilder {
    /// Creates a new builder for the specified agent type.
    pub fn new(agent_type: AgentType) -> Self {
        Self {
            agent_type,
            model: None,
            system_prompt: None,
            tools: None,
            config: AgentConfig::default(),
        }
    }

    /// Sets the model to use.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Sets the system prompt.
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Sets the available tools.
    pub fn with_tools(mut self, tools: Vec<Arc<dyn Tool>>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Sets the maximum number of steps.
    pub fn with_max_steps(mut self, max_steps: usize) -> Self {
        self.config.max_steps = max_steps;
        self
    }

    /// Sets the timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    /// Builds the agent.
    pub fn build(
        self,
        llm_client: Arc<dyn LLMClient>,
        tool_registry: Arc<ToolRegistry>,
    ) -> Result<Box<dyn Agent>, BuildError> {
        let model = self.model
            .unwrap_or_else(|| self.default_model_for_type());
        let tools = self.tools
            .unwrap_or_else(|| self.default_tools_for_type(&tool_registry));
        let system_prompt = self.system_prompt
            .unwrap_or_else(|| self.default_prompt_for_type());

        match self.agent_type {
            AgentType::GeneralPurpose => {
                Ok(Box::new(GeneralAgent::new(
                    llm_client, tools, model, system_prompt, self.config,
                )))
            }
            AgentType::Explore { thoroughness } => {
                Ok(Box::new(ExploreAgent::new(
                    llm_client, tools, model, system_prompt, thoroughness, self.config,
                )))
            }
            AgentType::Plan => {
                Ok(Box::new(PlanAgent::new(
                    llm_client, tools, model, system_prompt, self.config,
                )))
            }
            AgentType::Task => {
                Ok(Box::new(TaskAgent::new(
                    llm_client, tools, model, system_prompt, self.config,
                )))
            }
            _ => Err(BuildError::UnsupportedAgentType),
        }
    }
}
```

---

## 5. Callback Interfaces

### 5.1 Event Handler

```rust
/// Handler for agent events.
#[async_trait]
pub trait EventHandler: Send + Sync {
    /// Called when an event occurs.
    async fn on_event(&self, event: &Event);

    /// Returns the event types this handler is interested in.
    fn event_filter(&self) -> Option<Vec<EventType>> {
        None // All events by default
    }
}

/// Simple function-based event handler.
pub struct FnEventHandler<F> {
    handler: F,
}

impl<F> FnEventHandler<F>
where
    F: Fn(&Event) + Send + Sync,
{
    pub fn new(handler: F) -> Self {
        Self { handler }
    }
}

#[async_trait]
impl<F> EventHandler for FnEventHandler<F>
where
    F: Fn(&Event) + Send + Sync,
{
    async fn on_event(&self, event: &Event) {
        (self.handler)(event);
    }
}
```

### 5.2 Permission Handler

```rust
/// Handler for permission requests.
#[async_trait]
pub trait PermissionHandler: Send + Sync {
    /// Called when a tool requires permission.
    ///
    /// # Arguments
    /// * `request` - The permission request details
    ///
    /// # Returns
    /// Whether to allow the operation
    async fn handle_permission_request(
        &self,
        request: PermissionRequest,
    ) -> PermissionDecision;
}

/// Permission request details.
pub struct PermissionRequest {
    /// The tool being called.
    pub tool_name: String,

    /// The tool call details.
    pub call: ToolCall,

    /// Reason for the permission check.
    pub reason: String,

    /// Risk level assessment.
    pub risk_level: RiskLevel,
}

/// Permission decision.
pub enum PermissionDecision {
    /// Allow the operation.
    Allow,

    /// Allow this and future similar operations.
    AllowAlways,

    /// Deny the operation.
    Deny,

    /// Deny this and future similar operations.
    DenyAlways,

    /// Modify the operation.
    Modify { new_call: ToolCall },
}

/// Risk level for operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}
```

---

## 6. Error Types

```rust
use thiserror::Error;

/// Top-level error type for Sage.
#[derive(Debug, Error)]
pub enum SageError {
    #[error("Agent error: {0}")]
    Agent(#[from] AgentError),

    #[error("Tool error: {0}")]
    Tool(#[from] ToolError),

    #[error("LLM error: {0}")]
    LLM(#[from] LLMError),

    #[error("MCP error: {0}")]
    MCP(#[from] MCPError),

    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("Session error: {0}")]
    Session(#[from] SessionError),

    #[error("Cancelled")]
    Cancelled,

    #[error("Timeout after {0:?}")]
    Timeout(Duration),
}

/// Agent-specific errors.
#[derive(Debug, Error)]
pub enum AgentError {
    #[error("Agent not initialized")]
    NotInitialized,

    #[error("Agent already running")]
    AlreadyRunning,

    #[error("Maximum steps exceeded: {0}")]
    MaxStepsExceeded(usize),

    #[error("Task execution failed: {0}")]
    ExecutionFailed(String),
}

/// Tool-specific errors.
#[derive(Debug, Error)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Timeout after {0:?}")]
    Timeout(Duration),
}

/// LLM-specific errors.
#[derive(Debug, Error)]
pub enum LLMError {
    #[error("API error: {status} - {message}")]
    ApiError { status: u16, message: String },

    #[error("Rate limited, retry after {retry_after:?}")]
    RateLimited { retry_after: Option<Duration> },

    #[error("Context length exceeded: {used}/{max}")]
    ContextLengthExceeded { used: usize, max: usize },

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Network error: {0}")]
    Network(String),
}
```

---

## 7. Type Aliases and Common Types

```rust
/// Type aliases for common patterns.
pub type Result<T, E = SageError> = std::result::Result<T, E>;

/// Boxed future for async trait methods.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Boxed stream for async iteration.
pub type BoxStream<'a, T> = Pin<Box<dyn Stream<Item = T> + Send + 'a>>;

/// Unique identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(pub Uuid);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AgentId(pub Uuid);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskId(pub Uuid);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ToolCallId(pub String);

/// Implementation of ID types.
impl SessionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl AgentId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl TaskId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl ToolCallId {
    pub fn new() -> Self {
        Self(format!("toolu_{}", Uuid::new_v4().simple()))
    }
}
```
