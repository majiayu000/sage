---
name: sage-checkpoint-system
description: Sage 独有的检查点系统设计，包含状态快照、文件追踪、回滚恢复机制
when_to_use: 当需要实现状态恢复、设计撤销功能、或处理事务性操作时使用
allowed_tools:
  - Read
  - Grep
  - Glob
  - Edit
  - Write
user_invocable: true
priority: 75
---

# Sage 检查点系统指南

## 概述

Sage 的 `checkpoints/` 模块是**独有的竞争优势**，提供：

- **状态快照**: 保存完整执行状态
- **文件追踪**: 追踪所有文件变更
- **差异计算**: 高效计算和存储差异
- **回滚恢复**: 安全回滚到任意检查点

## 模块结构

```
checkpoints/
├── mod.rs              # 公开接口
├── manager.rs          # 检查点管理器
├── storage/            # 存储实现
│   ├── mod.rs
│   ├── file.rs         # 文件存储
│   └── memory.rs       # 内存存储（测试用）
├── snapshot.rs         # 快照类型
├── diff.rs             # 差异计算
├── detector.rs         # 变更检测
└── restore.rs          # 恢复逻辑
```

## 检查点类型

```rust
/// 检查点类型
pub enum CheckpointType {
    /// 自动检查点（定期创建）
    Auto,

    /// 手动检查点（用户请求）
    Manual,

    /// 工具执行前检查点
    PreToolExecution,

    /// 会话检查点
    Session,
}

/// 完整检查点
pub struct Checkpoint {
    /// 检查点 ID
    pub id: CheckpointId,

    /// 检查点类型
    pub checkpoint_type: CheckpointType,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 描述
    pub description: Option<String>,

    /// 文件状态快照
    pub file_states: Vec<FileSnapshot>,

    /// 会话状态快照
    pub conversation: ConversationSnapshot,

    /// Token 使用快照
    pub token_usage: TokenUsageSnapshot,

    /// 工具执行记录
    pub tool_executions: Vec<ToolExecutionRecord>,
}
```

## 检查点管理器

### 初始化

```rust
use sage_core::checkpoints::{CheckpointManager, CheckpointManagerConfig};

let config = CheckpointManagerConfig {
    // 存储路径
    storage_path: PathBuf::from(".sage/checkpoints"),

    // 最大检查点数
    max_checkpoints: 50,

    // 自动检查点间隔
    auto_checkpoint_interval: Duration::from_secs(300),  // 5 分钟

    // 保留策略
    retention_policy: RetentionPolicy::KeepLast(20),
};

let manager = CheckpointManager::new(config).await?;
```

### 创建检查点

```rust
// 手动创建
let checkpoint = manager.create_checkpoint(
    CheckpointType::Manual,
    Some("Before refactoring".to_string()),
).await?;

println!("Checkpoint created: {}", checkpoint.id);

// 自动创建（工具执行前）
let checkpoint = manager.create_pre_tool_checkpoint(&tool_call).await?;
```

### 列出检查点

```rust
let checkpoints = manager.list_checkpoints().await?;

for cp in checkpoints {
    println!("{} - {} - {:?}",
        cp.id,
        cp.created_at.format("%Y-%m-%d %H:%M:%S"),
        cp.checkpoint_type,
    );
}
```

### 恢复检查点

```rust
use sage_core::checkpoints::{RestoreOptions, RestorePreview};

// 预览恢复
let preview = manager.preview_restore(&checkpoint_id).await?;

println!("Files to restore: {}", preview.files_to_restore.len());
println!("Files to delete: {}", preview.files_to_delete.len());
for file in &preview.files_to_restore {
    println!("  {} ({:+} lines)", file.path, file.line_diff);
}

// 确认恢复
let options = RestoreOptions {
    restore_files: true,
    restore_conversation: false,  // 保留当前对话
    dry_run: false,
};

let result = manager.restore(&checkpoint_id, options).await?;
println!("Restored {} files", result.files_restored);
```

## 文件状态快照

### FileSnapshot 结构

```rust
pub struct FileSnapshot {
    /// 文件路径
    pub path: PathBuf,

    /// 文件状态
    pub state: FileState,

    /// 内容哈希
    pub content_hash: String,

    /// 文件大小
    pub size: u64,

    /// 修改时间
    pub modified_at: DateTime<Utc>,

    /// 权限（Unix）
    pub permissions: Option<u32>,
}

pub enum FileState {
    /// 文件存在，存储完整内容
    Exists { content: Vec<u8> },

    /// 文件存在，存储差异
    ExistsDiff { base_hash: String, diff: TextDiff },

    /// 文件不存在
    NotExists,
}
```

### 变更检测

