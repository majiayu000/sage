# UnifiedExecutor Refactoring Analysis

> 综合分析报告 - 基于10个并行子agent的深度分析

## Executive Summary

UnifiedExecutor 是一个典型的 **God Object 反模式**，包含 15+ 字段、21+ 方法，横跨 8 个不同的责任域。本分析基于10个并行子agent对以下维度的深入研究：

| 维度 | Agent ID | 主要发现 |
|------|----------|----------|
| 核心结构 | acd8b36 | 8个责任域，20+方法，提议5组件抽取 |
| 状态管理 | ad59518 | 16个状态字段，4个语义层 |
| 事件处理 | af7843b | 事件分散在7+位置，缺少统一EventManager |
| 工具执行 | a6cfe85 | 650行step_execution.rs，三阶段执行模型 |
| 会话管理 | adc4c45 | 5个分散的会话字段 |
| 上下文管理 | a8757b3 | CLAUDE.md未加载，缺少ContextBuilder |
| LLM交互 | a7cd961 | 缺少中间件抽象，retry/fallback分散 |
| 错误处理 | a9b1188 | 字符串匹配错误分类，10+ silent failures |
| 并发模式 | aa383f5 | hook token未绑定task scope，无取消保护 |
| 依赖耦合 | a87e954 | 测试性评分 3/10，零trait抽象 |

---

## 1. 核心问题诊断

### 1.1 God Object 反模式

**当前 UnifiedExecutor 字段 (15+)：**
```rust
pub struct UnifiedExecutor {
    id: Id,
    config: Config,
    llm_client: LlmClient,
    tool_executor: ToolExecutor,
    options: ExecutionOptions,
    input_channel: Option<InputChannel>,
    session_recorder: Option<Arc<Mutex<SessionRecorder>>>,
    animation_manager: AnimationManager,
    jsonl_storage: Option<Arc<JsonlSessionStorage>>,
    message_tracker: MessageChainTracker,
    current_session_id: Option<String>,
    file_tracker: FileSnapshotTracker,
    last_summary_msg_count: usize,
    auto_compact: AutoCompact,
    skill_registry: Arc<RwLock<SkillRegistry>>,
    hook_executor: HookExecutor,
}
```

**问题分析：**
- 单一结构体管理 8 个不同责任域
- 21+ 公共方法分散在 11 个文件中
- step_execution.rs 达到 650 行，违反 200 行规则
- 测试性评分仅 3/10

### 1.2 责任域分布

| 责任域 | 字段数 | 方法数 | 文件 |
|--------|--------|--------|------|
| 执行编排 | 1 | 3 | execution_loop.rs |
| LLM 交互 | 2 | 2 | step_execution.rs |
| 工具管理 | 2 | 4 | step_execution.rs |
| 用户交互 | 2 | 3 | user_interaction.rs |
| 会话持久化 | 4 | 8 | session.rs |
| 文件追踪 | 1 | 2 | session.rs |
| UI/动画 | 1 | 多次调用 | step_execution.rs |
| 系统支持 | 4 | 6 | 多文件 |

---

## 2. 提议的组件架构

基于分析结果，建议将 UnifiedExecutor 重构为以下 5 个核心组件：

### 2.1 ExecutionEngine (执行引擎)

**职责：** 主循环控制、步骤排序、完成检测

```rust
pub struct ExecutionEngine {
    // 编排其他组件
}

impl ExecutionEngine {
    pub async fn execute(&mut self, task: TaskMetadata) -> SageResult<ExecutionOutcome>;
    fn run_loop(&mut self) -> SageResult<()>;
    fn handle_step_completion(&mut self, step: AgentStep);
    fn detect_repetition(&self, outputs: &[String]) -> bool;
}
```

### 2.2 LlmOrchestrator (LLM 编排器)

**职责：** 所有 LLM 通信和提示词构建

```rust
pub struct LlmOrchestrator {
    client: Arc<dyn LlmService>,
    retry_strategy: Arc<dyn RetryStrategy>,
    fallback_manager: Arc<dyn FallbackManager>,
    recorder: Option<Arc<dyn ResponseRecorder>>,
}

#[async_trait]
pub trait LlmService: Send + Sync {
    async fn chat(&self, messages: &[LlmMessage], tools: Option<&[ToolSchema]>) -> SageResult<LlmResponse>;
    async fn chat_stream(&self, messages: &[LlmMessage], tools: Option<&[ToolSchema]>) -> SageResult<LlmStream>;
}
```

### 2.3 ToolOrchestrator (工具编排器)

**职责：** 工具执行生命周期管理

```rust
pub struct ToolOrchestrator {
    tool_executor: Arc<ToolExecutor>,
    hook_executor: Arc<HookExecutor>,
    permission_handler: Arc<dyn PermissionHandler>,
    file_tracker: Arc<FileSnapshotTracker>,
}

impl ToolOrchestrator {
    /// 三阶段执行模型
    pub async fn execute_tool_call(
        &self,
        tool_call: &ToolCall,
        task_scope: &TaskScope,
    ) -> SageResult<ToolExecutionResult> {
        self.pre_execution_phase(&context).await?;
        self.execution_phase(&context).await?;
        self.post_execution_phase(&context).await?;
        Ok(context.result())
    }
}
```

### 2.4 SessionManager (会话管理器)

**职责：** 消息持久化、会话状态、文件快照

