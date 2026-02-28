# Sage Trajectory JSON 设计分析报告

> 基于 Open-Claude-Code v2.0.76 和 Sage 当前实现的对比分析

## 1. 执行摘要

### 1.1 当前问题

Sage 的 trajectory/session 系统存在以下核心问题：

| 问题 | 严重程度 | 影响 |
|------|----------|------|
| **存储系统碎片化** | 🔴 高 | 3 套并行存储系统，数据不一致 |
| **Trajectory 功能禁用** | 🔴 高 | 所有 trajectory 命令返回 "temporarily disabled" |
| **数据模型不统一** | 🟡 中 | `ConversationMessage` vs `EnhancedMessage` 冲突 |
| **缺少实时持久化** | 🟡 中 | 执行过程中消息未实时保存 |
| **分支功能未实现** | 🟢 低 | sidechain 代码存在但未集成 |

### 1.2 建议方案

采用 **Claude Code 风格的 JSONL 存储**，统一数据模型，实现实时持久化。

---

## 2. Open-Claude-Code 设计分析

### 2.1 存储架构

```
~/.claude-code/
├── sessions.db              # SQLite 主数据库 (索引+元数据)
├── sessions/
│   └── {session-id}/
│       ├── messages.jsonl   # 消息链 (一行一条消息)
│       ├── snapshots.jsonl  # 文件历史快照
│       └── metadata.json    # 会话元数据
└── session-index.json       # 快速索引
```

### 2.2 核心数据结构

#### Session Metadata
```typescript
interface SessionMetadata {
  id: string;                    // UUID
  title: string;                 // 自动生成或用户自定义
  created_at: string;            // ISO 8601
  updated_at: string;
  model: string;                 // "claude-opus-4.5"
  status: "active" | "completed" | "aborted";
  working_directory: string;
  git_branch?: string;

  // 配置快照
  allowed_tools: string[];
  max_tokens: number;
  temperature: number;
  system_prompt_hash: string;

  // 分支信息
  parent_session_id?: string;    // 分叉来源
  is_sidechain: boolean;
  sidechain_parent_uuid?: string;
}
```

#### Message (JSONL 格式)
```typescript
interface TranscriptMessage {
  type: "user" | "assistant" | "tool_result" | "system" |
        "error" | "summary" | "custom_title" | "file_history_snapshot";
  uuid: string;                  // 消息唯一 ID
  parentUuid: string | null;     // 父消息 ID (用于分支)
  timestamp: string;             // ISO 8601
  sessionId: string;
  version: string;               // CLI 版本

  // 上下文
  context: {
    cwd: string;
    gitBranch?: string;
    platform: string;
    userType: string;
  };

  // 消息内容
  message: {
    role: string;
    content: string;
    toolCalls?: ToolCall[];
    toolResults?: ToolResult[];
  };

  // Token 统计
  usage?: {
    inputTokens: number;
    outputTokens: number;
    cacheReadTokens: number;
    cacheWriteTokens: number;
  };

  // 扩展思考
  thinkingMetadata?: {
    level: "none" | "low" | "medium" | "high";
    disabled: boolean;
    triggers: string[];
  };

  // 任务列表快照
  todos: TodoItem[];

  // 分支标记
  isSidechain: boolean;
}
```

#### Tool Call / Result
```typescript
interface ToolCall {
  id: string;
  name: string;
  arguments: Record<string, any>;
}

interface ToolResult {
  toolCallId: string;
  toolName: string;
  content: string;
  success: boolean;
  error?: string;
}
```

### 2.3 关键特性

1. **实时持久化**: 每条消息立即写入 JSONL
2. **消息链追踪**: `uuid` + `parentUuid` 支持分支
3. **Sidechain 分支**: 从任意消息点创建分支会话
4. **文件历史快照**: 记录文件修改前状态，支持回滚
5. **Queue 操作记录**: 追踪异步操作队列

---

## 3. Sage 当前实现分析

### 3.1 存储系统碎片化

Sage 目前有 **3 套并行存储系统**：

```
存储系统 1: FileSessionStorage
位置: ~/.config/sage/sessions/{id}.json
格式: 完整 Session JSON
文件: crates/sage-core/src/session/storage.rs

存储系统 2: JsonlSessionStorage
位置: ~/.sage/sessions/{id}/messages.jsonl
格式: JSONL (一行一条消息)
文件: crates/sage-core/src/session/jsonl_storage/

存储系统 3: MemorySessionStorage
位置: 内存
用途: 测试
文件: crates/sage-core/src/session/storage.rs
```

**问题**: 这三套系统没有统一的调用入口，导致数据可能不一致。

### 3.2 数据模型冲突

存在两套消息类型：

