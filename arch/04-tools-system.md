# Tools System

## Tool Trait

All tools implement the `Tool` trait:

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    /// Tool name (unique identifier)
    fn name(&self) -> &str;

    /// Human-readable description
    fn description(&self) -> &str;

    /// JSON Schema for parameters
    fn schema(&self) -> ToolSchema;

    /// Execute the tool
    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError>;

    /// Risk level for permission system
    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    /// Concurrency mode
    fn concurrency_mode(&self) -> ConcurrencyMode {
        ConcurrencyMode::Parallel
    }

    /// Tool category for grouping
    fn category(&self) -> &str {
        "general"
    }
}
```

## Tool Schema

```rust
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
}

pub struct ToolParameter {
    pub name: String,
    pub description: String,
    pub param_type: ParameterType,
    pub required: bool,
    pub default: Option<Value>,
}
```

## Tool Call & Result

```rust
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: HashMap<String, Value>,
    pub call_id: Option<String>,
}

pub struct ToolResult {
    pub call_id: String,
    pub tool_name: String,
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub metadata: HashMap<String, Value>,
}
```

## Permission System

### Risk Levels

```rust
pub enum RiskLevel {
    Low,      // Read-only operations (view files, list)
    Medium,   // Modifications (edit files, create)
    High,     // System operations (execute commands, install)
    Critical, // Destructive operations (delete, format)
}
```

### Permission Flow

```
┌─────────────────────────────────────────────────────────────┐
│                    Permission Flow                           │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Tool Call                                                   │
│      │                                                       │
│      ▼                                                       │
│  ┌─────────────────┐                                        │
│  │  Build Context  │                                        │
│  │  - tool name    │                                        │
│  │  - arguments    │                                        │
│  │  - working dir  │                                        │
│  └────────┬────────┘                                        │
│           │                                                  │
│           ▼                                                  │
│  ┌─────────────────┐                                        │
│  │  Get Risk Level │                                        │
│  │  from Tool      │                                        │
│  └────────┬────────┘                                        │
│           │                                                  │
│           ▼                                                  │
│  ┌─────────────────┐      ┌────────────────┐               │
│  │  Check Cache    │─────▶│ Already Allowed│──────┐        │
│  └────────┬────────┘      └────────────────┘      │        │
│           │ miss                                   │        │
│           ▼                                        │        │
│  ┌─────────────────┐                              │        │
│  │ Permission      │                              │        │
│  │ Handler         │                              │        │
│  │ .check_permission()                            │        │
│  └────────┬────────┘                              │        │
│           │                                        │        │
│     ┌─────┴─────┐                                 │        │
│     │           │                                  │        │
│     ▼           ▼                                  │        │
│  Allowed     Denied                               │        │
│     │           │                                  │        │
│     │           ▼                                  ▼        │
│     │      ┌─────────┐                      ┌─────────┐   │
│     │      │  Error  │                      │ Execute │   │
│     │      └─────────┘                      │  Tool   │   │
│     │                                       └─────────┘   │
│     └──────────────────────────────────────────▲          │
│                                                            │
└─────────────────────────────────────────────────────────────┘
```

### Permission Handlers

```rust
#[async_trait]
pub trait PermissionHandler: Send + Sync {
    async fn check_permission(&self, context: &ToolContext) -> PermissionResult;
}

// Auto-allow all operations
pub struct AutoAllowHandler;

// Policy-based handler
pub struct PolicyHandler {
    policies: Vec<PermissionPolicy>,
}

pub struct PermissionPolicy {
    pub tool_pattern: String,      // regex pattern
    pub path_pattern: Option<String>,
    pub max_risk_level: RiskLevel,
    pub action: PolicyAction,
}
```

### Permission Cache

```rust
pub struct PermissionCache {
    cache: DashMap<String, (PermissionResult, Instant)>,
    ttl: Duration,
}

impl PermissionCache {
    pub fn get(&self, key: &str) -> Option<PermissionResult> {
        self.cache.get(key).and_then(|entry| {
            if entry.1.elapsed() < self.ttl {
                Some(entry.0.clone())
            } else {
                None
            }
        })
    }

    pub fn set(&self, key: String, result: PermissionResult) {
        self.cache.insert(key, (result, Instant::now()));
    }
}
```

## Tool Executors

### ToolExecutor (Sequential)

```rust
pub struct ToolExecutor {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolExecutor {
    pub async fn execute_tools(&self, calls: &[ToolCall]) -> Vec<ToolResult> {
        let mut results = Vec::new();
        for call in calls {
            let result = self.execute_single(call).await;
            results.push(result);
        }
        results
    }
}
```

### BatchToolExecutor

```rust
pub struct BatchToolExecutor {
    tools: DashMap<String, Arc<dyn Tool>>,
}

impl BatchToolExecutor {
    pub async fn execute_batch(&self, calls: &[ToolCall]) -> Vec<ToolResult> {
        let futures: Vec<_> = calls.iter().map(|call| {
            self.execute_single(call)
        }).collect();

        futures::future::join_all(futures).await
    }
}
```

### ParallelToolExecutor

```rust
pub struct ParallelToolExecutor {
    tools: DashMap<String, Arc<dyn Tool>>,
    config: ParallelExecutorConfig,
    global_semaphore: Arc<Semaphore>,
    type_semaphores: DashMap<String, Arc<Semaphore>>,
    sequential_lock: Arc<Mutex<()>>,
    permission_handler: Option<Arc<dyn PermissionHandler>>,
    cancellation_token: Option<CancellationToken>,
}

pub struct ParallelExecutorConfig {
    pub max_concurrent: usize,
    pub max_concurrent_per_type: usize,
    pub timeout: Duration,
}
```

## Built-in Tools (sage-tools)

| Tool | Description | Risk Level |
|------|-------------|------------|
| `bash` | Execute shell commands | High |
| `str_replace_based_edit_tool` | View/edit/create files | Medium |
| `json_edit_tool` | JSON file manipulation | Medium |
| `codebase_retrieval` | Semantic code search | Low |
| `add_tasks` | Task management | Low |
| `update_tasks` | Update task status | Low |
| `view_tasklist` | View current tasks | Low |
| `task_done` | Mark task complete | Low |
| `sequentialthinking` | Structured reasoning | Low |

## MCP Tool Adapter

```rust
pub struct McpToolAdapter {
    client: Arc<McpClient>,
    tool: McpTool,
}

#[async_trait]
impl Tool for McpToolAdapter {
    fn name(&self) -> &str {
        &self.tool.name
    }

    fn description(&self) -> &str {
        self.tool.description.as_deref().unwrap_or("")
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.tool.name.clone(),
            description: self.tool.description.clone().unwrap_or_default(),
            parameters: self.tool.input_schema.clone(),
        }
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let arguments: Value = call.arguments.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        match self.client.call_tool(&self.tool.name, arguments).await {
            Ok(result) => {
                // Convert MCP result to ToolResult
            }
            Err(e) => Err(ToolError::ExecutionFailed(e.to_string())),
        }
    }
}
```
