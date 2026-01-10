---
name: sage-session-management
description: Sage 会话管理开发指南，涵盖会话持久化、分支、缓存、存储后端
when_to_use: 当涉及会话创建、恢复、存储、分支管理、会话缓存时使用
allowed_tools:
  - Read
  - Grep
  - Glob
  - Edit
  - Write
  - Bash
user_invocable: true
priority: 91
---

# Sage 会话管理开发指南

## 模块概览

会话模块提供会话持久化和恢复功能，代码量 **5547 行**，包含：

```
crates/sage-core/src/session/
├── mod.rs              # 公开接口 (100行)
├── manager.rs          # SessionManager (442行)
├── storage.rs          # 存储后端 (285行)
├── conversation.rs     # 消息类型
├── summary.rs          # 摘要生成 (201行)
├── file_tracker.rs     # 文件追踪
├── file_tracking.rs    # 文件追踪实现
├── types/              # 类型定义
│   ├── mod.rs          # 入口
│   ├── base.rs         # 基础类型 (176行)
│   ├── session.rs      # Session 结构 (248行)
│   └── metadata.rs     # 元数据 (147行)
├── branching/          # 分支系统
│   ├── mod.rs          # 入口
│   ├── manager/        # BranchManager
│   │   ├── core.rs     # 核心实现
│   │   ├── operations.rs # 操作方法
│   │   ├── query.rs    # 查询方法
│   │   └── serialization.rs # 序列化
│   ├── tree.rs         # 分支树
│   ├── types.rs        # 类型定义
│   └── tests.rs        # 测试
├── enhanced/           # 增强会话
│   ├── mod.rs          # 入口
│   ├── context.rs      # 增强上下文
│   └── message.rs      # 增强消息
├── session_cache/      # 会话缓存
│   ├── mod.rs          # 入口
│   ├── types.rs        # 类型定义 (149行)
│   ├── manager.rs      # 缓存管理器 (129行)
│   ├── cache_ops.rs    # 缓存操作 (137行)
│   ├── data_ops.rs     # 数据操作 (74行)
│   ├── persistence.rs  # 持久化 (53行)
│   └── tests.rs        # 测试
└── jsonl_storage/      # JSONL 存储
    ├── mod.rs          # 入口
    ├── metadata.rs     # 元数据
    ├── tracker.rs      # 消息追踪 (122行)
    ├── storage/        # 存储实现
    │   ├── core.rs     # 核心
    │   ├── read_ops.rs # 读取操作 (221行)
    │   ├── write_ops.rs # 写入操作 (76行)
    │   └── metadata_ops.rs # 元数据操作
    └── tests.rs        # 测试
```

---

## 一、核心架构：Session

### 1.1 Session 结构

```rust
// crates/sage-core/src/session/types/session.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// 唯一 ID
    pub id: SessionId,

    /// 会话名称
    pub name: Option<String>,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,

    /// 工作目录
    pub working_directory: PathBuf,

    /// 关联的 Git 分支
    pub git_branch: Option<String>,

    /// 对话消息
    pub messages: Vec<ConversationMessage>,

    /// Token 使用统计
    pub token_usage: TokenUsage,

    /// 当前状态
    pub state: SessionState,

    /// 错误信息
    pub error: Option<String>,

    /// 使用的模型
    pub model: Option<String>,

    /// 元数据
    pub metadata: HashMap<String, Value>,
}

impl Session {
    /// 创建新会话
    pub fn new(working_directory: PathBuf) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: None,
            created_at: now,
            updated_at: now,
            working_directory,
            git_branch: None,
            messages: Vec::new(),
            token_usage: TokenUsage::default(),
            state: SessionState::Active,
            error: None,
            model: None,
            metadata: HashMap::new(),
        }
    }
}
```

### 1.2 会话状态

```rust
// crates/sage-core/src/session/types/base.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    /// 活跃中
    Active,
    /// 已暂停
    Paused,
    /// 已完成
    Completed,
    /// 已失败
    Failed,
}
```