```rust
pub struct SessionManager {
    storage: Arc<JsonlSessionStorage>,
    message_tracker: MessageChainTracker,
    session_id: String,
    metadata_cache: Arc<Mutex<SessionMetadata>>,
}

#[async_trait]
pub trait SessionRecorder: Send + Sync {
    async fn record_user_message(&mut self, content: &str) -> SageResult<EnhancedMessage>;
    async fn record_assistant_message(&mut self, content: &str, tool_calls: Option<Vec<EnhancedToolCall>>) -> SageResult<EnhancedMessage>;
    async fn create_file_snapshot(&mut self) -> SageResult<()>;
}
```

### 2.5 EventManager (事件管理器)

**职责：** 统一事件发射、记录和订阅

```rust
pub struct EventManager {
    middleware: Vec<Arc<dyn EventMiddleware>>,
    recorders: Vec<Arc<dyn EventRecorder>>,
    listeners: Vec<Arc<dyn EventListener>>,
}

pub enum ExecutionEvent {
    ToolExecution { event: ToolExecutionEvent, timestamp: Instant, step_number: u32 },
    MessageRecorded { event: MessageEvent, message_uuid: String },
    SessionEvent { event: SessionEvent, session_id: String },
}
```

---

## 3. 关键问题及修复建议

### 3.1 错误处理问题

**问题：** 字符串匹配错误分类

```rust
// 当前 (脆弱)
let error_type = if error_message.contains("API") {
    "api_error"
} else if error_message.contains("timeout") {
    "timeout_error"
}
```

**修复：** 使用结构化错误码

```rust
// 推荐
match error.error_code() {
    "SAGE_TIMEOUT" => ErrorCategory::Timeout,
    "SAGE_LLM_RATE_LIMIT" => ErrorCategory::RateLimit,
    _ => ErrorCategory::Execution,
}
```

### 3.2 并发问题

**问题：** Hook token 未绑定到 task scope

```rust
// 当前 (问题)
let cancel_token = CancellationToken::new();  // 新token，与task无关
```

**修复：** 使用 task scope 的 token

```rust
// 推荐
let cancel_token = task_scope.token().clone();  // 使用任务的token
```

### 3.3 Silent Failures

**问题：** 10+ 处使用 `let _ =` 丢弃错误

```rust
// 当前
let _ = self.record_file_snapshot(&msg.uuid).await;
```

**修复：** 记录非致命错误

```rust
// 推荐
if let Err(e) = self.record_file_snapshot(&msg.uuid).await {
    tracing::warn!(error_code = %e.error_code(), "File snapshot failed (non-fatal)");
}
```

### 3.4 测试性问题

**问题：** 零 trait 抽象，无法 mock

**修复：** 引入 5 个核心服务 traits

```rust
pub trait LlmService: Send + Sync { /* ... */ }
pub trait ToolService: Send + Sync { /* ... */ }
pub trait SessionRecorder: Send + Sync { /* ... */ }
pub trait UserInteractionService: Send + Sync { /* ... */ }
pub trait ProgressReporter: Send + Sync { /* ... */ }
```

---

## 4. 重构路线图

### Phase 1: 基础抽取 (优先级: 高)
1. 提取 `SessionManager` - 最高隔离度，零行为风险
2. 提取 `ToolOrchestrator` - 降低认知负担，提升测试性
3. 创建核心 trait 定义

### Phase 2: 交互层重构 (优先级: 中)
4. 提取 `LlmOrchestrator` - 支持 provider 切换
5. 提取 `EventManager` - 统一事件处理
6. 实现 `ContextBuilder` - 加载 CLAUDE.md

### Phase 3: 稳定性增强 (优先级: 中)
7. 修复 hook cancellation token
8. 添加工具执行取消支持
9. 实现结构化错误分类
10. 添加 lock 超时保护

### Phase 4: 测试与文档 (优先级: 低)
11. 为每个组件创建 mock 实现
12. 编写单元测试和集成测试
13. 更新架构文档

---

## 5. 预期收益

| 指标 | 当前 | 重构后 | 改进 |
|------|------|--------|------|
| 变更原因数 | 8 | 1/组件 | 87.5% 降低 |
| 平均方法数/文件 | 20-30 | 5-8 | 更易理解 |
| 测试性评分 | 3/10 | 9/10 | 200% 提升 |
| step_execution.rs 行数 | 650 | <300 | 54% 减少 |
| 代码复用性 | 低 | 高 | 组件可独立使用 |

---

## 6. 文件影响分析

| 文件 | 当前行数 | 预计变化 | 新职责 |
|------|----------|----------|--------|
| mod.rs | 232 | -80 | 仅组件组合 |
| executor.rs | 89 | 保持 | 高层 execute |
| step_execution.rs | 650 | -350 | 委托给 ToolOrchestrator |
| session.rs | 494 | 移除 | 迁移到 SessionManager |
| execution_loop.rs | 220 | -50 | 简化循环控制 |
| **新增: tool_orchestrator.rs** | - | +250 | 工具执行生命周期 |
| **新增: session_manager.rs** | - | +200 | 会话管理 |
| **新增: event_manager.rs** | - | +150 | 事件处理 |
| **新增: llm_orchestrator.rs** | - | +180 | LLM 编排 |

---

## 7. 结论

UnifiedExecutor 当前是一个承载过多职责的 God Object，严重违反了单一职责原则。通过提取 5 个专门组件（ExecutionEngine、LlmOrchestrator、ToolOrchestrator、SessionManager、EventManager），可以：

1. **提升可维护性** - 每个组件专注单一职责
2. **提升测试性** - 可独立 mock 各组件
3. **提升可扩展性** - 通过 trait 注入实现
4. **符合 CLAUDE.md 规范** - 文件保持在 200 行以内

**预估工作量：** 3-4 天（含测试）
**风险等级：** 低（可增量迁移）
