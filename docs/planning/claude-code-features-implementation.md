# Claude Code 功能实现分析

基于 open-claude-code 源码分析，为 Sage Agent 提供实现参考。

## 1. MCP (Model Context Protocol) 实现

### Claude Code 架构

```
src/mcp/
├── client.js          # MCP 客户端管理
├── server.js          # MCP 服务器连接
├── transport.js       # 传输层 (stdio/sse)
├── tools.js           # MCP 工具集成
└── resources.js       # MCP 资源管理
```

### 核心概念

1. **MCP Scope (配置范围)**
   - `local`: 项目本地配置 (私有)
   - `project`: 项目配置 (通过 .mcp.json 共享)
   - `user`: 用户配置 (全局)
   - `dynamic`: 命令行动态配置
   - `enterprise`: 企业管理配置

2. **MCP Server 状态**
   - `connected`: 已连接
   - `pending`: 连接中
   - `disabled`: 已禁用
   - `failed`: 连接失败

3. **MCP 能力**
   - `tools`: 工具调用能力
   - `resources`: 资源访问能力
   - `prompts`: 提示模板能力

### 工具命名规范

```javascript
// MCP 工具名称格式
`mcp__${serverName}__${toolName}`

// 解析工具名
function parseMcpToolName(name) {
  const parts = name.split("__");
  if (parts[0] !== "mcp") return null;
  return {
    serverName: parts[1],
    toolName: parts.slice(2).join("__")
  };
}
```

### Sage 实现建议

```rust
// crates/sage-mcp/src/lib.rs
pub mod client;
pub mod server;
pub mod transport;
pub mod tools;
pub mod resources;

// MCP 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}

// MCP 客户端
pub struct McpClient {
    servers: HashMap<String, McpServerConnection>,
    tools: Vec<McpTool>,
    resources: Vec<McpResource>,
}
```

---

## 2. LSP (Language Server Protocol) 集成

### Claude Code 架构

```
// LSP 服务器配置 (.lsp.json)
{
  "typescript-language-server": {
    "command": "typescript-language-server",
    "args": ["--stdio"],
    "extensionToLanguage": {
      ".ts": "typescript",
      ".tsx": "typescriptreact"
    }
  }
}
```

### 核心功能

1. **服务器管理器**
   - 自动发现项目语言
   - 按需启动 LSP 服务器
   - 连接池管理

2. **诊断信息**
   - 实时获取代码错误/警告
   - 缓存诊断结果
   - 传递给 LLM 作为上下文

3. **代码智能**
   - 定义跳转
   - 引用查找
   - 符号搜索

### Sage 实现建议

```rust
// crates/sage-lsp/src/lib.rs

pub struct LspServerManager {
    servers: HashMap<String, LspServerWrapper>,
    diagnostics: DiagnosticsCache,
}

pub struct LspServerWrapper {
    config: LspServerConfig,
    client: LspClient,
    extensions: Vec<String>,
}

// LSP 工具
pub struct LspTool {
    server_manager: Arc<LspServerManager>,
}

impl Tool for LspTool {
    async fn execute(&self, input: Value) -> Result<ToolOutput> {
        match input["action"].as_str() {
            Some("diagnostics") => self.get_diagnostics(&input).await,
            Some("definition") => self.goto_definition(&input).await,
            Some("references") => self.find_references(&input).await,
            Some("symbols") => self.search_symbols(&input).await,
            _ => Err(ToolError::InvalidInput),
        }
    }
}
```

---

## 3. 多会话 & 持久化

### Claude Code 架构

```
~/.claude/
├── sessions/
│   ├── {session_id}.json       # 会话消息
│   └── {session_id}_meta.json  # 会话元数据
├── memory/
│   └── {project_hash}.md       # 项目记忆
└── settings.json               # 用户设置
```

### 会话数据结构

```javascript
// 会话文件结构
{
  "messages": [...],           // 消息历史
  "summaries": [...],          // 摘要信息
  "customTitles": {...},       // 自定义标题
  "fileHistorySnapshots": [...] // 文件历史快照
}

// 会话元数据
{
  "id": "session_id",
  "created": "2025-01-01T00:00:00Z",
  "modified": "2025-01-01T01:00:00Z",
  "messageCount": 42,
  "gitBranch": "main",
  "projectPath": "/path/to/project",
  "isSidechain": false
}
```