### 1.3 Token 使用统计

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    /// 输入 token
    pub input_tokens: u64,
    /// 输出 token
    pub output_tokens: u64,
    /// 缓存读取 token
    pub cache_read_tokens: u64,
    /// 缓存创建 token
    pub cache_creation_tokens: u64,
}
```

---

## 二、SessionManager

### 2.1 管理器结构

```rust
// crates/sage-core/src/session/manager.rs
pub struct SessionManager {
    /// 存储后端
    storage: BoxedSessionStorage,

    /// 活跃会话缓存
    active_sessions: Arc<RwLock<HashMap<SessionId, Session>>>,

    /// 默认工作目录
    default_working_dir: PathBuf,
}
```

### 2.2 创建会话

```rust
impl SessionManager {
    /// 创建新会话
    pub async fn create(&self, config: SessionConfig) -> SageResult<SessionId> {
        let working_dir = config
            .working_directory
            .unwrap_or_else(|| self.default_working_dir.clone());

        let mut session = Session::new(working_dir);

        if let Some(name) = config.name {
            session.name = Some(name);
        }

        if let Some(model) = config.model {
            session.model = Some(model);
        }

        if let Some(system_prompt) = config.system_prompt {
            session.add_message(ConversationMessage::system(system_prompt));
        }

        for (key, value) in config.metadata {
            session.metadata.insert(key, value);
        }

        let id = session.id.clone();

        // 保存到存储
        self.storage.save(&session).await?;

        // 缓存活跃会话
        self.active_sessions.write().await.insert(id.clone(), session);

        info!("Created new session: {}", id);
        Ok(id)
    }
}
```

### 2.3 恢复会话

```rust
impl SessionManager {
    /// 恢复已有会话
    pub async fn resume(&self, id: &SessionId) -> SageResult<Session> {
        // 1. 检查缓存
        {
            let cache = self.active_sessions.read().await;
            if let Some(session) = cache.get(id) {
                let mut session = session.clone();
                session.resume();
                return Ok(session);
            }
        }

        // 2. 从存储加载
        let session = self.storage.load(id).await?
            .ok_or_else(|| SageError::invalid_input(format!("Session not found: {}", id)))?;

        // 3. 更新状态
        let mut session = session;
        if session.state == SessionState::Paused {
            session.resume();
        }

        // 4. 更新缓存
        self.active_sessions.write().await.insert(id.clone(), session.clone());

        info!("Resumed session {} from storage", id);
        Ok(session)
    }
}
```

### 2.4 其他操作

```rust
impl SessionManager {
    /// 保存会话
    pub async fn save(&self, session: &Session) -> SageResult<()>;

    /// 获取会话
    pub async fn get(&self, id: &SessionId) -> SageResult<Option<Session>>;

    /// 添加消息
    pub async fn add_message(&self, id: &SessionId, msg: ConversationMessage) -> SageResult<()>;

    /// 暂停会话
    pub async fn pause(&self, id: &SessionId) -> SageResult<()>;

    /// 完成会话
    pub async fn complete(&self, id: &SessionId) -> SageResult<()>;

    /// 标记失败
    pub async fn fail(&self, id: &SessionId, error: &str) -> SageResult<()>;

    /// 删除会话
    pub async fn delete(&self, id: &SessionId) -> SageResult<()>;

    /// 列出所有会话
    pub async fn list(&self) -> SageResult<Vec<SessionSummary>>;
}
```

---

## 三、存储后端

### 3.1 存储 Trait

```rust
// crates/sage-core/src/session/storage.rs
#[async_trait]
pub trait SessionStorage: Send + Sync {
    /// 保存会话
    async fn save(&self, session: &Session) -> SageResult<()>;

    /// 加载会话
    async fn load(&self, id: &SessionId) -> SageResult<Option<Session>>;

    /// 删除会话
    async fn delete(&self, id: &SessionId) -> SageResult<()>;