```rust
use sage_core::checkpoints::ChangeDetector;

let detector = ChangeDetector::new(&working_dir);

// 扫描变更
let changes = detector.detect_changes(&previous_checkpoint).await?;

for change in changes {
    match change {
        FileChange::Created(path) => println!("+ {}", path),
        FileChange::Modified(path) => println!("M {}", path),
        FileChange::Deleted(path) => println!("- {}", path),
    }
}
```

## 差异计算

### 文本差异

```rust
use sage_core::checkpoints::{TextDiff, DiffHunk};

// 计算差异
let diff = TextDiff::compute(&old_content, &new_content);

println!("Hunks: {}", diff.hunks.len());
for hunk in &diff.hunks {
    println!("@@ -{},{} +{},{} @@",
        hunk.old_start, hunk.old_lines,
        hunk.new_start, hunk.new_lines,
    );
    for line in &hunk.lines {
        match line {
            DiffLine::Context(s) => println!(" {}", s),
            DiffLine::Added(s) => println!("+{}", s),
            DiffLine::Removed(s) => println!("-{}", s),
        }
    }
}

// 应用差异
let restored = diff.apply(&old_content)?;
assert_eq!(restored, new_content);
```

### 增量存储

```rust
impl CheckpointStorage for FileCheckpointStorage {
    async fn store(&self, checkpoint: &Checkpoint) -> Result<()> {
        for file in &checkpoint.file_states {
            match &file.state {
                FileState::Exists { content } => {
                    // 检查是否可以存储为差异
                    if let Some(base) = self.find_base(&file.content_hash).await? {
                        let diff = TextDiff::compute(&base, content);
                        if diff.size() < content.len() / 2 {
                            // 差异更小，存储差异
                            self.store_diff(&file.path, &checkpoint.id, diff).await?;
                            continue;
                        }
                    }
                    // 存储完整内容
                    self.store_full(&file.path, &checkpoint.id, content).await?;
                }
                FileState::NotExists => {
                    self.store_deletion(&file.path, &checkpoint.id).await?;
                }
                _ => {}
            }
        }
        Ok(())
    }
}
```

## 会话快照

```rust
pub struct ConversationSnapshot {
    /// 消息历史
    pub messages: Vec<ConversationMessage>,

    /// 当前模式
    pub mode: AgentMode,

    /// 活跃的 skills
    pub active_skills: Vec<String>,

    /// 上下文变量
    pub context_variables: HashMap<String, String>,
}

pub struct TokenUsageSnapshot {
    /// 总 token 数
    pub total_tokens: u64,

    /// 输入 token
    pub input_tokens: u64,

    /// 输出 token
    pub output_tokens: u64,

    /// 成本（美元）
    pub cost_usd: f64,
}
```

## 恢复策略

### 选择性恢复

```rust
pub struct RestoreOptions {
    /// 恢复文件
    pub restore_files: bool,

    /// 恢复会话
    pub restore_conversation: bool,

    /// 恢复 token 计数
    pub restore_token_usage: bool,

    /// 只恢复特定文件
    pub file_filter: Option<Vec<PathBuf>>,

    /// 排除文件
    pub exclude_files: Vec<PathBuf>,

    /// 干运行（不实际修改）
    pub dry_run: bool,

    /// 创建恢复前备份
    pub backup_before_restore: bool,
}

// 使用示例
let options = RestoreOptions {
    restore_files: true,
    restore_conversation: false,
    file_filter: Some(vec![
        PathBuf::from("src/main.rs"),
        PathBuf::from("src/lib.rs"),
    ]),
    dry_run: true,  // 先预览
    ..Default::default()
};

let preview = manager.restore(&checkpoint_id, options).await?;
```

### 冲突处理

```rust
pub enum ConflictResolution {
    /// 检查点版本优先
    PreferCheckpoint,

    /// 当前版本优先
    PreferCurrent,

    /// 创建冲突文件
    CreateConflictFile,

    /// 询问用户
    AskUser,
}

impl CheckpointManager {
    pub async fn restore_with_conflicts(
        &self,
        checkpoint_id: &CheckpointId,
        resolution: ConflictResolution,
    ) -> Result<RestoreResult> {
        let preview = self.preview_restore(checkpoint_id).await?;

        for conflict in preview.conflicts {
            match resolution {
                ConflictResolution::PreferCheckpoint => {
                    self.restore_file(&conflict.path, checkpoint_id).await?;
                }
                ConflictResolution::PreferCurrent => {
                    // 保持当前文件
                }
                ConflictResolution::CreateConflictFile => {
                    let conflict_path = format!("{}.checkpoint", conflict.path);
                    self.restore_file_to(&conflict.path, checkpoint_id, &conflict_path).await?;
                }
                ConflictResolution::AskUser => {
                    // 触发用户交互
                    self.emit_conflict_event(&conflict).await?;
                }
            }
        }

        Ok(RestoreResult { /* ... */ })
    }
}
```

