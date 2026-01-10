---
name: sage-agent-execution
description: Sage Agent 执行引擎开发指南，涵盖 UnifiedExecutor、Subagent、Lifecycle 管理
when_to_use: 当涉及 Agent 执行流程、子Agent编排、生命周期管理时使用
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

# Sage Agent 执行引擎开发指南

## 模块概览

Agent 模块是 Sage 的核心执行引擎，代码量 **10658 行**，包含：

```
crates/sage-core/src/agent/
├── mod.rs              # 公开接口
├── execution.rs        # 执行入口 (420行)
├── outcome.rs          # 执行结果 (423行)
├── state.rs            # Agent状态 (250行)
├── options.rs          # 配置选项 (415行)
├── completion.rs       # 完成检测 (393行)
├── traits.rs           # 核心trait (228行)
├── lifecycle/          # 生命周期管理
│   ├── manager.rs      # 生命周期管理器 (354行)
│   └── tests.rs
├── subagent/           # 子Agent系统
│   ├── runner.rs       # 子Agent运行器 (487行)
│   ├── builtin.rs      # 内置子Agent (272行)
│   ├── registry/       # 子Agent注册表
│   └── types/          # 类型定义
└── unified/            # 统一执行器
    ├── mod.rs          # 入口 (250行)
    ├── session.rs      # 会话管理 (536行)
    ├── execution_loop.rs  # 执行循环 (247行)
    ├── step_execution.rs  # 步骤执行 (285行)
    ├── context_builder.rs # 上下文构建 (245行)
    └── tool_orchestrator.rs # 工具编排 (222行)
```

---

## 一、核心架构：UnifiedExecutor

### 1.1 设计理念（学习自 Crush）

Crush 的 `SessionAgent` 设计：
```go
type Agent struct {
    largeModel    Model  // 主模型
    smallModel    Model  // 辅助模型
    messageQueue  []Message
    tools         []Tool
}
```

Sage 的 `UnifiedExecutor` 对应实现：

```rust
// crates/sage-core/src/agent/unified/mod.rs
pub struct UnifiedExecutor {
    /// LLM 客户端
    llm_client: Arc<dyn LlmClient>,

    /// 工具注册表
    tool_registry: ToolRegistry,

    /// 上下文管理器
    context_manager: ContextManager,

    /// 执行选项
    options: ExecutionOptions,

    /// 生命周期管理器
    lifecycle: LifecycleManager,

    /// 技能注册表
    skill_registry: SkillRegistry,
}
```

### 1.2 执行循环设计

```
┌─────────────────────────────────────────────────────────────┐
│                    UnifiedExecutor                           │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │                   执行循环 (execution_loop.rs)          │ │
│  │                                                         │ │
│  │   用户输入 ──▶ prepare_context() ──▶ 构建消息           │ │
│  │                     │                                   │ │
│  │                     ▼                                   │ │
│  │             execute_step() ◄───────────────┐            │ │
│  │                     │                      │            │ │
│  │                     ▼                      │            │ │
│  │           ┌─────────────────┐              │            │ │
│  │           │   LLM 调用      │              │            │ │
│  │           └────────┬────────┘              │            │ │
│  │                    │                       │            │ │
│  │           ┌────────▼────────┐              │            │ │
│  │           │  解析响应       │              │            │ │
│  │           │  - 文本输出     │              │            │ │
│  │           │  - 工具调用     │              │            │ │
│  │           └────────┬────────┘              │            │ │
│  │                    │                       │            │ │
│  │     ┌──────────────┼──────────────┐        │            │ │
│  │     ▼              ▼              ▼        │            │ │
│  │ [无工具]      [有工具]       [结束信号]    │            │ │
│  │     │              │              │        │            │ │
│  │     │     execute_tools()         │        │            │ │
│  │     │              │              │        │            │ │
│  │     │              ▼              │        │            │ │
│  │     │     工具结果追加到上下文    │        │            │ │
│  │     │              │              │        │            │ │
│  │     └──────────────┼──────────────┘        │            │ │
│  │                    │                       │            │ │
│  │                    └───────────────────────┘            │ │
│  │                         (继续循环)                       │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
│                          ▼                                   │
│                   AgentOutcome                               │
└─────────────────────────────────────────────────────────────┘
```

