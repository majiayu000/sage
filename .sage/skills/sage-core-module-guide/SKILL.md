---
name: sage-core-module-guide
description: Sage-core 各模块职责详解和开发指南，包含模块间依赖关系和扩展方式
when_to_use: 当需要修改 sage-core 模块、理解模块职责、或添加新功能时使用
allowed_tools:
  - Read
  - Grep
  - Glob
  - Edit
  - Write
  - Bash
user_invocable: true
priority: 95
---

# Sage-core 模块指南

## 模块总览（36 个模块）

```
sage-core/src/
├── agent/          # 核心执行引擎
├── builder/        # 构建器模式
├── cache/          # 缓存系统
├── checkpoints/    # 状态快照 ⭐ 独有
├── commands/       # 斜杠命令
├── concurrency/    # 并发管理
├── config/         # 配置加载
├── context/        # 上下文管理
├── cost/           # 成本追踪
├── error/          # 错误定义
├── events/         # 事件总线
├── hooks/          # 钩子系统
├── input/          # 输入处理
├── interrupt/      # 中断管理
├── learning/       # 学习系统 ⭐ 独有
├── llm/            # LLM 抽象
├── mcp/            # MCP 协议
├── memory/         # 长期记忆
├── modes/          # 模式管理
├── output/         # 输出格式化
├── plugins/        # 插件系统
├── prompts/        # Prompt 模板
├── recovery/       # 恢复系统 ⭐ 独有
├── sandbox/        # 沙箱执行 ⭐ 独有
├── session/        # 会话管理
├── settings/       # 运行时设置
├── skills/         # 技能系统
├── storage/        # 持久化
├── telemetry/      # 遥测统计
├── tools/          # 工具系统
├── trajectory/     # 轨迹记录
├── types/          # 公共类型
├── ui/             # 终端 UI
├── utils/          # 工具函数
├── validation/     # 验证系统
└── workspace/      # 工作区分析
```

## 核心模块详解

### agent/ - 执行引擎

**职责**：Agent 生命周期管理、执行循环、状态转换

```
agent/
├── mod.rs              # 公开接口
├── unified/            # 统一执行器（主要）
│   ├── executor.rs     # 执行循环
│   ├── llm_orchestrator.rs  # LLM 调度
│   └── step_processor.rs    # 步骤处理
├── lifecycle/          # 生命周期
│   ├── manager.rs      # 管理器
│   ├── hooks/          # 生命周期钩子
│   └── phase.rs        # 阶段定义
├── state.rs            # Agent 状态
├── step.rs             # 执行步骤
└── completion.rs       # 完成检测
```

**关键类型**：
```rust
pub struct UnifiedExecutor { ... }
pub struct AgentExecution { ... }
pub enum AgentState { Idle, Running, Waiting, Completed, Failed }
pub struct AgentStep { ... }
```

**扩展点**：
- 实现 `LifecycleHook` 添加生命周期钩子
- 扩展 `ExecutionMode` 添加执行模式

---

### llm/ - LLM 抽象层

**职责**：多提供者支持、流式处理、降级策略

```
llm/
├── mod.rs
├── client.rs           # 客户端 trait
├── providers/          # 提供者实现
│   ├── anthropic.rs
│   ├── openai.rs
│   ├── google.rs
│   ├── glm.rs
│   └── ...
├── fallback/           # 降级策略
├── parsers/            # 响应解析
├── converters/         # 格式转换
└── rate_limiter/       # 限流
```

**关键类型**：
```rust
#[async_trait]
pub trait LlmClient {
    async fn chat(&self, request: ChatRequest) -> Result<LlmResponse>;
    async fn chat_stream(&self, request: ChatRequest) -> Result<impl Stream<Item = StreamEvent>>;
}

pub enum LlmProvider { Anthropic, OpenAI, Google, Glm, ... }
```

**扩展点**：
- 实现 `LlmClient` 添加新提供者
- 在 `fallback/` 中配置降级链

---

### tools/ - 工具系统

**职责**：工具注册、执行、权限管理、结果缓存

```
tools/
├── mod.rs
├── base/               # 基础定义
│   ├── tool.rs         # Tool trait
│   ├── schema.rs       # JSON Schema
│   └── result.rs       # 执行结果
├── executor/           # 执行器
├── permission/         # 权限系统
│   ├── rules.rs
│   └── handlers.rs
├── parallel_executor/  # 并行执行
├── tool_cache/         # 结果缓存
└── registry.rs         # 工具注册表
```

**关键类型**：
```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn schema(&self) -> ToolSchema;
    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError>;
}

pub struct ToolExecutor { ... }
pub struct ToolRegistry { ... }
```

**扩展点**：
- 实现 `Tool` trait 添加新工具
- 在 `permission/rules.rs` 添加权限规则

---

### recovery/ - 恢复系统 ⭐ Sage 独有

**职责**：熔断器、限流器、重试策略、任务监督

```
recovery/
├── mod.rs
├── circuit_breaker/    # 熔断器
│   ├── breaker.rs
│   ├── config.rs
│   └── registry.rs
├── rate_limiter/       # 限流器
│   ├── sliding_window.rs
│   └── token_bucket.rs
└── supervisor/         # 任务监督
    ├── policy.rs
    └── supervisor.rs
```

