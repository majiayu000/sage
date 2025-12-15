# MCP Integration

## Overview

Model Context Protocol (MCP) is a JSON-RPC based protocol for extending LLM agents with external tool servers.

## Protocol Specification

### Message Types

```rust
pub enum McpMessage {
    Request(McpRequest),
    Response(McpResponse),
    Notification(McpNotification),
}

pub struct McpRequest {
    pub jsonrpc: String,  // "2.0"
    pub id: RequestId,
    pub method: String,
    pub params: Option<Value>,
}

pub struct McpResponse {
    pub jsonrpc: String,
    pub id: RequestId,
    pub result: Option<Value>,
    pub error: Option<McpError>,
}

pub struct McpNotification {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
}
```

### Standard Methods

| Method | Description |
|--------|-------------|
| `initialize` | Initialize connection |
| `notifications/initialized` | Confirm initialization |
| `ping` | Health check |
| `tools/list` | List available tools |
| `tools/call` | Execute a tool |
| `resources/list` | List available resources |
| `resources/read` | Read a resource |
| `prompts/list` | List prompt templates |
| `prompts/get` | Get a prompt |

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                       Sage Agent                             │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────────────────────────────────────────────────┐    │
│  │                   McpRegistry                        │    │
│  │  ┌─────────────┐  ┌─────────────┐  ┌────────────┐   │    │
│  │  │ tool_mapping│  │resource_map │  │prompt_map  │   │    │
│  │  │ DashMap     │  │  DashMap    │  │  DashMap   │   │    │
│  │  └─────────────┘  └─────────────┘  └────────────┘   │    │
│  │                                                      │    │
│  │  ┌─────────────────────────────────────────────┐    │    │
│  │  │              clients: DashMap                │    │    │
│  │  │  ┌─────────┐  ┌─────────┐  ┌─────────┐      │    │    │
│  │  │  │McpClient│  │McpClient│  │McpClient│      │    │    │
│  │  │  │(server1)│  │(server2)│  │(server3)│      │    │    │
│  │  │  └────┬────┘  └────┬────┘  └────┬────┘      │    │    │
│  │  └───────┼────────────┼───────────┼────────────┘    │    │
│  └──────────┼────────────┼───────────┼─────────────────┘    │
│             │            │           │                       │
└─────────────┼────────────┼───────────┼───────────────────────┘
              │            │           │
              ▼            ▼           ▼
        ┌──────────┐ ┌──────────┐ ┌──────────┐
        │  Stdio   │ │  HTTP    │ │WebSocket │
        │Transport │ │Transport │ │Transport │
        └────┬─────┘ └────┬─────┘ └────┬─────┘
             │            │           │
             ▼            ▼           ▼
        ┌──────────┐ ┌──────────┐ ┌──────────┐
        │MCP Server│ │MCP Server│ │MCP Server│
        │(Process) │ │ (HTTP)   │ │  (WS)    │
        └──────────┘ └──────────┘ └──────────┘
```

## Transport Layer

### McpTransport Trait

```rust
#[async_trait]
pub trait McpTransport: Send + Sync {
    async fn send(&mut self, message: McpMessage) -> Result<(), McpError>;
    async fn receive(&mut self) -> Result<McpMessage, McpError>;
    async fn close(&mut self) -> Result<(), McpError>;
    fn is_connected(&self) -> bool;
}
```

### Stdio Transport

```rust
pub struct StdioTransport {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout: Option<BufReader<ChildStdout>>,
    line_buffer: String,
    connected: bool,
}

impl StdioTransport {
    pub async fn spawn(
        command: impl AsRef<str>,
        args: &[impl AsRef<str>],
    ) -> Result<Self, McpError>;

    pub async fn spawn_with_env(
        command: impl AsRef<str>,
        args: &[impl AsRef<str>],
        env: &HashMap<String, String>,
    ) -> Result<Self, McpError>;
}
```

### Transport Configuration

```rust
pub enum TransportConfig {
    Stdio {
        command: String,
        args: Vec<String>,
        env: HashMap<String, String>,
    },
    Http {
        base_url: String,
        headers: HashMap<String, String>,
    },
    WebSocket {
        url: String,
    },
}
```

## MCP Client

```rust
pub struct McpClient {
    transport: Mutex<Box<dyn McpTransport>>,
    server_info: RwLock<Option<McpServerInfo>>,
    capabilities: RwLock<McpCapabilities>,
    tools: RwLock<Vec<McpTool>>,
    resources: RwLock<Vec<McpResource>>,
    prompts: RwLock<Vec<McpPrompt>>,
    request_id: AtomicU64,
    initialized: RwLock<bool>,
}

