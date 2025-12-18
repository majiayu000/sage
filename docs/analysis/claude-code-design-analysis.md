# Claude Code 设计理念与 Sage Agent 改进建议

> 基于 `claude-code-system-prompts` 和 `open-claude-code` 深度分析

---

## 一、Claude Code 系统提示词架构

### 1.1 模块化分层设计

Claude Code 采用**模块化、分层设计**，而非单一大型提示词：

| 类别 | 文件数 | 用途 |
|------|--------|------|
| 主系统提示 (system-prompt-*) | 6 | 核心行为定义 |
| 工具描述 (tool-description-*) | 25+ | 每个工具的详细使用指南 |
| Agent 提示 (agent-prompt-*) | 18+ | 子代理的专用提示 |
| 系统提醒 (system-reminder-*) | 5 | 运行时状态提醒 |

### 1.2 核心设计原则

```markdown
# Tone and style
- Only use emojis if the user explicitly requests it.
- NEVER create files unless they're absolutely necessary
- ALWAYS prefer editing an existing file to creating a new one

# Professional objectivity
Prioritize technical accuracy and truthfulness over validating the user's beliefs.

# Planning without timelines
Provide concrete implementation steps without time estimates.

# Complete tasks fully
Do not stop mid-task or leave work incomplete.
Continue working until the task is done or the user stops you.
```

---

## 二、任务完成判断机制

### 2.1 多维度完成判断

Claude Code **不只依赖 task_done 工具**，而是综合多个信号：

```javascript
function shouldContinueExecution(response, toolResults, hookResults) {
    // 1. Hook 阻止继续
    if (hookResults.preventContinuation) return false;

    // 2. 显式停止原因
    if (hookResults.stopReason) return false;

    // 3. LLM 自然结束（无工具调用 + end_turn）
    if (response.stopReason === "end_turn" && !response.hasToolCalls) {
        return false;
    }

    // 4. 继续执行
    return true;
}
```

### 2.2 Hook 机制

Claude Code 有完整的 Hook 系统，支持：

| Hook 类型 | 触发时机 | 返回值作用 |
|-----------|----------|-----------|
| PreToolUse | 工具执行前 | 可阻止工具执行 |
| PostToolUse | 工具执行后 | 可阻止继续执行 |
| PromptSubmit | 用户提交前 | 可修改输入 |
| Stop | 任务停止时 | 清理资源 |

```javascript
// Hook 返回格式
{
    continue: boolean,           // 是否继续
    suppressOutput: boolean,     // 是否隐藏输出
    stopReason: string,          // 停止原因
    decision: "approve" | "block",
    permissionDecision: "allow" | "deny" | "ask"
}
```

---

## 三、Plan Mode 设计

### 3.1 Claude Code 的 Plan Mode 是可选的

**关键原则：** Plan Mode 只用于复杂任务，简单任务直接执行。

```markdown
## When to Use EnterPlanMode
1. New Feature Implementation - 新功能实现
2. Multiple Valid Approaches - 多种实现方案
3. Code Modifications - 影响现有代码的修改
4. Architectural Decisions - 架构决策
5. Multi-File Changes - 涉及多个文件
6. Unclear Requirements - 需求不清晰

## When NOT to Use
- Single-line fixes (typos, obvious bugs)
- Adding a single function with clear requirements
- Very specific, detailed instructions from user
- Pure research/exploration tasks
```

### 3.2 Plan Mode 五阶段工作流

```
Phase 1: Understanding (理解需求)
    ├── 使用 Explore agents 探索代码库
    └── 询问澄清问题
        |
        v
Phase 2: Designing (设计方案)
    ├── 使用 Plan agents 设计实现
    └── 考虑多种实现方案
        |
        v
Phase 3: Reviewing (审核方案)
    ├── 读取关键文件
    └── 向用户提问确认
        |
        v
Phase 4: Finalizing (编写计划)
    └── 将计划写入文件
        |
        v
Phase 5: ExitPlanMode (退出)
    └── 等待用户批准后开始实现
```

---

## 四、工具系统设计

### 4.1 工具并发执行

Claude Code 支持**智能并发执行**：

```javascript
class ToolExecutionQueue {
    canExecuteTool(isConcurrencySafe) {
        let executing = this.tools.filter(t => t.status === "executing");
        // 只有当：
        // 1. 没有工具在执行，或
        // 2. 所有执行中的工具都是并发安全的
        // 才能执行新工具
        return executing.length === 0 ||
               (isConcurrencySafe && executing.every(t => t.isConcurrencySafe));
    }
}
```

### 4.2 工具优先级规则

```markdown
# 专用工具优先于 Bash
- Read 代替 cat/head/tail
- Edit 代替 sed/awk
- Write 代替 echo/cat heredoc
- Grep 代替 grep/rg 命令

# 代码工具优先于文档工具
- 创建/修改代码 > 创建文档
- 执行测试 > 描述测试计划
```

---

## 五、Sage Agent 设计建议

### 5.1 系统提示词模块化

**当前问题：** 系统提示词硬编码在 `base.rs`

**建议架构：**

