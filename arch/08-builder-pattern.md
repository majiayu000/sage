# SageBuilder Pattern

## Overview

SageBuilder provides a fluent interface for constructing fully configured Sage agents with all necessary components.

## Builder Structure

```rust
pub struct SageBuilder {
    config: Option<Config>,
    providers: HashMap<String, ProviderConfig>,
    default_provider: Option<String>,
    model_params: Option<ModelParameters>,
    tools: Vec<Arc<dyn Tool>>,
    hooks: Vec<Arc<dyn LifecycleHook>>,
    mcp_servers: Vec<(String, TransportConfig)>,
    trajectory_path: Option<PathBuf>,
    cache_config: Option<CacheConfig>,
    event_bus_capacity: usize,
    max_steps: Option<u32>,
    working_dir: Option<PathBuf>,
}
```

## Fluent API

```
┌─────────────────────────────────────────────────────────────┐
│                    SageBuilder Flow                          │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│   SageBuilder::new()                                        │
│        │                                                     │
│        ▼                                                     │
│   ┌─────────────────────────────────────────────────────┐   │
│   │              Configuration Phase                     │   │
│   │  .with_config(config)                               │   │
│   │  .with_config_file("path/to/config.json")           │   │
│   └──────────────────────┬──────────────────────────────┘   │
│                          │                                   │
│                          ▼                                   │
│   ┌─────────────────────────────────────────────────────┐   │
│   │               Provider Phase                         │   │
│   │  .with_openai("api_key")                            │   │
│   │  .with_anthropic("api_key")                         │   │
│   │  .with_google("api_key")                            │   │
│   │  .with_provider("name", config)                     │   │
│   │  .with_default_provider("anthropic")                │   │
│   └──────────────────────┬──────────────────────────────┘   │
│                          │                                   │
│                          ▼                                   │
│   ┌─────────────────────────────────────────────────────┐   │
│   │                Model Phase                           │   │
│   │  .with_model("claude-3-opus")                       │   │
│   │  .with_temperature(0.7)                             │   │
│   │  .with_max_tokens(4096)                             │   │
│   └──────────────────────┬──────────────────────────────┘   │
│                          │                                   │
│                          ▼                                   │
│   ┌─────────────────────────────────────────────────────┐   │
│   │                 Tools Phase                          │   │
│   │  .with_tool(tool)                                   │   │
│   │  .with_tools(vec![tool1, tool2])                    │   │
│   │  .with_mcp_server("name", config)                   │   │
│   │  .with_mcp_stdio_server("name", "cmd", args)        │   │
│   └──────────────────────┬──────────────────────────────┘   │
│                          │                                   │
│                          ▼                                   │
│   ┌─────────────────────────────────────────────────────┐   │
│   │               Lifecycle Phase                        │   │
│   │  .with_hook(hook)                                   │   │
│   │  .with_hooks(vec![hook1, hook2])                    │   │
│   └──────────────────────┬──────────────────────────────┘   │
│                          │                                   │
│                          ▼                                   │
│   ┌─────────────────────────────────────────────────────┐   │
│   │              Infrastructure Phase                    │   │
│   │  .with_trajectory_path("path")                      │   │
│   │  .with_cache()                                      │   │
│   │  .with_cache_config(config)                         │   │
│   │  .with_event_bus_capacity(1000)                     │   │
│   │  .with_max_steps(50)                                │   │
│   │  .with_working_dir("/path")                         │   │
│   └──────────────────────┬──────────────────────────────┘   │
│                          │                                   │
│                          ▼                                   │
│   ┌─────────────────────────────────────────────────────┐   │
│   │                 Build Phase                          │   │
│   │  .build() -> SageComponents                         │   │
│   │  .build_llm_client() -> LLMClient                   │   │
│   │  .build_tool_executor() -> ToolExecutor             │   │
│   │  .build_batch_executor() -> BatchToolExecutor       │   │
│   │  .build_mcp_registry() -> McpRegistry               │   │
│   │  .build_lifecycle_manager() -> LifecycleManager     │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Provider Configuration

```rust
// Individual providers
let builder = SageBuilder::new()
    .with_openai("sk-...")
    .with_anthropic("sk-ant-...")
    .with_google("...")
    .with_default_provider("anthropic");

// Custom provider
let custom_config = ProviderConfig::new("custom")
    .with_api_key("...")
    .with_base_url("https://api.custom.com")
    .with_timeout(120)
    .with_max_retries(5);

let builder = SageBuilder::new()
    .with_provider("custom", custom_config);
```

## Model Configuration

```rust
let builder = SageBuilder::new()
    .with_anthropic("key")
    .with_model("claude-3-opus-20240229")
    .with_temperature(0.7)
    .with_max_tokens(8192);
```

## Tool Registration

```rust
// Single tool
let builder = SageBuilder::new()
    .with_tool(Arc::new(BashTool::new()));

// Multiple tools
let builder = SageBuilder::new()
    .with_tools(vec![
        Arc::new(BashTool::new()),
        Arc::new(EditTool::new()),
        Arc::new(CodebaseRetrieval::new()),
    ]);