    /// 检查是否存在
    async fn exists(&self, id: &SessionId) -> SageResult<bool>;

    /// 列出所有会话
    async fn list(&self) -> SageResult<Vec<SessionSummary>>;
}

/// 类型别名
pub type BoxedSessionStorage = Box<dyn SessionStorage>;
```

### 3.2 内存存储

```rust
pub struct MemorySessionStorage {
    sessions: Arc<RwLock<HashMap<SessionId, Session>>>,
}

impl MemorySessionStorage {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl SessionStorage for MemorySessionStorage {
    async fn save(&self, session: &Session) -> SageResult<()> {
        self.sessions.write().await.insert(session.id.clone(), session.clone());
        Ok(())
    }

    async fn load(&self, id: &SessionId) -> SageResult<Option<Session>> {
        Ok(self.sessions.read().await.get(id).cloned())
    }
    // ...
}
```

### 3.3 文件存储

```rust
pub struct FileSessionStorage {
    base_path: PathBuf,
}

impl FileSessionStorage {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    fn session_path(&self, id: &SessionId) -> PathBuf {
        self.base_path.join(format!("{}.json", id))
    }
}

#[async_trait]
impl SessionStorage for FileSessionStorage {
    async fn save(&self, session: &Session) -> SageResult<()> {
        let path = self.session_path(&session.id);
        let json = serde_json::to_string_pretty(session)?;
        tokio::fs::write(&path, json).await?;
        Ok(())
    }

    async fn load(&self, id: &SessionId) -> SageResult<Option<Session>> {
        let path = self.session_path(id);
        if !path.exists() {
            return Ok(None);
        }
        let json = tokio::fs::read_to_string(&path).await?;
        let session: Session = serde_json::from_str(&json)?;
        Ok(Some(session))
    }
    // ...
}
```

### 3.4 JSONL 存储

```rust
// crates/sage-core/src/session/jsonl_storage/
pub struct JsonlSessionStorage {
    base_path: PathBuf,
    tracker: MessageChainTracker,
}

impl JsonlSessionStorage {
    /// 追加消息到 JSONL 文件
    pub async fn append_message(&self, id: &SessionId, msg: &ConversationMessage) -> SageResult<()> {
        let path = self.messages_path(id);
        let line = serde_json::to_string(msg)?;

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await?;

        file.write_all(format!("{}\n", line).as_bytes()).await?;
        Ok(())
    }
}
```

---

## 四、分支系统

### 4.1 设计理念

允许在对话关键点保存状态，探索不同方案后可恢复到之前状态：

```
        Root
          │
          ├── Branch A (探索方案1)
          │     │
          │     └── ... 继续对话
          │
          └── Branch B (探索方案2)
                │
                └── ... 继续对话
```

### 4.2 BranchManager

```rust
// crates/sage-core/src/session/branching/manager/
pub struct BranchManager {
    /// 分支树
    tree: RwLock<BranchTree>,
    /// 当前分支 ID
    current_branch: RwLock<Option<BranchId>>,
}

impl BranchManager {
    /// 创建新分支
    pub async fn create_branch(&self, name: Option<String>) -> SageResult<BranchId> {
        let mut tree = self.tree.write().await;
        let current = self.current_branch.read().await.clone();

        let branch_id = BranchId::new();
        let node = BranchNode::new(branch_id.clone(), name, current.clone());

        tree.add_node(node);
        Ok(branch_id)
    }

    /// 切换到分支
    pub async fn switch_to(&self, branch_id: &BranchId) -> SageResult<BranchSnapshot> {
        let tree = self.tree.read().await;
        let node = tree.get_node(branch_id)?;

        *self.current_branch.write().await = Some(branch_id.clone());
        Ok(node.snapshot.clone())
    }

    /// 获取当前分支快照
    pub async fn current_snapshot(&self) -> Option<BranchSnapshot> {
        let current = self.current_branch.read().await.clone()?;
        let tree = self.tree.read().await;
        tree.get_node(&current).ok().map(|n| n.snapshot.clone())
    }