### 1.3 核心代码模式

#### 执行器初始化

```rust
impl UnifiedExecutor {
    pub fn new(
        config: &Config,
        options: ExecutionOptions,
    ) -> SageResult<Self> {
        // 1. 初始化 LLM 客户端
        let llm_client = create_llm_client(config)?;

        // 2. 初始化工具注册表
        let tool_registry = ToolRegistry::new();
        tool_registry.register_builtins()?;

        // 3. 初始化上下文管理器
        let context_manager = ContextManager::new(
            options.max_context_tokens,
            options.auto_compact_threshold,
        );

        // 4. 初始化技能注册表
        let skill_registry = SkillRegistry::new();
        skill_registry.discover(&options.working_directory)?;

        Ok(Self {
            llm_client,
            tool_registry,
            context_manager,
            options,
            lifecycle: LifecycleManager::new(),
            skill_registry,
        })
    }
}
```

#### 执行步骤

```rust
impl UnifiedExecutor {
    /// 执行单个步骤
    async fn execute_step(&mut self, context: &ExecutionContext) -> SageResult<StepResult> {
        // 1. 构建消息
        let messages = self.context_manager.build_messages(context)?;

        // 2. 注入技能 prompt（如果匹配）
        let skill_context = SkillContext::from_execution_context(context);
        if let Some(skill) = self.skill_registry.find_best_match(&skill_context) {
            let activation = skill.activate(&skill_context);
            messages.push(activation.as_system_message());
        }

        // 3. 调用 LLM
        let response = self.llm_client
            .complete(messages, &self.options.llm_options)
            .await?;

        // 4. 解析响应
        let parsed = parse_llm_response(&response)?;

        // 5. 执行工具（如果有）
        if !parsed.tool_calls.is_empty() {
            let tool_results = self.execute_tools(&parsed.tool_calls).await?;
            return Ok(StepResult::ToolCalls {
                text: parsed.text,
                calls: parsed.tool_calls,
                results: tool_results,
            });
        }

        Ok(StepResult::Complete { text: parsed.text })
    }
}
```

---

## 二、子Agent系统

### 2.1 设计理念（学习自 Claude Code）

Claude Code 的 Task Tool 支持启动子Agent处理复杂任务。Sage 实现类似机制：

```rust
// crates/sage-core/src/agent/subagent/types.rs
pub struct SubagentDefinition {
    /// 子Agent名称
    pub name: String,

    /// 子Agent类型
    pub agent_type: SubagentType,

    /// 描述
    pub description: String,

    /// 工具访问控制
    pub tool_access: ToolAccessControl,

    /// 工作目录配置
    pub working_directory: WorkingDirectoryConfig,
}

pub enum SubagentType {
    /// 探索型 - 代码库搜索
    Explore,
    /// 规划型 - 制定实施方案
    Plan,
    /// 执行型 - 执行具体任务
    Execute,
    /// 通用型
    GeneralPurpose,
}
```

### 2.2 子Agent继承机制

**关键点：子Agent必须继承父Agent的工作目录**

```rust
// crates/sage-core/src/agent/subagent/types/working_directory.rs
pub enum WorkingDirectoryConfig {
    /// 继承父进程（默认）
    Inherited,
    /// 显式指定
    Explicit(PathBuf),
    /// 使用进程当前目录
    ProcessCwd,
}

impl UnifiedExecutor {
    /// 创建子Agent执行器
    pub fn create_subagent_executor(
        &self,
        definition: &SubagentDefinition,
    ) -> SageResult<UnifiedExecutor> {
        // 1. 确定工作目录
        let working_dir = match &definition.working_directory {
            WorkingDirectoryConfig::Inherited => {
                self.options.working_directory.clone()  // ◄── 继承
            }
            WorkingDirectoryConfig::Explicit(path) => path.clone(),
            WorkingDirectoryConfig::ProcessCwd => std::env::current_dir()?,
        };

        // 2. 复制选项并修改
        let mut options = self.options.clone();
        options.working_directory = working_dir;

        // 3. 配置工具访问
        let tool_registry = match &definition.tool_access {
            ToolAccessControl::Inherited => self.tool_registry.clone(),
            ToolAccessControl::Restricted(names) => {
                self.tool_registry.filter_by_names(names)
            }
            ToolAccessControl::All => ToolRegistry::new_with_all(),
        };

        Ok(UnifiedExecutor {
            llm_client: self.llm_client.clone(),
            tool_registry,
            context_manager: ContextManager::new_for_subagent(),
            options,
            lifecycle: LifecycleManager::new(),
            skill_registry: self.skill_registry.clone(),
        })
    }
}
```

