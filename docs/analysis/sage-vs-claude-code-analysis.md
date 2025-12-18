# Sage Agent vs Claude Code 设计差异分析

> 基于 trajectory `sage_20251217_170636.json` 分析
> 用户请求: "帮我设计一个天气网站 你可以搜索下看下天气的api"
> 实际结果: 只生成了设计文档，没有写任何代码

## 📋 问题摘要

| 问题类别 | 严重程度 | 描述 |
|---------|---------|------|
| 任务理解 | 🔴 严重 | "设计网站" 被理解为 "生成设计文档" 而非 "实现代码" |
| 过早终止 | 🔴 严重 | Plan Mode 结束后直接调用 task_done，没有执行实现 |
| Agent Loop | 🔴 严重 | 缺乏持续执行循环，9步就结束了 |
| 工具选择 | 🟡 中等 | 没有使用 Write/Bash 等编码工具 |
| 错误处理 | 🟡 中等 | Web Search 失败后没有合理降级 |

---

## 🔍 问题详细分析

### 1. 任务理解偏差

**Trajectory 中的表现:**
```
用户: "帮我设计一个天气网站"
Agent理解: → enter_plan_mode → sequentialthinking → add_tasks → exit_plan_mode → task_done
结果: 只输出了设计文档
```

**Claude Code 的行为预期:**
```
用户: "帮我设计一个天气网站"
Claude Code理解: → 分析需求 → 创建项目结构 → 编写代码文件 → 验证运行
结果: 实际可运行的代码
```

**根本原因:**
- 中文"设计"在用户语境中通常等同于"实现"
- 系统提示词没有明确指导如何理解这类模糊请求
- 缺少 "ask for clarification" 的触发机制

---

### 2. 过早终止问题

**Trajectory 执行流程:**
```
Step 1: web-search x2 (失败，返回 placeholder)
Step 2: web-search x2 (再次失败)
Step 3: web-search x2 (仍然失败)
Step 4: enter_plan_mode
Step 5: sequentialthinking (设计思考)
Step 6: sequentialthinking (详细设计)
Step 7: add_tasks (添加10个任务)
Step 8: exit_plan_mode
Step 9: task_done ← 直接结束！
```

**问题关键:**
- Plan Mode 只是规划阶段
- exit_plan_mode 后应该进入 **Implementation Mode**
- 但 Agent 直接调用了 task_done

**系统提示词中的问题语句:**
```
## CRITICAL: Task Completion Rules
**ALWAYS call `task_done` when you have completed the user's request!**
```

这导致 Agent 认为"设计完成 = 任务完成"。

---

### 3. 缺乏 Agentic Loop

**Claude Code 的执行模式:**
```
while not truly_completed:
    response = llm.generate(context + tools)
    if has_tool_calls:
        results = execute_tools(tool_calls)
        context.append(results)
    if needs_continuation:
        continue_with_results()
    if user_confirms_done:
        break
```

**Sage Agent 当前模式:**
```
for step in range(max_steps):
    response = llm.generate()
    if task_done_called:
        break  ← 没有验证实际完成
```

**关键差异:**
| 特性 | Claude Code | Sage Agent |
|------|-------------|------------|
| 完成判断 | 多重验证 + 用户确认 | 单一 task_done 调用 |
| 持续性 | 直到真正完成 | 容易过早终止 |
| 进度追踪 | 持续反馈 | 只在结束时输出 |

---

### 4. 工具选择问题

**Trajectory 中使用的工具:**
```
✅ web-search (6次，全部失败)
✅ enter_plan_mode (1次)
✅ sequentialthinking (2次)
✅ add_tasks (1次)
✅ exit_plan_mode (1次)
✅ task_done (1次)
```

**应该使用但没使用的工具:**
```
❌ Write - 创建代码文件
❌ bash - 初始化项目、安装依赖、运行测试
❌ str_replace_based_edit_tool - 编辑代码
❌ ask_user_question - 确认是否需要实际实现
```

---

### 5. Web Search 失败处理

**问题:**
- 连续6次搜索全部返回 `placeholder`
- Agent 没有切换策略
- 应该使用内置知识或直接开始编码

**期望的降级策略:**
```python
if web_search_failed:
    if can_use_builtin_knowledge:
        proceed_with_known_apis()  # OpenWeatherMap, Open-Meteo 等
    else:
        ask_user("搜索失败，是否继续使用已知的天气API?")
```