    /// 列出所有分支
    pub async fn list_branches(&self) -> Vec<BranchId>;

    /// 合并分支
    pub async fn merge(&self, source: &BranchId, target: &BranchId) -> SageResult<()>;

    /// 删除分支
    pub async fn delete_branch(&self, branch_id: &BranchId) -> SageResult<()>;
}
```

### 4.3 分支快照

```rust
// crates/sage-core/src/session/branching/types.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchSnapshot {
    /// 快照消息
    pub messages: Vec<SerializedMessage>,
    /// 快照时间
    pub timestamp: DateTime<Utc>,
    /// 工具调用历史
    pub tool_calls: Vec<SerializedToolCall>,
    /// 元数据
    pub metadata: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchNode {
    /// 分支 ID
    pub id: BranchId,
    /// 分支名称
    pub name: Option<String>,
    /// 父分支 ID
    pub parent: Option<BranchId>,
    /// 子分支 IDs
    pub children: Vec<BranchId>,
    /// 分支快照
    pub snapshot: BranchSnapshot,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}
```

---

## 五、会话缓存

### 5.1 设计目标

提供类似 Claude Code `~/.claude.json` 的持久化状态：

```rust
// crates/sage-core/src/session/session_cache/types.rs
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionCacheData {
    /// 版本号
    pub version: u32,

    /// 工具信任设置
    pub tool_trust: ToolTrustSettings,

    /// MCP 服务器配置
    pub mcp_servers: McpServerCache,

    /// 最近会话
    pub recent_sessions: Vec<RecentSession>,

    /// 用户偏好
    pub preferences: UserPreferences,

    /// 当前会话 ID
    pub current_session_id: Option<String>,

    /// 自定义元数据
    pub metadata: HashMap<String, Value>,

    /// 最后保存时间
    pub last_saved: Option<DateTime<Utc>>,
}
```

### 5.2 工具信任设置

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolTrustSettings {
    /// 始终允许的工具
    pub always_allowed: HashSet<String>,
    /// 始终拒绝的工具
    pub always_denied: HashSet<String>,
    /// 需要确认的工具
    pub require_confirmation: HashSet<String>,
    /// 更新时间
    pub updated_at: Option<DateTime<Utc>>,
}
```

### 5.3 最近会话

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentSession {
    pub id: String,
    pub name: Option<String>,
    pub working_directory: String,
    pub model: Option<String>,
    pub last_active: DateTime<Utc>,
    pub message_count: usize,
    pub description: Option<String>,
}
```

### 5.4 SessionCache

```rust
// crates/sage-core/src/session/session_cache/manager.rs
pub struct SessionCache {
    config: SessionCacheConfig,
    data: RwLock<SessionCacheData>,
    global_cache_path: PathBuf,
    project_cache_path: Option<PathBuf>,
}

impl SessionCache {
    /// 加载缓存
    pub async fn load(&self) -> SageResult<()>;

    /// 保存缓存
    pub async fn save(&self) -> SageResult<()>;

    /// 添加最近会话
    pub async fn add_recent_session(&self, session: &Session) -> SageResult<()>;

    /// 更新工具信任
    pub async fn update_tool_trust(&self, tool: &str, trust: ToolTrust) -> SageResult<()>;

    /// 获取用户偏好
    pub async fn get_preferences(&self) -> UserPreferences;

