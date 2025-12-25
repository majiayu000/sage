# Sage Agent vs Open Claude Code 关键功能对比分析

## 概述

本文档对比分析了 Sage Agent 和 Open Claude Code (Claude Code 的反编译版本) 的关键功能，
为 SWE-bench 优化提供参考。

---

## 1. System Prompt 结构对比

### Claude Code 特点
- **模块化条件组合**: 根据可用工具动态组合提示段
- **Bug Fix 专门指导** (来自 `src/transform/map/p3_z1_m1_n2.js`):
  ```
  - A bug fix doesn't need surrounding code cleaned up
  - A simple feature doesn't need extra configurability
  - Don't add docstrings, comments, or type annotations to code you didn't change
  - Don't create helpers, utilities, or abstractions for one-time operations
  - Don't design for hypothetical future requirements
  ```
- **简化原则 (Anti-Over-Engineering)**:
  - 不添加额外功能、重构或"改进"
  - 不为假设的未来需求设计
  - 三行相似代码优于过早抽象

### Sage Agent 现状
- 有类似的模块化结构 (`system_prompt.rs`)
- 有 DOING_TASKS 部分
- **缺少**: 针对 bug fix 的专门提示

### 建议改进
在 `DOING_TASKS` 中添加 SWE-bench 模式专用段落，强调：
- 修复 bug 时只修改必要的代码
- 先运行测试验证理解
- 修改后必须运行测试

---

## 2. TodoWrite / 任务管理对比

### Claude Code 实现 (来自 `decompiled/readable_v2/tools/tools_006.js`)

```javascript
// 任务状态
STATUS_SCHEMA = _.enum(["pending", "in_progress", "completed"])

// 任务必须有两种形式
TODO_ITEM_SCHEMA = _.object({
    content: _.string().min(1),       // 祈使句形式: "Fix bug"
    status: STATUS_SCHEMA,
    activeForm: _.string().min(1)     // 进行时形式: "Fixing bug"
})

// 关键规则
- 只能有 ONE task 处于 in_progress
- 完成后立即标记为 completed
- 不要批量完成任务
```

**提醒机制** (来自 `decompiled/readable_v2/tools/tools_010.js`):
```javascript
// 检查上次使用 TodoWrite 后的轮次
let {turnsSinceLastTodoWrite, turnsSinceLastReminder} = tV5(messages);

// 达到阈值时触发提醒
if (turnsSinceLastTodoWrite >= TURNS_SINCE_WRITE &&
    turnsSinceLastReminder >= TURNS_BETWEEN_REMINDERS) {
    return [{ type: "todo_reminder", content: todos, itemCount: todos.length }];
}

// 提醒内容
"The TodoWrite tool hasn't been used recently. If you're working on tasks
that would benefit from tracking progress, consider using the TodoWrite tool
to track progress."
```

### Sage Agent 现状
- 有 TODO_TOOL_NAME 配置
- 有基本的任务管理
- **缺少**: 自动提醒机制

### 建议改进
- 添加 `turnsSinceLastTodoWrite` 跟踪
- 实现自动提醒系统
- 在 SWE-bench 模式中可以禁用（减少噪音）

---

## 3. 执行循环对比

### Claude Code 实现
- **无明确步数限制**: 没有找到 max_steps 常量
- **使用"gentle reminder"系统**: 而不是硬性限制
- **无限上下文**: 通过 automatic summarization 支持

### Sage Agent 现状 (`execution_loop.rs`)
```rust
// 步数限制
if let Some(max) = max_steps.filter(|&max| step_number > max) {
    tracing::warn!("Reached maximum steps: {}", max);
    execution.complete(false, Some("Reached maximum steps".to_string()));
    break 'execution_loop ExecutionOutcome::MaxStepsReached { execution };
}

// 重复检测
const MAX_RECENT_OUTPUTS: usize = 3;
const REPETITION_THRESHOLD: usize = 2;
```

### 建议改进
- SWE-bench 模式: 保持 max_steps 但设为更高值或 None
- 添加"分析步数"限制（不同于总步数）
- 实现进度提示而非硬性终止

---

## 4. Thinking/Analysis 控制对比

### Claude Code 实现 (来自 `src/message/thinking_n1.js`)
```javascript
function is_thinking_enabled() {
    // 环境变量控制
    if (process.env.MAX_THINKING_TOKENS)
        return parseInt(process.env.MAX_THINKING_TOKENS, 10) > 0;

    // 配置控制
    let config = getSettings().alwaysThinkingEnabled;
    if (config === true || config === false) return config;

    // 模型默认值
    if (!getDefaultSonnetModel().includes("claude-sonnet-4-5"))
        return false;
    return true;
}
```

**Thinking 显示控制**:
- `ctrl+<key>` 快捷键显示/隐藏 thinking 内容
- 计时显示: "∴ Thought for N seconds"
- 过滤 trailing thinking blocks

### Sage Agent 现状
- 使用 `sequential_thinking` 作为独立工具
- 没有 thinking token 限制
- 没有启用/禁用开关

### 建议改进
- 在 SWE-bench 模式考虑限制或禁用 sequential_thinking
- 如果使用，添加使用次数限制
- "分析麻痹"问题的根源可能在这里

---

## 5. 行动导向 vs 分析导向

### Claude Code 设计哲学
从 prompt 中可以看到强烈的"行动导向"：
```
IMPORTANT: Prefer taking action over asking questions.
For most tasks, make reasonable default choices and proceed.
```