**关键类型**：
```rust
pub struct CircuitBreaker { ... }
pub enum CircuitState { Closed, Open, HalfOpen }

pub struct SlidingWindowRateLimiter { ... }
pub struct TokenBucketRateLimiter { ... }

pub struct TaskSupervisor { ... }
pub enum SupervisionPolicy { Restart, Resume, Stop }
```

**使用示例**：
```rust
// 熔断器
let breaker = CircuitBreaker::new(CircuitBreakerConfig {
    failure_threshold: 5,
    recovery_timeout: Duration::from_secs(30),
    ..Default::default()
});

breaker.call(|| async {
    llm_client.chat(request).await
}).await?;

// 限流器
let limiter = SlidingWindowRateLimiter::new(100, Duration::from_secs(60));
limiter.acquire().await?;
```

---

### learning/ - 学习系统 ⭐ Sage 独有

**职责**：模式识别、用户偏好学习、纠正记录

```
learning/
├── mod.rs
├── engine/             # 学习引擎
│   ├── engine.rs
│   └── config.rs
├── patterns/           # 模式检测
│   ├── detector.rs
│   └── types.rs
└── types/              # 类型定义
    ├── correction.rs
    ├── pattern.rs
    └── preference.rs
```

**关键类型**：
```rust
pub struct LearningEngine { ... }

pub struct Pattern {
    pub id: PatternId,
    pub pattern_type: PatternType,
    pub confidence: f32,
    pub occurrences: usize,
}

pub struct CorrectionRecord {
    pub original: String,
    pub corrected: String,
    pub reason: Option<String>,
}
```

---

### checkpoints/ - 检查点系统 ⭐ Sage 独有

**职责**：状态快照、文件变更追踪、回滚恢复

```
checkpoints/
├── mod.rs
├── manager.rs          # 检查点管理器
├── storage.rs          # 存储接口
│   ├── file.rs         # 文件存储
│   └── memory.rs       # 内存存储
├── snapshot.rs         # 快照类型
├── diff.rs             # 差异计算
└── restore.rs          # 恢复逻辑
```

**关键类型**：
```rust
pub struct CheckpointManager { ... }
pub struct Checkpoint {
    pub id: CheckpointId,
    pub created_at: DateTime<Utc>,
    pub file_states: Vec<FileSnapshot>,
    pub conversation: ConversationSnapshot,
}

pub struct RestoreOptions {
    pub restore_files: bool,
    pub restore_conversation: bool,
}
```

---

### skills/ - 技能系统

**职责**：Skill 发现、加载、匹配、激活

```
skills/
├── mod.rs
├── registry.rs         # 技能注册表
├── types.rs            # 类型定义
├── loader.rs           # SKILL.md 加载器
└── watcher.rs          # 热重载监视器
```

**关键类型**：
```rust
pub struct SkillRegistry { ... }
pub struct Skill {
    pub metadata: SkillMetadata,
    pub prompt: String,
    pub triggers: Vec<SkillTrigger>,
    pub when_to_use: Option<String>,
    pub available_tools: ToolAccess,
}

pub struct SkillContext { ... }
pub struct SkillActivation { ... }
```

---

## 模块依赖关系

```
                    ┌─────────────────┐
                    │     agent       │
                    └────────┬────────┘
                             │
         ┌───────────────────┼───────────────────┐
         │                   │                   │
         ▼                   ▼                   ▼
    ┌─────────┐        ┌─────────┐        ┌─────────┐
    │   llm   │        │  tools  │        │ skills  │
    └────┬────┘        └────┬────┘        └────┬────┘
         │                  │                   │
         │                  ▼                   │
         │           ┌──────────┐               │
         │           │ sandbox  │               │
         │           └──────────┘               │
         │                                      │
         ▼                                      ▼
    ┌─────────┐                          ┌─────────┐
    │recovery │                          │ prompts │
    └─────────┘                          └─────────┘
         │
         ▼
    ┌─────────┐     ┌─────────┐     ┌─────────┐
    │ context │ ←── │ session │ ←── │ memory  │
    └─────────┘     └─────────┘     └─────────┘
         │
         ▼
    ┌─────────┐     ┌─────────┐     ┌─────────┐
    │ storage │ ←── │  config │ ←── │settings │
    └─────────┘     └─────────┘     └─────────┘
```

## 扩展指南

### 添加新 LLM 提供者

1. 在 `llm/providers/` 创建 `new_provider.rs`
2. 实现 `LlmClient` trait
3. 在 `LlmProvider` enum 添加变体
4. 在配置中添加支持

### 添加新工具

参见 `/sage-tool-development` skill

### 添加新模式

1. 在 `modes/` 创建模式定义
2. 实现 `AgentMode` trait
3. 在 `ModeManager` 注册

### 添加新钩子类型

1. 在 `hooks/types.rs` 定义事件类型
2. 在 `HookRegistry` 添加注册方法
3. 在相应位置触发钩子

## 代码规范速查

```rust
// 模块命名
pub mod rate_limiter;  // 正确
pub mod rateLimiter;   // 错误

// 类型命名 (RFC 430)
struct LlmClient;      // 正确
struct LLMClient;      // 错误

// 错误处理
fn do_something() -> Result<T, Error>;  // 正确
fn do_something() -> T { x.unwrap() }   // 错误

// 异步代码
async fn fetch() -> Result<Data>;  // 正确
fn fetch() -> Data { block_on() }  // 错误
```