### 2.3 内置子Agent类型

```rust
// crates/sage-core/src/agent/subagent/builtin.rs
pub fn get_builtin_subagents() -> Vec<SubagentDefinition> {
    vec![
        SubagentDefinition {
            name: "Explore".to_string(),
            agent_type: SubagentType::Explore,
            description: "快速探索代码库，查找文件和代码".to_string(),
            tool_access: ToolAccessControl::Restricted(vec![
                "Read", "Glob", "Grep",
            ]),
            working_directory: WorkingDirectoryConfig::Inherited,
        },
        SubagentDefinition {
            name: "Plan".to_string(),
            agent_type: SubagentType::Plan,
            description: "制定实施方案，识别关键文件".to_string(),
            tool_access: ToolAccessControl::Restricted(vec![
                "Read", "Glob", "Grep", "WebSearch",
            ]),
            working_directory: WorkingDirectoryConfig::Inherited,
        },
        SubagentDefinition {
            name: "GeneralPurpose".to_string(),
            agent_type: SubagentType::GeneralPurpose,
            description: "通用任务处理".to_string(),
            tool_access: ToolAccessControl::Inherited,
            working_directory: WorkingDirectoryConfig::Inherited,
        },
    ]
}
```

---

## 三、生命周期管理

### 3.1 Agent 状态机

```rust
// crates/sage-core/src/agent/state.rs
pub enum AgentState {
    /// 初始化中
    Initializing,
    /// 空闲，等待输入
    Idle,
    /// 执行中
    Executing(ExecutionPhase),
    /// 等待用户确认
    WaitingForConfirmation,
    /// 暂停
    Paused,
    /// 完成
    Completed(CompletionReason),
    /// 错误
    Error(AgentError),
}

pub enum ExecutionPhase {
    /// 准备上下文
    PreparingContext,
    /// 调用 LLM
    CallingLlm,
    /// 执行工具
    ExecutingTools,
    /// 处理响应
    ProcessingResponse,
}
```

### 3.2 生命周期管理器

```rust
// crates/sage-core/src/agent/lifecycle/manager.rs
pub struct LifecycleManager {
    /// 当前状态
    state: AgentState,

    /// 状态历史
    history: Vec<StateTransition>,

    /// 中断处理器
    interrupt_handler: InterruptHandler,

    /// 事件发布器
    event_emitter: EventEmitter,
}

impl LifecycleManager {
    /// 状态转换
    pub fn transition(&mut self, new_state: AgentState) -> SageResult<()> {
        // 1. 验证转换合法性
        self.validate_transition(&self.state, &new_state)?;

        // 2. 记录转换
        self.history.push(StateTransition {
            from: self.state.clone(),
            to: new_state.clone(),
            timestamp: Instant::now(),
        });

        // 3. 发布事件
        self.event_emitter.emit(AgentEvent::StateChanged {
            from: self.state.clone(),
            to: new_state.clone(),
        });

        // 4. 更新状态
        self.state = new_state;

        Ok(())
    }

    /// 处理中断
    pub async fn handle_interrupt(&mut self) -> SageResult<InterruptAction> {
        match self.state {
            AgentState::Executing(_) => {
                self.transition(AgentState::Paused)?;
                Ok(InterruptAction::Pause)
            }
            AgentState::WaitingForConfirmation => {
                Ok(InterruptAction::Cancel)
            }
            _ => Ok(InterruptAction::Ignore),
        }
    }
}
```

