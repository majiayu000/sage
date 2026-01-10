---
name: sage-mcp-protocol
description: Sage MCP 协议开发指南，涵盖客户端、传输层、服务发现、通知处理
when_to_use: 当涉及 MCP 服务器连接、工具发现、协议消息、传输层时使用
allowed_tools:
  - Read
  - Grep
  - Glob
  - Edit
  - Write
  - Bash
user_invocable: true
priority: 90
---

# Sage MCP 协议开发指南

## 模块概览

MCP (Model Context Protocol) 模块提供与外部工具服务器的集成，代码量 **5194 行**，包含：

```
crates/sage-core/src/mcp/
├── mod.rs              # 公开接口 (64行)
├── client.rs           # McpClient 实现
├── protocol.rs         # 协议消息类型 (364行)
├── types.rs            # 类型定义 (344行)
├── registry.rs         # McpRegistry (345行)
├── error.rs            # 错误类型 (277行)
├── transport/          # 传输层
│   ├── mod.rs          # 入口 (84行)
│   ├── stdio.rs        # Stdio 传输 (207行)
│   └── http.rs         # HTTP 传输 (361行)
├── discovery/          # 服务发现
│   ├── mod.rs          # 入口
│   ├── manager.rs      # McpServerManager
│   ├── builder.rs      # 构建器
│   ├── scanner.rs      # 路径扫描
│   ├── connection.rs   # 连接管理
│   ├── health.rs       # 健康检查
│   ├── types.rs        # 类型定义
│   ├── utils.rs        # 工具函数
│   └── tests.rs        # 测试
├── cache/              # 缓存系统
│   ├── mod.rs          # 入口
│   ├── cache.rs        # McpCache
│   ├── eviction.rs     # 淘汰策略
│   ├── types.rs        # 类型定义
│   └── tests.rs        # 测试
├── schema_translator/  # Schema 转换
│   ├── mod.rs          # 入口
│   ├── translator.rs   # SchemaTranslator (145行)
│   ├── converters.rs   # 转换器 (200行)
│   ├── types.rs        # 类型
│   └── tests.rs        # 测试
└── notifications/      # 通知处理
    ├── mod.rs          # 入口
    ├── handlers.rs     # 处理器 (217行)
    ├── processor.rs    # 处理器 (170行)
    ├── types.rs        # 类型 (127行)
    └── tests.rs        # 测试
```

---

## 一、MCP 协议概述

### 1.1 协议版本

```rust
// crates/sage-core/src/mcp/protocol.rs
pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";
pub const JSONRPC_VERSION: &str = "2.0";
```

### 1.2 消息类型

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum McpMessage {
    /// 请求消息
    Request(McpRequest),
    /// 响应消息
    Response(McpResponse),
    /// 通知消息（无 id）
    Notification(McpNotification),
}

/// JSON-RPC 请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: RequestId,
    pub method: String,
    pub params: Option<Value>,
}

/// JSON-RPC 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponse {
    pub jsonrpc: String,
    pub id: RequestId,
    pub result: Option<Value>,
    pub error: Option<McpRpcError>,
}

/// JSON-RPC 通知
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpNotification {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
}
```

### 1.3 MCP 方法

```rust
pub mod methods {
    // 生命周期
    pub const INITIALIZE: &str = "initialize";
    pub const INITIALIZED: &str = "notifications/initialized";

    // 工具
    pub const TOOLS_LIST: &str = "tools/list";
    pub const TOOLS_CALL: &str = "tools/call";

    // 资源
    pub const RESOURCES_LIST: &str = "resources/list";
    pub const RESOURCES_READ: &str = "resources/read";
    pub const RESOURCES_SUBSCRIBE: &str = "resources/subscribe";

    // Prompt
    pub const PROMPTS_LIST: &str = "prompts/list";
    pub const PROMPTS_GET: &str = "prompts/get";