### 核心功能

1. **会话管理**
   - 创建/恢复会话
   - 会话列表 (按项目/分支过滤)
   - 会话搜索

2. **持久化**
   - 本地文件存储
   - OAuth 云同步 (可选)
   - 自动保存

3. **会话记忆**
   - 自动摘要长会话
   - 项目级记忆 (跨会话)
   - 上下文注入

### Sage 实现建议

```rust
// crates/sage-session/src/lib.rs

pub struct SessionManager {
    storage: SessionStorage,
    current: Option<Session>,
    memory: ProjectMemory,
}

pub struct Session {
    pub id: String,
    pub messages: Vec<Message>,
    pub summaries: Vec<Summary>,
    pub metadata: SessionMetadata,
}

pub struct SessionMetadata {
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub message_count: usize,
    pub git_branch: Option<String>,
    pub project_path: PathBuf,
}

// 会话存储
pub trait SessionStorage: Send + Sync {
    async fn save(&self, session: &Session) -> Result<()>;
    async fn load(&self, id: &str) -> Result<Session>;
    async fn list(&self, filter: SessionFilter) -> Result<Vec<SessionMetadata>>;
    async fn delete(&self, id: &str) -> Result<()>;
}

// 本地存储实现
pub struct LocalSessionStorage {
    base_path: PathBuf,
}
```

---

## 4. 实现优先级

### Phase 1: 会话持久化 (1-2 周)
- [ ] Session 数据结构
- [ ] LocalSessionStorage 实现
- [ ] CLI 命令: `sage session list/resume/delete`
- [ ] 自动保存/恢复

### Phase 2: MCP 客户端 (2-3 周)
- [ ] MCP 协议实现
- [ ] stdio 传输层
- [ ] 工具注册/调用
- [ ] 配置文件解析 (.mcp.json)

### Phase 3: LSP 集成 (2-3 周)
- [ ] LSP 客户端库集成
- [ ] 服务器管理器
- [ ] 诊断信息获取
- [ ] LSP 工具暴露给 Agent

### Phase 4: 高级功能 (持续)
- [ ] 会话记忆/摘要
- [ ] MCP 资源支持
- [ ] 多会话并行
- [ ] 云同步 (可选)

---

## 5. 依赖库建议

```toml
# Cargo.toml

# MCP
mcp-sdk = "0.1"  # 如果有 Rust SDK
jsonrpc-core = "18"

# LSP
lsp-types = "0.95"
tower-lsp = "0.20"

# 会话存储
sqlx = { version = "0.7", features = ["sqlite"] }
# 或使用 JSON 文件存储

# 序列化
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

---

## 6. 文件结构建议

```
crates/
├── sage-core/          # 核心 (已有)
├── sage-cli/           # CLI (已有)
├── sage-tools/         # 工具 (已有)
├── sage-mcp/           # MCP 客户端 (新增)
│   ├── src/
│   │   ├── lib.rs
│   │   ├── client.rs
│   │   ├── server.rs
│   │   ├── transport/
│   │   │   ├── mod.rs
│   │   │   ├── stdio.rs
│   │   │   └── sse.rs
│   │   ├── tools.rs
│   │   └── resources.rs
│   └── Cargo.toml
├── sage-lsp/           # LSP 集成 (新增)
│   ├── src/
│   │   ├── lib.rs
│   │   ├── manager.rs
│   │   ├── wrapper.rs
│   │   └── diagnostics.rs
│   └── Cargo.toml
└── sage-session/       # 会话管理 (新增)
    ├── src/
    │   ├── lib.rs
    │   ├── session.rs
    │   ├── storage/
    │   │   ├── mod.rs
    │   │   ├── local.rs
    │   │   └── memory.rs
    │   └── memory.rs
    └── Cargo.toml
```

---

## 7. 详细实现参考 (基于 Claude Code 分析)

### 7.1 MCP 客户端实现

```rust
// crates/sage-mcp/src/client.rs