---

## 📊 Claude Code vs Sage Agent 对比清单

### A. 系统设计层面

| 维度 | Claude Code | Sage Agent (当前) | 建议改进 |
|------|-------------|------------------|---------|
| **执行模式** | 响应驱动持续循环 | 有限步数，易终止 | 增加完成验证机制 |
| **任务理解** | 倾向于执行 | 倾向于规划 | 修改系统提示词 |
| **中断机制** | 用户主动中断 | task_done 即结束 | 增加二次确认 |
| **错误恢复** | 智能降级 | 简单重试 | 增加降级策略 |

### B. 工具系统层面

| 维度 | Claude Code | Sage Agent (当前) | 建议改进 |
|------|-------------|------------------|---------|
| **工具并行** | 智能批量执行 | ✅ 已实现 | - |
| **权限控制** | 精细粒度 | ✅ 已实现 | - |
| **Plan Mode** | 可选，不强制 | 过度使用 | 简化规划流程 |
| **代码工具** | 优先使用 | 优先文档 | 调整工具优先级 |

### C. 提示词层面

| 维度 | Claude Code | Sage Agent (当前) | 建议改进 |
|------|-------------|------------------|---------|
| **任务完成定义** | 代码可运行 | 文档完成 | 重新定义完成标准 |
| **行动倾向** | "Just do it" | "Plan first" | 平衡规划与执行 |
| **用户意图理解** | 默认实现 | 默认规划 | 增加意图识别 |
| **确认机制** | 关键操作前确认 | 很少确认 | 增加 ask_user 调用 |

### D. 用户体验层面

| 维度 | Claude Code | Sage Agent (当前) | 建议改进 |
|------|-------------|------------------|---------|
| **反馈频率** | 持续实时 | 步骤间 | 增加进度反馈 |
| **结果展示** | 代码 + 说明 | 主要是文档 | 强调代码输出 |
| **交互流畅度** | 自然对话 | 结构化流程 | 减少中间态 |

---

## 🛠 具体改进建议

### 1. 修改系统提示词

**移除或修改:**
```diff
- **ALWAYS call `task_done` when you have completed the user's request!**
+ **Only call `task_done` after:
+   1. Code has been written and saved
+   2. The implementation is testable/runnable
+   3. You've verified the core functionality works
+ If the user asks to "design" or "create" something, they expect working code, not just documentation.**
```

**添加新指导:**
```markdown
## Task Interpretation Rules
- When users say "设计/design", "创建/create", "做/make" a website/app/system:
  → They expect WORKING CODE, not documentation
  → Start coding immediately after brief planning
  → Only create docs if explicitly requested

## Execution Priority
1. PREFER action over planning
2. PREFER code over documentation
3. PREFER asking user over making assumptions
4. PREFER smaller working increments over big plans
```

### 2. 添加任务完成验证

```rust
// 在 ClaudeStyleAgent 中添加
fn verify_task_completion(&self, task_type: TaskType) -> CompletionStatus {
    match task_type {
        TaskType::CreateWebsite | TaskType::CreateApp => {
            // 检查是否有代码文件被创建
            if self.files_created.is_empty() {
                return CompletionStatus::Incomplete("No code files created");
            }
            // 检查是否可运行
            if !self.verified_runnable {
                return CompletionStatus::NeedsVerification;
            }
        }
        TaskType::FixBug => {
            // 检查是否有修改
            if self.files_modified.is_empty() {
                return CompletionStatus::Incomplete("No files modified");
            }
        }
        _ => {}
    }
    CompletionStatus::Complete
}
```

### 3. 改进 Plan Mode 流程

```rust
// exit_plan_mode 后应该自动进入实现阶段
pub async fn exit_plan_mode(&mut self) -> Result<()> {
    self.plan_mode = false;

    // 不要直接结束，而是开始实现
    if self.has_implementation_tasks() {
        self.start_implementation_phase().await?;
    }

    Ok(())
}
```

### 4. 增加意图识别

```rust
pub fn detect_user_intent(message: &str) -> UserIntent {
    let keywords_code = ["设计", "创建", "开发", "做", "写", "实现"];
    let keywords_plan = ["规划", "计划", "分析", "评估"];
    let keywords_doc = ["文档", "说明", "readme"];

    // 检查关键词
    if contains_any(message, &keywords_doc) {
        return UserIntent::Documentation;
    }
    if contains_any(message, &keywords_code) {
        return UserIntent::Implementation;
    }
    if contains_any(message, &keywords_plan) {
        return UserIntent::Planning;
    }

    // 默认倾向于实现
    UserIntent::Implementation
}
```