    // 通知
    pub const TOOLS_LIST_CHANGED: &str = "notifications/tools/list_changed";
    pub const RESOURCES_LIST_CHANGED: &str = "notifications/resources/list_changed";
    pub const RESOURCES_UPDATED: &str = "notifications/resources/updated";
}
```

---

## 二、McpClient

### 2.1 客户端结构

```rust
// crates/sage-core/src/mcp/client.rs
pub struct McpClient {
    /// 传输层
    transport: Arc<Mutex<Box<dyn McpTransport>>>,
    /// 服务器信息
    server_info: RwLock<Option<McpServerInfo>>,
    /// 服务器能力
    capabilities: RwLock<McpCapabilities>,
    /// 缓存的工具
    tools: RwLock<Vec<McpTool>>,
    /// 缓存的资源
    resources: RwLock<Vec<McpResource>>,
    /// 缓存的 Prompt
    prompts: RwLock<Vec<McpPrompt>>,
    /// 请求 ID 计数器
    request_id: AtomicU64,
    /// 后台接收器命令通道
    command_sender: mpsc::Sender<ReceiverCommand>,
    /// 是否已初始化
    initialized: RwLock<bool>,
    /// 是否正在运行
    running: Arc<AtomicBool>,
    /// 请求超时
    request_timeout: Duration,
    /// 通知处理器
    notification_handler: RwLock<Option<Box<dyn NotificationHandler>>>,
}
```

### 2.2 初始化流程

```rust
impl McpClient {
    /// 初始化 MCP 连接
    pub async fn initialize(&self) -> Result<InitializeResult, McpError> {
        // 1. 发送 initialize 请求
        let params = InitializeParams {
            protocol_version: MCP_PROTOCOL_VERSION.to_string(),
            capabilities: ClientCapabilities::default(),
            client_info: ClientInfo {
                name: "sage-agent".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };

        let response = self.send_request(methods::INITIALIZE, Some(json!(params))).await?;

        // 2. 解析服务器信息和能力
        let result: InitializeResult = serde_json::from_value(response)?;

        *self.server_info.write().await = Some(result.server_info.clone());
        *self.capabilities.write().await = result.capabilities.clone();

        // 3. 发送 initialized 通知
        self.send_notification(methods::INITIALIZED, None).await?;

        // 4. 缓存工具/资源/Prompt 列表
        self.refresh_caches().await?;

        *self.initialized.write().await = true;
        Ok(result)
    }
}
```

### 2.3 工具调用

```rust
impl McpClient {
    /// 列出可用工具
    pub async fn list_tools(&self) -> Result<Vec<McpTool>, McpError> {
        let response = self.send_request(methods::TOOLS_LIST, None).await?;
        let result: ToolsListResult = serde_json::from_value(response)?;
        *self.tools.write().await = result.tools.clone();
        Ok(result.tools)
    }

    /// 调用工具
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: Value
    ) -> Result<McpToolResult, McpError> {
        let params = json!({
            "name": name,
            "arguments": arguments,
        });

        let response = self.send_request(methods::TOOLS_CALL, Some(params)).await?;
        let result: McpToolResult = serde_json::from_value(response)?;
        Ok(result)
    }
}
```

### 2.4 资源读取

```rust
impl McpClient {
    /// 列出可用资源
    pub async fn list_resources(&self) -> Result<Vec<McpResource>, McpError> {
        let response = self.send_request(methods::RESOURCES_LIST, None).await?;
        let result: ResourcesListResult = serde_json::from_value(response)?;
        *self.resources.write().await = result.resources.clone();
        Ok(result.resources)
    }

    /// 读取资源
    pub async fn read_resource(&self, uri: &str) -> Result<McpResourceContent, McpError> {
        let params = json!({ "uri": uri });
        let response = self.send_request(methods::RESOURCES_READ, Some(params)).await?;
        let result: McpResourceContent = serde_json::from_value(response)?;
        Ok(result)
    }
}
```

---

## 三、传输层

### 3.1 传输 Trait

```rust
// crates/sage-core/src/mcp/transport/mod.rs
#[async_trait]
pub trait McpTransport: Send + Sync {
    /// 发送消息
    async fn send(&mut self, message: &McpMessage) -> Result<(), McpError>;