use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// MCP 配置作用域
#[derive(Debug, Clone, PartialEq)]
pub enum McpScope {
    Local,      // 项目本地 (.claude/.mcp.json)
    Project,    // 项目共享 (.mcp.json)
    User,       // 用户全局 (~/.claude/.mcp.json)
    Dynamic,    // 命令行参数
    Enterprise, // 企业配置
}

/// MCP 服务器状态
#[derive(Debug, Clone, PartialEq)]
pub enum ServerStatus {
    Connected,
    Pending,
    Disabled,
    Failed(String),
}

/// MCP 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub command: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default = "default_transport")]
    pub transport: TransportType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransportType {
    Stdio,
    Sse { url: String },
}

fn default_transport() -> TransportType {
    TransportType::Stdio
}

/// MCP 客户端
pub struct McpClient {
    servers: HashMap<String, McpServerConnection>,
    config_paths: Vec<(McpScope, PathBuf)>,
}

impl McpClient {
    pub fn new() -> Self {
        Self {
            servers: HashMap::new(),
            config_paths: vec![],
        }
    }

    /// 加载配置文件 (按优先级)
    pub async fn load_config(&mut self) -> Result<(), McpError> {
        // 按优先级加载: local > project > user
        let paths = [
            (McpScope::Local, ".claude/.mcp.json"),
            (McpScope::Project, ".mcp.json"),
            (McpScope::User, "~/.claude/.mcp.json"),
        ];

        for (scope, path) in paths {
            if let Ok(config) = self.load_config_file(&path).await {
                self.merge_config(scope, config);
            }
        }
        Ok(())
    }

    /// 连接服务器
    pub async fn connect(&mut self, name: &str) -> Result<(), McpError> {
        let config = self.servers.get(name)
            .ok_or(McpError::ServerNotFound(name.to_string()))?;

        match &config.config.transport {
            TransportType::Stdio => self.connect_stdio(name).await,
            TransportType::Sse { url } => self.connect_sse(name, url).await,
        }
    }

    /// 调用 MCP 工具
    pub async fn call_tool(
        &self,
        server: &str,
        tool: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, McpError> {
        let conn = self.servers.get(server)
            .ok_or(McpError::ServerNotFound(server.to_string()))?;

        conn.call_tool(tool, args).await
    }

    /// 获取工具名称 (mcp__server__tool 格式)
    pub fn get_tool_name(server: &str, tool: &str) -> String {
        format!("mcp__{}__{}",
            Self::sanitize_name(server),
            Self::sanitize_name(tool)
        )
    }

    /// 解析工具名称
    pub fn parse_tool_name(name: &str) -> Option<(String, String)> {
        let parts: Vec<&str> = name.split("__").collect();
        if parts.len() >= 3 && parts[0] == "mcp" {
            Some((parts[1].to_string(), parts[2..].join("__")))
        } else {
            None
        }
    }

    fn sanitize_name(name: &str) -> String {
        name.chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect()
    }
}
```

### 7.2 会话持久化实现

```rust
// crates/sage-session/src/session.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 会话元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub id: String,
    pub title: String,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub message_count: usize,
    pub git_branch: Option<String>,
    pub project_path: Option<PathBuf>,
    pub is_sidechain: bool,
}

/// 完整会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub metadata: SessionMetadata,
    pub messages: Vec<Message>,
    pub summaries: Vec<Summary>,
}

/// 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
}

/// 会话存储接口
#[async_trait::async_trait]
pub trait SessionStorage: Send + Sync {
    async fn save(&self, session: &Session) -> Result<(), StorageError>;
    async fn load(&self, id: &str) -> Result<Session, StorageError>;
    async fn list(&self, filter: Option<SessionFilter>) -> Result<Vec<SessionMetadata>, StorageError>;
    async fn delete(&self, id: &str) -> Result<(), StorageError>;
}

/// 本地文件存储
pub struct LocalSessionStorage {
    base_path: PathBuf,
}