```
crates/sage-core/src/prompts/
├── mod.rs                    # 模块入口
├── system_prompt.rs          # 主系统提示词
├── tool_descriptions/        # 工具描述
│   ├── bash.rs
│   ├── edit.rs
│   └── ...
├── agent_prompts/            # 子代理提示词
│   ├── explore.rs
│   └── plan.rs
├── system_reminders.rs       # 运行时提醒
└── builder.rs                # 提示词构建器
```

**提示词构建器：**

```rust
pub struct SystemPromptBuilder {
    base_prompt: String,
    tool_descriptions: Vec<ToolDescription>,
    agent_type: Option<AgentType>,
    reminders: Vec<SystemReminder>,
}

impl SystemPromptBuilder {
    pub fn new() -> Self { ... }
    pub fn with_tools(mut self, tools: &[ToolSchema]) -> Self { ... }
    pub fn with_plan_mode(mut self, ctx: PlanModeContext) -> Self { ... }
    pub fn with_reminder(mut self, reminder: SystemReminder) -> Self { ... }
    pub fn build(&self) -> String { ... }
}
```

### 5.2 执行循环重构

**建议的执行循环：**

```rust
pub async fn run(&mut self, task: TaskMetadata) -> SageResult<ExecutionOutcome> {
    loop {
        // 1. 获取 LLM 响应
        let response = self.get_llm_response().await?;

        // 2. 无工具调用时检查是否结束
        if response.tool_calls.is_empty() {
            if self.completion_checker.should_end(&response) {
                return Ok(ExecutionOutcome::Completed(response));
            }
            continue;
        }

        // 3. 执行工具（支持 Hook）
        for tool_call in &response.tool_calls {
            // PreToolUse Hook
            let pre_result = self.hook_manager.run_pre_tool_use(tool_call).await?;
            if pre_result.should_prevent_continuation() {
                return Ok(ExecutionOutcome::StoppedByHook(pre_result.reason));
            }

            // 执行工具
            let result = self.tool_executor.execute(tool_call).await?;

            // PostToolUse Hook
            let post_result = self.hook_manager.run_post_tool_use(tool_call, &result).await?;
            if post_result.should_prevent_continuation() {
                return Ok(ExecutionOutcome::StoppedByHook(post_result.reason));
            }
        }

        // 4. 检查限制
        if self.is_budget_exceeded() || self.is_max_steps_reached() {
            return Ok(ExecutionOutcome::LimitReached);
        }
    }
}
```

### 5.3 任务完成验证增强

```rust
pub struct CompletionChecker {
    file_tracker: FileOperationTracker,
    task_type: TaskType,
}

impl CompletionChecker {
    pub fn check(&self, response: &LLMResponse, results: &[ToolResult]) -> CompletionStatus {
        // 1. 检查 task_done 调用
        if let Some(task_done) = self.find_task_done(results) {
            // 代码任务必须有文件操作
            if self.task_type.requires_code() && !self.file_tracker.has_operations() {
                return CompletionStatus::Continue {
                    reason: "No code files created/modified".into()
                };
            }
            return CompletionStatus::Completed { summary: task_done.summary };
        }

        // 2. 检查自然结束
        if response.stop_reason == StopReason::EndTurn && response.tool_calls.is_empty() {
            if self.is_natural_completion(response) {
                return CompletionStatus::Completed { ... };
            }
        }

        CompletionStatus::Continue { reason: "Task not complete".into() }
    }
}
```

### 5.4 Plan Mode 状态机

```rust
pub enum PlanModePhase {
    Understanding { explore_agents_launched: usize },
    Designing { plan_agents_launched: usize },
    Reviewing { critical_files_read: Vec<PathBuf> },
    Finalizing { plan_file_path: PathBuf },
    Exiting,
}

pub struct PlanModeManager {
    phase: PlanModePhase,
    plan_file_path: PathBuf,
}

impl PlanModeManager {
    pub async fn enter(&mut self, ctx: &TaskMetadata) -> SageResult<()> { ... }
    pub fn get_system_reminder(&self) -> String { ... }
    pub fn can_exit(&self) -> bool { ... }
    pub async fn exit(&mut self) -> SageResult<PlanModeResult> { ... }
}
```

---

## 六、优先级实现建议

| 优先级 | 任务 | 预期效果 |
|--------|------|----------|
| P0 | 系统提示词模块化 | 基础架构，支持后续扩展 |
| P0 | 任务完成验证增强 | 防止只有计划无代码 |
| P1 | Hook 机制实现 | 支持用户自定义行为 |
| P1 | 工具并发执行 | 性能提升 |
| P2 | Plan Mode 五阶段 | 复杂任务处理 |
| P2 | 子代理类型化 | Explore/Plan/CodeReview |

---

## 七、关键差异对比

| 方面 | Claude Code | Sage Agent (当前) | 建议目标 |
|------|-------------|------------------|----------|
| 提示词组织 | 模块化文件 | 硬编码 | 模块化 |
| 执行循环 | Hook 驱动 | 简单循环 | 支持 Hook |
| 完成判断 | 多信号综合 | 单一 task_done | 多维度验证 |
| Plan Mode | 可选/5阶段 | 过度使用 | 可选/快速 |
| 工具执行 | 智能并发 | 串行 | 支持并发 |
| 子代理 | 类型化 (18+种) | 通用型 | 类型化 |

---

*分析时间: 2025-12-18*
*基于: claude-code-system-prompts, open-claude-code*