impl McpClient {
    // Lifecycle
    pub async fn initialize(&self) -> Result<McpServerInfo, McpError>;
    pub async fn close(&self) -> Result<(), McpError>;
    pub async fn ping(&self) -> Result<(), McpError>;

    // Tools
    pub async fn list_tools(&self) -> Result<Vec<McpTool>, McpError>;
    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<McpToolResult, McpError>;

    // Resources
    pub async fn list_resources(&self) -> Result<Vec<McpResource>, McpError>;
    pub async fn read_resource(&self, uri: &str) -> Result<McpResourceContent, McpError>;

    // Prompts
    pub async fn list_prompts(&self) -> Result<Vec<McpPrompt>, McpError>;
    pub async fn get_prompt(&self, name: &str, arguments: Option<HashMap<String, String>>) -> Result<Vec<McpPromptMessage>, McpError>;

    // Cached access
    pub async fn cached_tools(&self) -> Vec<McpTool>;
    pub async fn cached_resources(&self) -> Vec<McpResource>;
    pub async fn cached_prompts(&self) -> Vec<McpPrompt>;
}
```

## MCP Registry

```rust
pub struct McpRegistry {
    clients: DashMap<String, Arc<McpClient>>,
    tool_mapping: DashMap<String, String>,      // tool_name -> server_name
    resource_mapping: DashMap<String, String>,   // uri -> server_name
    prompt_mapping: DashMap<String, String>,     // prompt_name -> server_name
}

impl McpRegistry {
    // Server management
    pub async fn register_server(&self, name: impl Into<String>, config: TransportConfig) -> Result<McpServerInfo, McpError>;
    pub async fn unregister_server(&self, name: &str) -> Result<(), McpError>;
    pub fn get_client(&self, name: &str) -> Option<Arc<McpClient>>;
    pub fn server_names(&self) -> Vec<String>;

    // Aggregated access
    pub async fn all_tools(&self) -> Vec<McpTool>;
    pub async fn all_resources(&self) -> Vec<McpResource>;
    pub async fn all_prompts(&self) -> Vec<McpPrompt>;

    // Tool operations
    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<String, McpError>;
    pub async fn read_resource(&self, uri: &str) -> Result<String, McpError>;

    // Sage integration
    pub async fn as_tools(&self) -> Vec<Arc<dyn Tool>>;

    // Cleanup
    pub async fn close_all(&self) -> Result<(), McpError>;
}
```

## Integration with SageBuilder

```rust
let components = SageBuilder::new()
    .with_anthropic("key")
    .with_mcp_stdio_server(
        "filesystem",
        "npx",
        vec!["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
    )
    .with_mcp_stdio_server(
        "git",
        "uvx",
        vec!["mcp-server-git"]
    )
    .build()
    .await?;

// MCP tools are automatically available
let mcp_tools = components.mcp_registry.as_tools().await;
```

## Protocol Flow

```
┌────────────┐                              ┌────────────┐
│   Client   │                              │   Server   │
└─────┬──────┘                              └─────┬──────┘
      │                                           │
      │  initialize                               │
      │  {protocolVersion, capabilities,          │
      │   clientInfo}                             │
      │──────────────────────────────────────────▶│
      │                                           │
      │  response                                 │
      │  {protocolVersion, capabilities,          │
      │   serverInfo}                             │
      │◀──────────────────────────────────────────│
      │                                           │
      │  notifications/initialized                │
      │  {}                                       │
      │──────────────────────────────────────────▶│
      │                                           │
      │  tools/list                               │
      │  {}                                       │
      │──────────────────────────────────────────▶│
      │                                           │
      │  response                                 │
      │  {tools: [...]}                           │
      │◀──────────────────────────────────────────│
      │                                           │
      │  tools/call                               │
      │  {name, arguments}                        │
      │──────────────────────────────────────────▶│
      │                                           │
      │  response                                 │
      │  {content: [...], isError}                │
      │◀──────────────────────────────────────────│
      │                                           │
```

## Error Handling

```rust
pub enum McpError {
    Connection(String),
    Transport(String),
    Protocol(String),
    Server { code: i32, message: String },
    Timeout,
    NotInitialized,
    AlreadyInitialized,
    ToolNotFound(String),
    ResourceNotFound(String),
    PromptNotFound(String),
    Serialization(String),
}
```
