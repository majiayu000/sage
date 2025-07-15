# MCP Integration Plan for Sage Agent

## Overview

This document outlines the implementation plan for integrating Model Context Protocol (MCP) support into Sage Agent, enabling standardized communication with external tools and services.

## Phase 1: Core MCP Infrastructure

### 1.1 MCP Protocol Implementation

Create `crates/sage-core/src/mcp/` module structure:

```
crates/sage-core/src/mcp/
├── mod.rs              # Module exports and public API
├── protocol.rs         # MCP message types and protocol
├── transport/          # Transport layer implementations
│   ├── mod.rs
│   ├── stdio.rs        # Standard I/O transport
│   ├── http.rs         # HTTP transport
│   └── websocket.rs    # WebSocket transport
├── client.rs           # MCP client implementation
├── server.rs           # MCP server capabilities
├── registry.rs         # Tool and resource registry
├── types.rs            # MCP-specific types
└── error.rs            # MCP error handling
```

### 1.2 Core MCP Types

```rust
// protocol.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum McpMessage {
    Request(McpRequest),
    Response(McpResponse),
    Notification(McpNotification),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    pub id: String,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponse {
    pub id: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<McpError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}
```

### 1.3 Transport Layer

```rust
// transport/mod.rs
#[async_trait]
pub trait McpTransport: Send + Sync {
    async fn send(&mut self, message: McpMessage) -> Result<(), McpError>;
    async fn receive(&mut self) -> Result<McpMessage, McpError>;
    async fn close(&mut self) -> Result<(), McpError>;
}

// transport/stdio.rs
pub struct StdioTransport {
    stdin: tokio::io::Stdin,
    stdout: tokio::io::Stdout,
}

// transport/http.rs
pub struct HttpTransport {
    client: reqwest::Client,
    base_url: String,
}

// transport/websocket.rs
pub struct WebSocketTransport {
    socket: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
}
```

## Phase 2: MCP Client Implementation

### 2.1 MCP Client

```rust
// client.rs
pub struct McpClient {
    transport: Box<dyn McpTransport>,
    tools: HashMap<String, McpTool>,
    resources: HashMap<String, McpResource>,
    request_id_counter: AtomicU64,
}

impl McpClient {
    pub async fn new(transport: Box<dyn McpTransport>) -> Result<Self, McpError> {
        // Initialize client and perform handshake
    }
    
    pub async fn list_tools(&mut self) -> Result<Vec<McpTool>, McpError> {
        // Request available tools from MCP server
    }
    
    pub async fn call_tool(&mut self, name: &str, arguments: serde_json::Value) -> Result<serde_json::Value, McpError> {
        // Execute tool call through MCP
    }
    
    pub async fn list_resources(&mut self) -> Result<Vec<McpResource>, McpError> {
        // Request available resources from MCP server
    }
    
    pub async fn read_resource(&mut self, uri: &str) -> Result<String, McpError> {
        // Read resource content through MCP
    }
}
```

### 2.2 MCP Tool Adapter

```rust
// Create adapter to bridge Sage tools with MCP
pub struct McpToolAdapter {
    client: Arc<Mutex<McpClient>>,
    tool_name: String,
    tool_schema: McpTool,
}

#[async_trait]
impl Tool for McpToolAdapter {
    fn name(&self) -> &str {
        &self.tool_name
    }
    
    fn description(&self) -> &str {
        &self.tool_schema.description
    }
    
    async fn execute(&self, call: &ToolCall) -> ToolResult {
        let mut client = self.client.lock().await;
        match client.call_tool(&self.tool_name, call.arguments.clone()).await {
            Ok(result) => ToolResult::success(&call.id, &call.name, result.to_string()),
            Err(e) => ToolResult::error(&call.id, &call.name, e.to_string()),
        }
    }
    
    fn schema(&self) -> ToolSchema {
        // Convert MCP schema to Sage tool schema
        ToolSchema {
            name: self.tool_name.clone(),
            description: self.tool_schema.description.clone(),
            parameters: self.tool_schema.input_schema.clone(),
        }
    }
}
```