    /// 接收消息
    async fn receive(&mut self) -> Result<McpMessage, McpError>;

    /// 关闭传输
    async fn close(&mut self) -> Result<(), McpError>;

    /// 检查是否打开
    fn is_open(&self) -> bool;
}
```

### 3.2 Stdio 传输

```rust
// crates/sage-core/src/mcp/transport/stdio.rs
pub struct StdioTransport {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl StdioTransport {
    /// 启动 MCP 服务器进程
    pub async fn spawn(
        command: &str,
        args: &[&str],
    ) -> Result<Self, McpError> {
        let mut child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());

        Ok(Self { child, stdin, stdout })
    }

    /// 使用环境变量启动
    pub async fn spawn_with_env(
        command: &str,
        args: &[&str],
        env: &HashMap<String, String>,
    ) -> Result<Self, McpError>;
}

#[async_trait]
impl McpTransport for StdioTransport {
    async fn send(&mut self, message: &McpMessage) -> Result<(), McpError> {
        let json = serde_json::to_string(message)?;
        self.stdin.write_all(json.as_bytes()).await?;
        self.stdin.write_all(b"\n").await?;
        self.stdin.flush().await?;
        Ok(())
    }

    async fn receive(&mut self) -> Result<McpMessage, McpError> {
        let mut line = String::new();
        self.stdout.read_line(&mut line).await?;
        let message: McpMessage = serde_json::from_str(&line)?;
        Ok(message)
    }
}
```

### 3.3 HTTP 传输

```rust
// crates/sage-core/src/mcp/transport/http.rs
pub struct HttpTransport {
    client: reqwest::Client,
    base_url: String,
    config: HttpTransportConfig,
}

#[derive(Debug, Clone)]
pub struct HttpTransportConfig {
    pub timeout: Duration,
    pub headers: HashMap<String, String>,
    pub auth_token: Option<String>,
}

impl HttpTransport {
    pub fn new(base_url: impl Into<String>, config: HttpTransportConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            config,
        }
    }
}

#[async_trait]
impl McpTransport for HttpTransport {
    async fn send(&mut self, message: &McpMessage) -> Result<(), McpError> {
        let mut request = self.client
            .post(&self.base_url)
            .json(message);

        if let Some(token) = &self.config.auth_token {
            request = request.bearer_auth(token);
        }

        for (key, value) in &self.config.headers {
            request = request.header(key.as_str(), value.as_str());
        }

        request.send().await?;
        Ok(())
    }
}
```

---

## 四、服务发现

### 4.1 McpServerManager

```rust
// crates/sage-core/src/mcp/discovery/manager.rs
pub struct McpServerManager {
    /// 已连接的客户端
    clients: Arc<RwLock<HashMap<String, McpClient>>>,
    /// 服务器配置
    configs: Arc<RwLock<HashMap<String, ServerConfig>>>,
    /// 注册表
    registry: Arc<McpRegistry>,
    /// 健康检查间隔
    health_check_interval: Duration,
}

impl McpServerManager {
    /// 从多个来源发现服务器
    pub async fn discover(&self, sources: Vec<DiscoverySource>) -> Result<(), McpError> {
        for source in sources {
            match source {
                DiscoverySource::Config(path) => {
                    self.discover_from_config(&path).await?;
                }
                DiscoverySource::Environment => {
                    self.discover_from_env().await?;
                }
                DiscoverySource::Standard => {
                    self.discover_from_standard_paths().await?;
                }
                DiscoverySource::Manual(config) => {
                    self.register_server(config).await?;
                }
            }
        }
        Ok(())
    }

    /// 连接到服务器
    pub async fn connect(&self, name: &str) -> Result<(), McpError>;

    /// 断开服务器连接
    pub async fn disconnect(&self, name: &str) -> Result<(), McpError>;

    /// 获取注册表
    pub fn registry(&self) -> Arc<McpRegistry>;