---

## 四、执行结果处理

### 4.1 AgentOutcome 设计

```rust
// crates/sage-core/src/agent/outcome.rs
pub struct AgentOutcome {
    /// 最终状态
    pub status: OutcomeStatus,

    /// 输出消息
    pub messages: Vec<AgentMessage>,

    /// 执行统计
    pub stats: ExecutionStats,

    /// 工具调用历史
    pub tool_history: Vec<ToolCallRecord>,

    /// 消耗的 token
    pub token_usage: TokenUsage,
}

pub enum OutcomeStatus {
    /// 成功完成
    Success,
    /// 用户取消
    Cancelled,
    /// 达到最大轮次
    MaxTurnsReached,
    /// 错误终止
    Error(AgentError),
}

pub struct ExecutionStats {
    /// 总轮次
    pub total_turns: usize,
    /// 工具调用次数
    pub tool_calls: usize,
    /// 总耗时
    pub duration: Duration,
    /// LLM 调用次数
    pub llm_calls: usize,
}
```

---

## 五、开发指南

### 5.1 添加新的子Agent类型

1. 在 `subagent/types.rs` 添加类型定义：
```rust
pub enum SubagentType {
    // ... 现有类型
    CodeReview,  // 新增
}
```

2. 在 `subagent/builtin.rs` 注册：
```rust
SubagentDefinition {
    name: "CodeReview".to_string(),
    agent_type: SubagentType::CodeReview,
    description: "代码审查专家".to_string(),
    tool_access: ToolAccessControl::Restricted(vec![
        "Read", "Grep", "Glob",
    ]),
    working_directory: WorkingDirectoryConfig::Inherited,
}
```

### 5.2 扩展执行循环

1. 创建新的执行策略：
```rust
// agent/unified/strategies/
pub trait ExecutionStrategy {
    async fn execute(
        &self,
        executor: &UnifiedExecutor,
        context: &ExecutionContext,
    ) -> SageResult<AgentOutcome>;
}

pub struct StandardStrategy;
pub struct StreamingStrategy;
pub struct BatchStrategy;
```

2. 在 `UnifiedExecutor` 中使用：
```rust
impl UnifiedExecutor {
    pub async fn execute_with_strategy<S: ExecutionStrategy>(
        &mut self,
        strategy: &S,
        context: ExecutionContext,
    ) -> SageResult<AgentOutcome> {
        strategy.execute(self, &context).await
    }
}
```

### 5.3 最佳实践

**1. 保持执行器无状态**
```rust
// 好 ✓ - 状态通过参数传递
async fn execute(&self, context: ExecutionContext) -> SageResult<AgentOutcome>

// 避免 ✗ - 内部可变状态
async fn execute(&mut self) -> SageResult<AgentOutcome>
```

**2. 使用 Arc 共享资源**
```rust
pub struct UnifiedExecutor {
    llm_client: Arc<dyn LlmClient>,  // ✓ 共享
    tool_registry: ToolRegistry,      // ✓ 可 Clone
}
```

**3. 正确处理中断**
```rust
loop {
    // 检查中断
    if self.lifecycle.check_interrupt()? {
        return Ok(AgentOutcome::cancelled());
    }

    // 执行步骤
    let result = self.execute_step(&context).await?;

    // ...
}
```

---

## 六、与 Crush 的对比

| 方面 | Crush (Go) | Sage (Rust) |
|------|-----------|-------------|
| 执行器 | SessionAgent | UnifiedExecutor |
| 状态管理 | 内部字段 | LifecycleManager |
| 工具编排 | 直接调用 | ToolOrchestrator |
| 子Agent | 有限支持 | 完整的 Subagent 系统 |
| 上下文 | 固定窗口 | 自动压缩 (auto_compact) |
| 恢复 | 无 | 熔断器/限流器/Supervisor |

---

## 七、相关模块

- `sage-context-management` - 上下文窗口管理
- `sage-tool-development` - 工具开发
- `sage-recovery-patterns` - 恢复模式
- `sage-llm-integration` - LLM 客户端

---

*最后更新: 2026-01-10*