    /// 更新用户偏好
    pub async fn update_preferences<F>(&self, f: F) -> SageResult<()>
    where
        F: FnOnce(&mut UserPreferences);
}
```

---

## 六、消息类型

### 6.1 ConversationMessage

```rust
// crates/sage-core/src/session/conversation.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    /// 角色
    pub role: MessageRole,
    /// 内容
    pub content: MessageContent,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 工具调用（如果有）
    pub tool_calls: Option<Vec<SessionToolCall>>,
    /// 工具结果（如果有）
    pub tool_result: Option<SessionToolResult>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

impl ConversationMessage {
    pub fn system(content: impl Into<String>) -> Self;
    pub fn user(content: impl Into<String>) -> Self;
    pub fn assistant(content: impl Into<String>) -> Self;
    pub fn tool(content: impl Into<String>, tool_call_id: impl Into<String>) -> Self;
}
```

### 6.2 增强消息（学习自 Claude Code）

```rust
// crates/sage-core/src/session/enhanced/message.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedMessage {
    /// 基础消息
    pub base: ConversationMessage,
    /// 消息类型
    pub message_type: EnhancedMessageType,
    /// 思考元数据
    pub thinking: Option<ThinkingMetadata>,
    /// Token 使用
    pub token_usage: Option<EnhancedTokenUsage>,
    /// 文件操作
    pub file_operations: Vec<FileBackupInfo>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum EnhancedMessageType {
    UserInput,
    AssistantResponse,
    ToolUse,
    ToolResult,
    SystemReminder,
    Thinking,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingMetadata {
    pub level: ThinkingLevel,
    pub content: String,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ThinkingLevel {
    None,
    Brief,
    Normal,
    Extended,
}
```

---

## 七、使用示例

### 7.1 基本使用

```rust
use sage_core::session::{SessionManager, SessionConfig, ConversationMessage};

// 创建管理器
let manager = SessionManager::in_memory();

// 创建会话
let config = SessionConfig::new()
    .with_name("My Session")
    .with_model("claude-3");

let session_id = manager.create(config).await?;

// 添加消息
manager.add_message(&session_id, ConversationMessage::user("Hello!")).await?;
manager.add_message(&session_id, ConversationMessage::assistant("Hi there!")).await?;

// 获取会话信息
let session = manager.get(&session_id).await?.unwrap();
println!("Session has {} messages", session.message_count());

// 暂停会话
manager.pause(&session_id).await?;

// 恢复会话
let resumed = manager.resume(&session_id).await?;

// 完成会话
manager.complete(&session_id).await?;
```

### 7.2 分支使用

```rust
use sage_core::session::{BranchManager, create_branch_manager};

let manager = create_branch_manager();

// 创建分支
let branch_a = manager.create_branch(Some("探索方案A".to_string())).await?;
let branch_b = manager.create_branch(Some("探索方案B".to_string())).await?;

// 切换分支
let snapshot = manager.switch_to(&branch_a).await?;

// 获取当前快照
if let Some(snapshot) = manager.current_snapshot().await {
    println!("Current branch has {} messages", snapshot.messages.len());
}

// 列出所有分支
let branches = manager.list_branches().await;
```

---

## 八、开发指南

### 8.1 添加新存储后端

1. 实现 `SessionStorage` trait：
```rust
pub struct RedisSessionStorage {
    client: redis::Client,
}

#[async_trait]
impl SessionStorage for RedisSessionStorage {
    async fn save(&self, session: &Session) -> SageResult<()> {
        // Redis 实现
    }
    // ...
}
```

2. 注册到 SessionManager

### 8.2 扩展会话状态

在 `SessionState` 枚举添加新状态：
```rust
pub enum SessionState {
    Active,
    Paused,
    Completed,
    Failed,
    Archived,  // 新增
}
```

### 8.3 存储位置

```
~/.sage/
├── sessions/           # 会话存储
│   ├── {id}.json      # 单个会话
│   └── {id}.jsonl     # JSONL 格式消息
├── cache.json         # 全局缓存
└── branches/          # 分支数据
```

```
./.sage/
├── cache.json         # 项目级缓存
└── sessions/          # 项目级会话
```

---

## 九、相关模块

- `sage-context-management` - 上下文管理（使用会话消息）
- `sage-agent-execution` - Agent 执行（会话集成）
- `sage-mcp-protocol` - MCP 协议（MCP 服务器缓存）

---

*最后更新: 2026-01-10*