    /// 获取服务器状态
    pub async fn get_status(&self, name: &str) -> Option<ServerStatus>;
}
```

### 4.2 发现来源

```rust
// crates/sage-core/src/mcp/discovery/types.rs
pub enum DiscoverySource {
    /// 从配置文件
    Config(PathBuf),
    /// 从环境变量
    Environment,
    /// 从标准路径
    Standard,
    /// 手动注册
    Manual(ServerConfig),
}

pub struct ServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub transport: TransportType,
    pub auto_connect: bool,
}

pub enum TransportType {
    Stdio,
    Http { url: String },
    WebSocket { url: String },
}
```

### 4.3 标准路径扫描

```rust
// crates/sage-core/src/mcp/discovery/scanner.rs
pub fn get_standard_mcp_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // Claude Code 标准路径
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".claude").join("mcp.json"));
    }

    // Sage 标准路径
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".sage").join("mcp.json"));
    }

    // 项目级路径
    if let Ok(cwd) = std::env::current_dir() {
        paths.push(cwd.join(".sage").join("mcp.json"));
        paths.push(cwd.join("sage_config.json")); // mcp 字段
    }

    paths
}
```

---

## 五、MCP 类型

### 5.1 工具类型

```rust
// crates/sage-core/src/mcp/types.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpTool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolResult {
    pub content: Vec<McpContent>,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum McpContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },
    #[serde(rename = "resource")]
    Resource { resource: McpResourceRef },
}
```

### 5.2 资源类型

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpResourceContent {
    pub uri: String,
    pub mime_type: Option<String>,
    pub text: Option<String>,
    pub blob: Option<String>,
}
```