## Phase 3: Integration with Sage Agent

### 3.1 MCP Registry

```rust
// registry.rs
pub struct McpRegistry {
    servers: HashMap<String, McpClient>,
    tools: HashMap<String, McpToolAdapter>,
    resources: HashMap<String, McpResource>,
}

impl McpRegistry {
    pub async fn discover_servers(&mut self) -> Result<(), McpError> {
        // Discover MCP servers in system PATH
        // Parse configuration files for server definitions
        // Connect to discovered servers
    }
    
    pub async fn register_server(&mut self, name: String, transport: Box<dyn McpTransport>) -> Result<(), McpError> {
        // Register a new MCP server
        let client = McpClient::new(transport).await?;
        self.servers.insert(name, client);
        self.refresh_tools_and_resources().await?;
        Ok(())
    }
    
    pub fn get_tools(&self) -> Vec<Arc<dyn Tool>> {
        self.tools.values()
            .map(|adapter| Arc::new(adapter.clone()) as Arc<dyn Tool>)
            .collect()
    }
}
```

### 3.2 Configuration Support

```rust
// Add to crates/sage-core/src/config/model.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    pub enabled: bool,
    pub servers: Vec<McpServerConfig>,
    pub discovery: McpDiscoveryConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub transport: McpTransportConfig,
    pub auto_connect: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpTransportConfig {
    Stdio { command: String, args: Vec<String> },
    Http { base_url: String, headers: HashMap<String, String> },
    WebSocket { url: String },
}
```

## Phase 4: Advanced Features

### 4.1 Resource Management

```rust
// Add resource support to MCP integration
pub struct McpResourceManager {
    registry: Arc<McpRegistry>,
    cache: HashMap<String, (String, SystemTime)>, // URI -> (content, timestamp)
}

impl McpResourceManager {
    pub async fn get_resource(&mut self, uri: &str) -> Result<String, McpError> {
        // Check cache first, then fetch from MCP server
    }
    
    pub async fn watch_resource(&mut self, uri: &str) -> Result<tokio::sync::mpsc::Receiver<String>, McpError> {
        // Set up resource watching for changes
    }
}
```

### 4.2 Tool Chaining

```rust
// Support for MCP tool chaining and workflows
pub struct McpWorkflow {
    steps: Vec<McpWorkflowStep>,
    context: HashMap<String, serde_json::Value>,
}

pub struct McpWorkflowStep {
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub condition: Option<String>, // JavaScript-like condition
    pub output_mapping: HashMap<String, String>,
}
```

## Implementation Timeline

### Week 1-2: Core Infrastructure
- [ ] Implement basic MCP protocol types
- [ ] Create stdio transport layer
- [ ] Build basic MCP client

### Week 3-4: Tool Integration
- [ ] Implement MCP tool adapter
- [ ] Create tool registry system
- [ ] Add configuration support

### Week 5-6: Advanced Transports
- [ ] Implement HTTP transport
- [ ] Add WebSocket transport
- [ ] Create transport auto-detection

### Week 7-8: Resource Management
- [ ] Implement resource reading
- [ ] Add resource caching
- [ ] Create resource watching

### Week 9-10: Testing & Polish
- [ ] Comprehensive testing suite
- [ ] Documentation and examples
- [ ] Performance optimization

## Testing Strategy

### Unit Tests
- Protocol message serialization/deserialization
- Transport layer functionality
- Tool adapter behavior
- Registry operations

### Integration Tests
- End-to-end MCP communication
- Multi-server scenarios
- Error handling and recovery
- Performance under load

### Example MCP Servers
Create example MCP servers for testing:
- File system operations
- Git repository management
- HTTP API client
- Database operations

## Documentation Requirements

- MCP integration guide
- Server configuration examples
- Tool development tutorial
- Troubleshooting guide
- Performance tuning tips

---

This plan provides a comprehensive roadmap for MCP integration while maintaining compatibility with existing Sage Agent functionality.