## 与其他模块集成

### 与 Tools 集成

```rust
impl ToolExecutor {
    pub async fn execute_with_checkpoint(&self, call: &ToolCall) -> Result<ToolResult> {
        // 1. 创建执行前检查点
        let checkpoint = self.checkpoint_manager
            .create_pre_tool_checkpoint(call)
            .await?;

        // 2. 执行工具
        let result = self.execute_inner(call).await;

        // 3. 根据结果处理
        match &result {
            Ok(_) => {
                // 成功，记录工具执行
                self.checkpoint_manager
                    .record_tool_execution(&checkpoint.id, call, &result)
                    .await?;
            }
            Err(e) if e.is_recoverable() => {
                // 可恢复错误，提供回滚选项
                log::warn!("Tool execution failed, checkpoint available: {}", checkpoint.id);
            }
            Err(_) => {
                // 不可恢复错误，自动回滚
                self.checkpoint_manager
                    .restore(&checkpoint.id, RestoreOptions::files_only())
                    .await?;
            }
        }

        result
    }
}
```

### 与 Agent 集成

```rust
impl AgentExecutor {
    pub async fn run_with_checkpoints(&mut self, task: &Task) -> Result<()> {
        // 会话开始检查点
        let session_checkpoint = self.checkpoint_manager
            .create_checkpoint(CheckpointType::Session, None)
            .await?;

        // 定期自动检查点
        let auto_checkpoint_task = tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(300)).await;
                self.checkpoint_manager
                    .create_checkpoint(CheckpointType::Auto, None)
                    .await?;
            }
        });

        // 执行任务
        let result = self.execute(task).await;

        auto_checkpoint_task.abort();

        result
    }
}
```

### 与 Commands 集成

```rust
// /checkpoint 命令
pub struct CheckpointCommand;

impl SlashCommand for CheckpointCommand {
    fn name(&self) -> &str { "checkpoint" }

    async fn execute(&self, args: &str, ctx: &CommandContext) -> Result<String> {
        match args.split_whitespace().next() {
            Some("create") => {
                let desc = args.strip_prefix("create").map(|s| s.trim().to_string());
                let cp = ctx.checkpoint_manager
                    .create_checkpoint(CheckpointType::Manual, desc)
                    .await?;
                Ok(format!("Checkpoint created: {}", cp.id))
            }
            Some("list") => {
                let checkpoints = ctx.checkpoint_manager.list_checkpoints().await?;
                let output = checkpoints.iter()
                    .map(|cp| format!("{} - {}", cp.id.short(), cp.created_at))
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(output)
            }
            Some("restore") => {
                let id = args.strip_prefix("restore")
                    .and_then(|s| s.trim().parse().ok())
                    .ok_or(Error::InvalidArgs)?;
                let result = ctx.checkpoint_manager
                    .restore(&id, RestoreOptions::default())
                    .await?;
                Ok(format!("Restored {} files", result.files_restored))
            }
            _ => Ok("Usage: /checkpoint [create|list|restore <id>]".to_string())
        }
    }
}
```

## 存储优化

### 压缩存储

```rust
impl CheckpointStorage {
    async fn store_compressed(&self, content: &[u8]) -> Result<Vec<u8>> {
        use flate2::write::GzEncoder;
        use flate2::Compression;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(content)?;
        Ok(encoder.finish()?)
    }
}
```

### 去重存储

```rust
impl CheckpointStorage {
    async fn store_deduplicated(&self, file: &FileSnapshot) -> Result<()> {
        // 检查内容是否已存在
        if self.content_exists(&file.content_hash).await? {
            // 只存储引用
            self.store_reference(&file.path, &file.content_hash).await?;
        } else {
            // 存储新内容
            self.store_content(&file.content_hash, &file.content).await?;
        }
        Ok(())
    }
}
```

## 配置建议

```rust
CheckpointManagerConfig {
    // 存储路径
    storage_path: PathBuf::from(".sage/checkpoints"),

    // 最大检查点数
    max_checkpoints: 50,

    // 自动检查点间隔（秒）
    auto_checkpoint_interval: 300,

    // 保留策略
    retention_policy: RetentionPolicy::KeepLast(20),

    // 最大单个检查点大小
    max_checkpoint_size: 100 * 1024 * 1024,  // 100MB

    // 启用压缩
    compression: true,

    // 启用去重
    deduplication: true,

    // 危险操作前自动创建检查点
    auto_checkpoint_before_dangerous: true,
}
```