### 5.3 Prompt 类型

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpPrompt {
    pub name: String,
    pub description: Option<String>,
    pub arguments: Option<Vec<McpPromptArgument>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpPromptArgument {
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpPromptMessage {
    pub role: String,
    pub content: McpContent,
}
```

---

## 六、McpRegistry

### 6.1 注册表结构

```rust
// crates/sage-core/src/mcp/registry.rs
pub struct McpRegistry {
    /// 工具映射：工具名 -> (服务器名, 工具)
    tools: RwLock<HashMap<String, (String, McpTool)>>,
    /// 资源映射
    resources: RwLock<HashMap<String, (String, McpResource)>>,
    /// Prompt 映射
    prompts: RwLock<HashMap<String, (String, McpPrompt)>>,
    /// 服务器到客户端映射
    server_clients: RwLock<HashMap<String, Arc<McpClient>>>,
}

impl McpRegistry {
    /// 注册服务器的工具
    pub async fn register_tools(&self, server: &str, tools: Vec<McpTool>);

    /// 注册服务器的资源
    pub async fn register_resources(&self, server: &str, resources: Vec<McpResource>);

    /// 获取所有工具
    pub async fn all_tools(&self) -> Vec<(String, McpTool)>;

    /// 按名称查找工具
    pub async fn find_tool(&self, name: &str) -> Option<(String, McpTool)>;

    /// 调用工具（自动路由到正确服务器）
    pub async fn call_tool(&self, name: &str, args: Value) -> Result<McpToolResult, McpError>;

    /// 读取资源（自动路由）
    pub async fn read_resource(&self, uri: &str) -> Result<McpResourceContent, McpError>;
}
```

---

## 七、Schema 转换

### 7.1 转换器

```rust
// crates/sage-core/src/mcp/schema_translator/translator.rs
pub struct SchemaTranslator;

impl SchemaTranslator {
    /// MCP 工具转 Sage 工具 Schema
    pub fn mcp_to_sage_tool(tool: &McpTool) -> ToolSchema {
        ToolSchema {
            name: tool.name.clone(),
            description: tool.description.clone().unwrap_or_default(),
            parameters: Self::convert_json_schema(&tool.input_schema),
        }
    }

    /// Sage 工具 Schema 转 MCP 工具
    pub fn sage_to_mcp_tool(tool: &ToolSchema) -> McpTool {
        McpTool {
            name: tool.name.clone(),
            description: Some(tool.description.clone()),
            input_schema: Self::to_json_schema(&tool.parameters),
        }
    }

    /// 转换 JSON Schema
    fn convert_json_schema(schema: &Value) -> ParameterSchema;
}
```

---

## 八、通知处理

### 8.1 通知分发器

```rust
// crates/sage-core/src/mcp/notifications/processor.rs
pub struct NotificationDispatcher {
    handlers: RwLock<HashMap<String, Vec<Box<dyn NotificationHandler>>>>,
}

impl NotificationDispatcher {
    /// 注册处理器
    pub async fn register(&self, method: &str, handler: Box<dyn NotificationHandler>);

    /// 分发通知
    pub async fn dispatch(&self, notification: &McpNotification) {
        let handlers = self.handlers.read().await;

        if let Some(handlers) = handlers.get(&notification.method) {
            for handler in handlers {
                handler.handle(&notification.method, notification.params.clone());
            }
        }
    }
}

pub trait NotificationHandler: Send + Sync {
    fn handle(&self, method: &str, params: Option<Value>);
}
```

### 8.2 内置处理器

```rust
// crates/sage-core/src/mcp/notifications/handlers.rs
/// 工具列表变更处理器
pub struct ToolsListChangedHandler {
    registry: Arc<McpRegistry>,
    client: Arc<McpClient>,
    server_name: String,
}

impl NotificationHandler for ToolsListChangedHandler {
    fn handle(&self, _method: &str, _params: Option<Value>) {
        // 异步刷新工具列表
        let registry = self.registry.clone();
        let client = self.client.clone();
        let server = self.server_name.clone();

        tokio::spawn(async move {
            if let Ok(tools) = client.list_tools().await {
                registry.register_tools(&server, tools).await;
            }
        });
    }
}
```

---

## 九、使用示例

### 9.1 直接使用客户端

```rust
use sage_core::mcp::{McpClient, StdioTransport};
use serde_json::json;

// 启动 MCP 服务器
let transport = StdioTransport::spawn("mcp-server", &["--mode", "stdio"]).await?;
let client = McpClient::new(Box::new(transport));

// 初始化
client.initialize().await?;

// 列出工具
let tools = client.list_tools().await?;
println!("Available tools: {:?}", tools);

// 调用工具
let result = client.call_tool("read_file", json!({
    "path": "/tmp/test.txt"
})).await?;

println!("Result: {:?}", result);
```

### 9.2 使用服务管理器

```rust
use sage_core::mcp::{McpServerManager, DiscoverySource};

let manager = McpServerManager::new();

// 自动发现服务器
manager.discover(vec![
    DiscoverySource::Standard,
    DiscoverySource::Environment,
]).await?;

// 获取注册表
let registry = manager.registry();

// 查找并调用工具
if let Some((server, tool)) = registry.find_tool("read_file").await {
    let result = registry.call_tool("read_file", json!({
        "path": "/tmp/test.txt"
    })).await?;
}
```

---

## 十、开发指南

### 10.1 添加新传输层

1. 实现 `McpTransport` trait：
```rust
pub struct WebSocketTransport {
    ws: WebSocket,
}

#[async_trait]
impl McpTransport for WebSocketTransport {
    async fn send(&mut self, message: &McpMessage) -> Result<(), McpError> {
        // WebSocket 实现
    }

    async fn receive(&mut self) -> Result<McpMessage, McpError> {
        // WebSocket 实现
    }
}
```

2. 在 `TransportType` 添加变体

### 10.2 MCP 配置格式

**mcp.json 示例：**
```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@anthropic/mcp-server-filesystem"],
      "env": {
        "ALLOWED_PATHS": "/home/user/project"
      }
    },
    "github": {
      "command": "npx",
      "args": ["-y", "@anthropic/mcp-server-github"],
      "env": {
        "GITHUB_TOKEN": "${GITHUB_TOKEN}"
      }
    }
  }
}
```

---

## 十一、相关模块

- `sage-tool-development` - 工具开发（MCP 工具集成）
- `sage-config-system` - 配置系统（MCP 配置）
- `sage-session-management` - 会话管理（MCP 缓存）

---

*最后更新: 2026-01-10*