### 5. 添加降级策略

```rust
pub async fn handle_search_failure(&mut self, attempts: u32) -> Strategy {
    if attempts >= 2 {
        // 切换到内置知识
        return Strategy::UseBuiltinKnowledge;
    }
    if attempts >= 4 {
        // 询问用户
        return Strategy::AskUser("Search failed. Proceed with known APIs?");
    }
    Strategy::Retry
}
```

---

## 📁 需要修改的文件清单

| 文件 | 修改类型 | 优先级 |
|------|---------|--------|
| `crates/sage-core/src/agent/reactive_agent.rs` | 添加完成验证 | 🔴 高 |
| `crates/sage-core/src/agent/prompts.rs` (需创建) | 系统提示词 | 🔴 高 |
| `crates/sage-cli/src/claude_mode.rs` | 改进用户交互 | 🟡 中 |
| `crates/sage-core/src/tools/mod.rs` | 工具优先级 | 🟡 中 |
| `crates/sage-core/src/agent/intent.rs` (需创建) | 意图识别 | 🟢 低 |

---

## 🎯 期望的执行流程对比

### 当前流程 (有问题)
```
用户请求 → 搜索(失败) → 进入Plan Mode → 思考 → 添加任务 → 退出Plan → 结束(task_done)
                                              ↑
                                        没有实际执行任何代码
```

### 期望流程 (Claude Code 风格)
```
用户请求 → 简短分析 → 创建项目结构 → 编写核心代码 → 编写配置文件
    → 测试基础功能 → 输出结果给用户 → 等待反馈 → 迭代改进
```

---

## 📈 改进后的预期效果

1. **用户说"设计一个天气网站"**
   - ✅ 创建项目目录结构
   - ✅ 生成 React/Vue 组件代码
   - ✅ 配置天气API调用
   - ✅ 提供运行命令

2. **遇到搜索失败**
   - ✅ 自动降级使用内置知识
   - ✅ 或询问用户偏好

3. **任务完成判断**
   - ✅ 检查代码文件是否创建
   - ✅ 验证基本可运行性
   - ✅ 获取用户确认

---

## ✅ 已完成的修改 (2025-12-18)

根据以上分析，已对 Sage Agent 进行了以下修改：

### 1. FileOperationTracker 修复
**文件:** `crates/sage-core/src/agent/reactive_agent.rs`
- ✅ 初始化 `file_tracker` 字段
- ✅ 在工具执行后追踪文件操作
- ✅ 在 task_done 调用时检查是否有文件操作

### 2. task_done 工具强化
**文件:** `crates/sage-tools/src/tools/task_mgmt/task_done.rs`
- ✅ 修改工具描述，强调必须有代码产出
- ✅ 明确禁止只有计划/文档时调用

### 3. Plan Mode 工具改进
**文件:** `crates/sage-tools/src/tools/planning/enter_plan_mode.rs`
- ✅ 修改描述为 "QUICK plan mode"
- ✅ 添加 2 分钟时间限制提示
- ✅ 强调 "Plans without code are WORTHLESS"

**文件:** `crates/sage-tools/src/tools/planning/exit_plan_mode.rs`
- ✅ 修改输出为强制实现模式
- ✅ 添加 "YOU MUST NOW START WRITING CODE IMMEDIATELY"

### 4. 系统提示词优化
**文件:** `crates/sage-core/src/agent/base.rs`
- ✅ 在最开头添加 "CODE-FIRST EXECUTION" 强制规则
- ✅ 明确 "设计/创建/实现 = 写代码"
- ✅ 强化任务完成规则，禁止只有计划时完成

### 5. 搜索失败降级策略
**文件:** `crates/sage-tools/src/tools/network/web_search.rs`
- ✅ 修改工具描述，提示使用内置知识
- ✅ 搜索失败时输出明确的降级指导
- ✅ 提供常见 API 示例 (OpenWeatherMap, Open-Meteo)

### 测试结果
```
cargo build    ✅ 成功
cargo test     ✅ 全部通过 (239 tests passed)
```

---

*文档生成时间: 2025-12-18*
*分析基于: sage_20251217_170636.json*
*修改完成时间: 2025-12-18*