```

## MCP Server Integration

```rust
// Stdio transport
let builder = SageBuilder::new()
    .with_mcp_stdio_server(
        "filesystem",
        "npx",
        vec!["-y", "@modelcontextprotocol/server-filesystem", "/tmp"],
    )
    .with_mcp_stdio_server(
        "git",
        "uvx",
        vec!["mcp-server-git"],
    );

// Custom transport config
let config = TransportConfig::Stdio {
    command: "python".into(),
    args: vec!["-m", "my_mcp_server"].into_iter().map(String::from).collect(),
    env: HashMap::from([("DEBUG".into(), "1".into())]),
};

let builder = SageBuilder::new()
    .with_mcp_server("custom", config);
```

## Lifecycle Hooks

```rust
let builder = SageBuilder::new()
    .with_hook(Arc::new(LoggingHook::all_phases()))
    .with_hook(Arc::new(MetricsHook::new()))
    .with_hook(Arc::new(CustomValidationHook::new()));
```

## SageComponents

```rust
pub struct SageComponents {
    pub tool_executor: ToolExecutor,
    pub batch_executor: BatchToolExecutor,
    pub lifecycle_manager: LifecycleManager,
    pub event_bus: EventBus,
    pub cancellation: CancellationHierarchy,
    pub trajectory_recorder: Option<Arc<Mutex<TrajectoryRecorder>>>,
    pub mcp_registry: McpRegistry,
    pub config: Option<Config>,
    pub max_steps: u32,
    pub working_dir: Option<PathBuf>,
}

impl SageComponents {
    pub fn shared_event_bus(&self) -> Arc<EventBus>;
    pub fn lifecycle_registry(&self) -> Arc<LifecycleHookRegistry>;
    pub async fn initialize(&self) -> SageResult<()>;
    pub async fn shutdown(&self) -> SageResult<()>;
}
```

## Convenience Builders

```rust
// Minimal configurations
let builder = SageBuilder::minimal_openai("key", "gpt-4");
let builder = SageBuilder::minimal_anthropic("key", "claude-3-opus");
let builder = SageBuilder::minimal_google("key", "gemini-pro");

// Development mode (with logging and metrics)
let builder = SageBuilder::development()
    .with_anthropic("key")
    .with_model("claude-3-sonnet");

// Production mode (optimized settings)
let builder = SageBuilder::production()
    .with_anthropic("key")
    .with_model("claude-3-opus");
```

## Complete Example

```rust
use sage_core::{
    SageBuilder, SageComponents, LoggingHook, MetricsHook,
    TransportConfig, CacheConfig,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build all components
    let components: SageComponents = SageBuilder::new()
        // Provider
        .with_anthropic(std::env::var("ANTHROPIC_API_KEY")?)

        // Model
        .with_model("claude-3-opus-20240229")
        .with_temperature(0.7)
        .with_max_tokens(4096)

        // Tools
        .with_tools(vec![
            Arc::new(BashTool::new()),
            Arc::new(EditTool::new()),
        ])

        // MCP servers
        .with_mcp_stdio_server(
            "filesystem",
            "npx",
            vec!["-y", "@modelcontextprotocol/server-filesystem", "."],
        )

        // Lifecycle
        .with_hook(Arc::new(LoggingHook::all_phases()))
        .with_hook(Arc::new(MetricsHook::new()))

        // Infrastructure
        .with_trajectory_path("./trajectories")
        .with_cache()
        .with_event_bus_capacity(5000)
        .with_max_steps(50)
        .with_working_dir(".")

        // Build
        .build()
        .await?;

    // Initialize
    components.initialize().await?;

    // Use components...
    let mcp_tools = components.mcp_registry.as_tools().await;
    println!("Loaded {} MCP tools", mcp_tools.len());

    // Subscribe to events
    let mut rx = components.event_bus.subscribe();
    tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            println!("Event: {:?}", event);
        }
    });

    // Shutdown
    components.shutdown().await?;

    Ok(())
}
```

## Build Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `build()` | `SageComponents` | All components |
| `build_llm_client()` | `LLMClient` | Just LLM client |
| `build_tool_executor()` | `ToolExecutor` | Sequential executor |
| `build_batch_executor()` | `BatchToolExecutor` | Batch executor |
| `build_mcp_registry()` | `McpRegistry` | MCP registry |
| `build_lifecycle_manager()` | `LifecycleManager` | Lifecycle manager |
| `build_event_bus()` | `EventBus` | Event bus |
| `build_cancellation_hierarchy()` | `CancellationHierarchy` | Cancellation |
| `build_trajectory_recorder()` | `Option<TrajectoryRecorder>` | Trajectory |
| `build_claude_style_agent()` | `ClaudeStyleAgent` | Reactive agent |

## Error Handling

```rust
pub enum BuilderError {
    MissingConfig(String),
    InvalidConfig(String),
    InitFailed(String),
    ProviderNotConfigured(String),
}

// Errors are automatically converted to SageError
let result = SageBuilder::new()
    .build_llm_client(); // Returns SageResult<LLMClient>
```