```rust
// 系统 1: ConversationMessage (conversation.rs)
pub struct ConversationMessage {
    pub role: MessageRole,
    pub content: String,
    pub tool_calls: Option<Vec<SessionToolCall>>,
    pub tool_results: Option<Vec<SessionToolResult>>,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, Value>,
}

// 系统 2: EnhancedMessage (enhanced/message.rs)
pub struct EnhancedMessage {
    pub message_type: EnhancedMessageType,
    pub uuid: String,
    pub parent_uuid: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    pub version: String,
    pub context: SessionContext,
    pub message: MessageContent,
    pub usage: Option<EnhancedTokenUsage>,
    pub thinking_metadata: Option<ThinkingMetadata>,
    pub todos: Vec<TodoItem>,
    pub is_sidechain: bool,
    pub metadata: HashMap<String, Value>,
}
```

**问题**: `EnhancedMessage` 是 Claude Code 风格的设计，但 `Session` 使用的是 `ConversationMessage`。

### 3.3 Trajectory 功能禁用

```rust
// crates/sage-cli/src/commands/trajectory.rs
pub async fn list(_directory: &Path) -> SageResult<()> {
    console.info("Trajectory listing is temporarily disabled during refactoring.");
    Ok(())
}

pub async fn show(_trajectory_file: &Path) -> SageResult<()> {
    console.info("Trajectory details view is temporarily disabled during refactoring.");
    Ok(())
}
```

**问题**: 所有 trajectory 命令都被禁用，用户无法查看执行历史。

### 3.4 缺少实时持久化

当前 `Session` 只在显式调用 `save()` 时才持久化：

```rust
// FileSessionStorage::save()
async fn save(&self, session: &Session) -> SageResult<()> {
    let json = serde_json::to_string_pretty(session)?;
    fs::write(&path, json).await?;
}
```

**问题**: 如果执行中断，未保存的消息会丢失。

---

## 4. 问题根因分析

### 4.1 架构演进问题

```
初始设计 (v0.1)
└── FileSessionStorage (简单 JSON)

添加 Claude Code 特性 (v0.2)
├── FileSessionStorage (保留)
├── JsonlSessionStorage (新增)
└── EnhancedMessage (新增)

当前状态 (v0.3)
├── FileSessionStorage (未删除)
├── JsonlSessionStorage (部分实现)
├── EnhancedMessage (未集成)
└── Trajectory 命令 (禁用)
```

### 4.2 具体 Bug 列表

| Bug ID | 描述 | 位置 | 严重程度 |
|--------|------|------|----------|
| BUG-001 | Session 使用 ConversationMessage 而非 EnhancedMessage | session/types/session.rs:36 | 高 |
| BUG-002 | JsonlSessionStorage 未被 CLI 使用 | cli/src/main.rs | 高 |
| BUG-003 | Trajectory 命令全部禁用 | cli/src/commands/trajectory.rs | 高 |
| BUG-004 | 缺少消息实时持久化 | 无实现 | 中 |
| BUG-005 | Sidechain 功能未集成到 CLI | cli/src/router.rs | 低 |
| BUG-006 | 文件历史快照未实现 | 无实现 | 低 |

---

## 5. 推荐解决方案

### 5.1 统一存储架构

```
~/.sage/
├── projects/
│   └── {escaped-cwd}/           # 按项目目录分组
│       ├── sessions/
│       │   └── {session-id}/
│       │       ├── messages.jsonl
│       │       ├── snapshots.jsonl
│       │       └── metadata.json
│       └── index.json           # 项目会话索引
└── config.toml
```

### 5.2 统一数据模型

**废弃** `ConversationMessage`，统一使用 `EnhancedMessage`：

```rust
// 修改 Session 结构
pub struct Session {
    pub id: SessionId,
    pub metadata: SessionMetadata,
    // 移除: pub messages: Vec<ConversationMessage>,
    // 消息通过 JSONL 存储，不在内存中保留完整历史
}

// SessionMetadata 包含元信息
pub struct SessionMetadata {
    pub id: String,
    pub title: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub working_directory: PathBuf,
    pub git_branch: Option<String>,
    pub model: Option<String>,
    pub state: SessionState,
    pub token_usage: TokenUsage,
    pub config: SessionConfig,

    // 分支信息
    pub parent_session_id: Option<String>,
    pub is_sidechain: bool,
}
```

### 5.3 实时持久化机制

```rust
/// 消息持久化器
pub struct MessagePersister {
    storage: JsonlSessionStorage,
    session_id: String,
    last_uuid: Option<String>,
}

impl MessagePersister {
    /// 持久化消息 (立即写入 JSONL)
    pub async fn persist(&mut self, message: EnhancedMessage) -> SageResult<String> {
        let uuid = message.uuid.clone();
        self.storage.append_message(&self.session_id, &message).await?;
        self.last_uuid = Some(uuid.clone());
        Ok(uuid)
    }

    /// 持久化工具结果
    pub async fn persist_tool_result(&mut self, result: EnhancedMessage) -> SageResult<()> {
        self.storage.append_message(&self.session_id, &result).await
    }
}
```