### Sage Agent 建议
为 SWE-bench 添加更强的行动导向提示：

```rust
pub const SWEBENCH_ACTION_ORIENTED: &'static str = r#"# SWE-bench Bug Fixing Mode

CRITICAL: This is a bug fixing task. Follow this exact workflow:

1. UNDERSTAND (max 30% of effort):
   - Read the bug description
   - Search for relevant code
   - Read the failing test

2. VERIFY UNDERSTANDING (mandatory):
   - Run the failing test to confirm the error
   - Identify the EXACT line(s) causing the issue

3. FIX (focused changes only):
   - Make the MINIMUM changes needed
   - Do NOT refactor surrounding code
   - Do NOT add extra features

4. VERIFY FIX (mandatory):
   - Run the test again
   - Confirm it passes

WARNING: If you spend more than 10 steps without calling Edit,
you are likely in analysis paralysis. Take action NOW.
"#;
```

---

## 6. 编辑工具反馈对比

### Claude Code Edit 工具
没有找到明确的增强反馈机制，但有：
- 详细的权限检查
- 结构化的输出 schema
- IDE 差异展示集成

### Sage Agent 建议
增强 Edit 工具的反馈：
```rust
// 成功时包含更多上下文
EditResult {
    success: true,
    modified_file: "path/to/file.py",
    changed_lines: (45, 50),
    // 添加: 周围代码片段
    context_before: "def method():\n    old_line",
    context_after: "def method():\n    new_line",
    // 添加: 提示运行测试
    suggestion: "Run tests to verify this change"
}
```

---

## 7. Plan Mode 对比

### Claude Code 实现 (来自 `src/agents/plan_z1_m1_n1.js`)
```javascript
// Plan Mode 退出选项
function agents_plan(outcome) {
    if (outcome === "yes-bypass-permissions") {
        setHasExitedPlanMode(true);
        // 设置 mode: "bypassPermissions"
    } else if (outcome === "yes-accept-edits") {
        setHasExitedPlanMode(true);
        // 设置 mode: "acceptEdits"
    } else if (outcome === "yes-default") {
        setHasExitedPlanMode(true);
        // 设置 mode: "default"
    }
}

// 支持外部编辑器编辑 plan
if (ctrl && key === "e") {
    let content = openEditorAndReadFile(planFilePath);
    // 更新 plan 内容
}
```

### Sage Agent 现状
- 没有 Plan Mode
- 没有权限模式切换

### 建议
- SWE-bench 模式可以直接使用 "bypassPermissions" 模式
- 减少不必要的确认步骤

---

## 总结: 代码调整优先级

### 高优先级 (直接影响 SWE-bench 成功率)

| 调整项 | Sage 位置 | 参考 Claude Code |
|--------|-----------|------------------|
| 添加测试验证提示 | `system_prompt.rs` DOING_TASKS | 类似结构 |
| 添加分析步数限制 | `execution_loop.rs` | Claude 无此限制，用 reminder |
| 添加 SWE-bench 模式提示 | `system_prompt.rs` 新增 | - |

### 中优先级

| 调整项 | Sage 位置 | 参考 Claude Code |
|--------|-----------|------------------|
| 限制 sequential_thinking | 工具配置 | thinking 开关 |
| 增强 Edit 反馈 | Edit 工具 | 类似设计 |
| 添加步数进度提示 | `execution_loop.rs` | gentle reminder |

### 低优先级

| 调整项 | Sage 位置 | 参考 Claude Code |
|--------|-----------|------------------|
| Docker 容器修复 | 评估脚本 | - |
| 权限错误处理 | 工具执行 | 权限检查 |

---

## 附录: 工具设计共性 (保留原文档内容)

### 设计共性

- 输入校验：使用 Zod schema 对工具参数进行严格验证，减少运行时分支和错误提示歧义。
- 权限与确认：工具调用前通常配套权限确认 UI 和策略规则，确保执行可控。
- 懒加载：工具定义与 UI 组件通过 lazy loader 初始化，降低启动成本。
- 流式与进度：长耗时工具输出通常支持进度/流式展示，避免阻塞 UI。

### Bash 工具（shell 执行）

- 命令校验与防注入：解析 shell 命令，结合策略进行拦截与限制。
- 沙箱与权限规则：支持沙箱模式、命令级权限管理。
- 进度与输出跟踪：支持后台执行、输出流追踪与 UI 反馈。
- 结果展示组件：React/Ink 组件用于渲染进度、错误和输出。

### 文件类工具（Read/Write/Edit）

- Write：写入前权限校验，写入过程提供 diff 预览与高亮。
- Edit：编辑操作与 IDE 差异展示结合，支持用户确认后落盘。
- Read：读操作与权限系统绑定，作为 Write 的前置条件。

### 搜索类工具（Glob/Grep）

- Grep：支持正则与多种输出模式（内容/文件/计数），并结合 UI 渲染结果。
- Glob：用于路径匹配，强调和 Grep 配合进行高效定位。
- 两者都被设计为"替代 Bash 中的 grep/find"，以便统一权限与输出格式。

### WebSearch / WebFetch 工具

- WebSearch：支持 `allowed_domains` / `blocked_domains` 域名过滤
- WebFetch：URL 安全校验、重定向处理、15分钟缓存、最大10MB内容限制