impl LocalSessionStorage {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    fn session_path(&self, id: &str) -> PathBuf {
        self.base_path.join("sessions").join(format!("{}.json", id))
    }

    fn metadata_path(&self) -> PathBuf {
        self.base_path.join("sessions.json")
    }
}

#[async_trait::async_trait]
impl SessionStorage for LocalSessionStorage {
    async fn save(&self, session: &Session) -> Result<(), StorageError> {
        // 保存会话内容
        let path = self.session_path(&session.metadata.id);
        tokio::fs::create_dir_all(path.parent().unwrap()).await?;
        let content = serde_json::to_string_pretty(session)?;
        tokio::fs::write(&path, content).await?;

        // 更新元数据索引
        self.update_metadata_index(&session.metadata).await?;

        Ok(())
    }

    async fn load(&self, id: &str) -> Result<Session, StorageError> {
        let path = self.session_path(id);
        let content = tokio::fs::read_to_string(&path).await?;
        Ok(serde_json::from_str(&content)?)
    }

    async fn list(&self, filter: Option<SessionFilter>) -> Result<Vec<SessionMetadata>, StorageError> {
        let path = self.metadata_path();
        if !path.exists() {
            return Ok(vec![]);
        }

        let content = tokio::fs::read_to_string(&path).await?;
        let mut sessions: Vec<SessionMetadata> = serde_json::from_str(&content)?;

        // 应用过滤器
        if let Some(f) = filter {
            sessions = sessions.into_iter()
                .filter(|s| f.matches(s))
                .collect();
        }

        // 按修改时间排序
        sessions.sort_by(|a, b| b.modified.cmp(&a.modified));

        Ok(sessions)
    }

    async fn delete(&self, id: &str) -> Result<(), StorageError> {
        let path = self.session_path(id);
        tokio::fs::remove_file(&path).await?;
        self.remove_from_metadata_index(id).await?;
        Ok(())
    }
}

/// 会话过滤器
#[derive(Debug, Default)]
pub struct SessionFilter {
    pub project_path: Option<PathBuf>,
    pub git_branch: Option<String>,
    pub search_query: Option<String>,
}

impl SessionFilter {
    pub fn matches(&self, session: &SessionMetadata) -> bool {
        if let Some(ref path) = self.project_path {
            if session.project_path.as_ref() != Some(path) {
                return false;
            }
        }
        if let Some(ref branch) = self.git_branch {
            if session.git_branch.as_ref() != Some(branch) {
                return false;
            }
        }
        if let Some(ref query) = self.search_query {
            let query = query.to_lowercase();
            if !session.title.to_lowercase().contains(&query) {
                return false;
            }
        }
        true
    }
}
```

### 7.3 CLI 命令扩展

```rust
// crates/sage-cli/src/commands/session.rs

use clap::Subcommand;

#[derive(Subcommand)]
pub enum SessionCommand {
    /// 列出所有会话
    List {
        #[arg(short, long)]
        project: Option<String>,

        #[arg(short, long)]
        branch: Option<String>,

        #[arg(short, long)]
        all: bool,
    },

    /// 恢复会话
    Resume {
        /// 会话 ID
        id: String,
    },

    /// 删除会话
    Delete {
        /// 会话 ID
        id: String,

        #[arg(short, long)]
        force: bool,
    },

    /// 重命名会话
    Rename {
        id: String,
        title: String,
    },
}

pub async fn handle_session_command(cmd: SessionCommand) -> Result<()> {
    let storage = LocalSessionStorage::new(get_sage_config_dir());

    match cmd {
        SessionCommand::List { project, branch, all } => {
            let filter = if all {
                None
            } else {
                Some(SessionFilter {
                    project_path: project.map(PathBuf::from),
                    git_branch: branch,
                    ..Default::default()
                })
            };

            let sessions = storage.list(filter).await?;

            for session in sessions {
                println!("{} - {} ({} messages)",
                    session.id,
                    session.title,
                    session.message_count
                );
            }
        }
        SessionCommand::Resume { id } => {
            let session = storage.load(&id).await?;
            // 恢复会话上下文...
        }
        // ...其他命令
    }

    Ok(())
}
```