### 5.4 Trajectory 命令恢复

```rust
// 重新实现 trajectory 命令
pub async fn list(directory: &Path) -> SageResult<()> {
    let storage = JsonlSessionStorage::for_directory(directory)?;
    let sessions = storage.list_sessions().await?;

    for session in sessions {
        println!("{} | {} | {} messages | {}",
            session.id,
            session.title.unwrap_or_default(),
            session.message_count,
            session.updated_at.format("%Y-%m-%d %H:%M")
        );
    }
    Ok(())
}

pub async fn show(session_id: &str) -> SageResult<()> {
    let storage = JsonlSessionStorage::default_path()?;
    let messages = storage.load_messages(session_id).await?;

    for msg in messages {
        match msg.message_type {
            EnhancedMessageType::User => {
                println!("👤 User: {}", truncate(&msg.message.content, 100));
            }
            EnhancedMessageType::Assistant => {
                println!("🤖 Assistant: {}", truncate(&msg.message.content, 100));
                if let Some(calls) = &msg.message.tool_calls {
                    for call in calls {
                        println!("   🔧 {}", call.name);
                    }
                }
            }
            EnhancedMessageType::ToolResult => {
                if let Some(results) = &msg.message.tool_results {
                    for result in results {
                        let status = if result.success { "✓" } else { "✗" };
                        println!("   {} {}", status, result.tool_name);
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}
```

---

## 6. 实施计划

### Phase 1: 统一存储 (P0)

1. 删除 `FileSessionStorage` 的使用
2. 将 `JsonlSessionStorage` 设为默认
3. 修改 CLI 使用新存储

### Phase 2: 统一数据模型 (P0)

1. 修改 `Session` 使用 `SessionMetadata`
2. 废弃 `ConversationMessage`
3. 统一使用 `EnhancedMessage`

### Phase 3: 实时持久化 (P1)

1. 实现 `MessagePersister`
2. 在 Agent 执行循环中集成
3. 添加崩溃恢复机制

### Phase 4: 恢复 Trajectory 命令 (P1)

1. 实现 `trajectory list`
2. 实现 `trajectory show`
3. 实现 `trajectory stats`
4. 实现 `trajectory analyze`

### Phase 5: 高级特性 (P2)

1. Sidechain 分支支持
2. 文件历史快照
3. 会话搜索和过滤

---

## 7. 数据迁移

### 7.1 迁移脚本

```rust
/// 将旧格式 Session JSON 迁移到新格式 JSONL
pub async fn migrate_session(old_path: &Path, new_storage: &JsonlSessionStorage) -> SageResult<()> {
    // 1. 读取旧格式
    let old_session: OldSession = serde_json::from_str(&fs::read_to_string(old_path).await?)?;

    // 2. 创建新会话
    let metadata = new_storage.create_session(&old_session.id, old_session.working_directory).await?;

    // 3. 转换消息
    for msg in old_session.messages {
        let enhanced = convert_to_enhanced(&msg, &old_session.id);
        new_storage.append_message(&old_session.id, &enhanced).await?;
    }

    // 4. 更新元数据
    new_storage.update_metadata(&old_session.id, |m| {
        m.token_usage = old_session.token_usage;
        m.state = old_session.state;
    }).await?;

    Ok(())
}
```

---

## 8. 测试计划

### 8.1 单元测试

- [ ] `JsonlSessionStorage::create_session`
- [ ] `JsonlSessionStorage::append_message`
- [ ] `JsonlSessionStorage::load_messages`
- [ ] `MessagePersister::persist`
- [ ] `EnhancedMessage` 序列化/反序列化

### 8.2 集成测试

- [ ] 完整会话流程 (创建 → 执行 → 保存 → 恢复)
- [ ] 崩溃恢复测试
- [ ] 大量消息性能测试
- [ ] 并发写入测试

### 8.3 E2E 测试

- [ ] `sage trajectory list`
- [ ] `sage trajectory show <id>`
- [ ] `sage -c` (继续会话)
- [ ] `sage -r <id>` (恢复指定会话)

---

## 9. 附录

### A. 文件变更清单

| 文件 | 操作 | 说明 |
|------|------|------|
| `session/types/session.rs` | 修改 | 移除 messages 字段 |
| `session/conversation.rs` | 废弃 | 使用 EnhancedMessage 替代 |
| `session/storage.rs` | 修改 | 移除 FileSessionStorage |
| `session/jsonl_storage/` | 增强 | 添加实时持久化 |
| `cli/src/commands/trajectory.rs` | 重写 | 恢复功能 |
| `cli/src/router.rs` | 修改 | 集成新存储 |

### B. 参考资料

- Claude Code v2.0.76 源码分析: `open-claude-code/docs/comparison/16-session-management.md`
- Sage 现有设计: `crates/sage-core/src/session/`
- JSONL 规范: https://jsonlines.org/
