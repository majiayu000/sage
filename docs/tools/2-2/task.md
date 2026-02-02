# Task Tool

## 基本信息

| 属性 | 值 |
|------|-----|
| 工具名称 | Task |
| Claude Code 版本 | 2.1.15 |
| 类别 | 执行工具 |
| Sage 实现 | `sage-tools/src/tools/process/task/` |

## 功能描述

启动专门的子 Agent 来自主处理复杂的多步骤任务。

## 完整 Prompt

```markdown
Launch a new agent to handle complex, multi-step tasks autonomously.

The ${TASK_TOOL} tool launches specialized agents (subprocesses) that autonomously handle complex tasks. Each agent type has specific capabilities and tools available to it.

Available agent types and the tools they have access to:
${AGENT_TYPE_REGISTRY_STRING}

When using the ${TASK_TOOL} tool, you must specify a subagent_type parameter to select which agent type to use.

When NOT to use the ${TASK_TOOL} tool:
- If you want to read a specific file path, use the ${READ_TOOL} or ${GLOB_TOOL} tool instead of the ${TASK_TOOL} tool, to find the match more quickly
- If you are searching for a specific class definition like "class Foo", use the ${GLOB_TOOL} tool instead, to find the match more quickly
- If you are searching for code within a specific file or set of 2-3 files, use the ${READ_TOOL} tool instead of the ${TASK_TOOL} tool, to find the match more quickly
- Other tasks that are not related to the agent descriptions above

Usage notes:
- Always include a short description (3-5 words) summarizing what the agent will do
- Launch multiple agents concurrently whenever possible, to maximize performance; to do that, use a single message with multiple tool uses
- When the agent is done, it will return a single message back to you. The result returned by the agent is not visible to the user. To show the user the result, you should send a text message back to the user with a concise summary of the result.
- You can optionally run agents in the background using the run_in_background parameter. When an agent runs in the background, the tool result will include an output_file path. To check on the agent's progress or retrieve its results, use the ${READ_TOOL} tool to read the output file, or use ${BASH_TOOL} with `tail` to see recent output. You can continue working while background agents run.
- Agents can be resumed using the `resume` parameter by passing the agent ID from a previous invocation. When resumed, the agent continues with its full previous context preserved. When NOT resuming, each invocation starts fresh and you should provide a detailed task description with all necessary context.
- When the agent is done, it will return a single message back to you along with its agent ID. You can use this ID to resume the agent later if needed for follow-up work.
- Provide clear, detailed prompts so the agent can work autonomously and return exactly the information you need.
- Agents with "access to current context" can see the full conversation history before the tool call. When using these agents, you can write concise prompts that reference earlier context (e.g., "investigate the error discussed above") instead of repeating information. The agent will receive all prior messages and understand the context.
- The agent's outputs should generally be trusted
- Clearly tell the agent whether you expect it to write code or just to do research (search, file reads, web fetches, etc.), since it is not aware of the user's intent
- If the agent description mentions that it should be used proactively, then you should try your best to use it without the user having to ask for it first. Use your judgement.
- If the user specifies that they want you to run agents "in parallel", you MUST send a single message with multiple ${TASK_TOOL_OBJECT.name} tool use content blocks.
```

## 参数

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| prompt | string | ✅ | 任务描述 |
| description | string | ✅ | 简短描述 (3-5 词) |
| subagent_type | string | ✅ | Agent 类型 |
| run_in_background | boolean | ❌ | 是否后台运行 |
| resume | string | ❌ | 恢复之前的 Agent ID |
| model | string | ❌ | 使用的模型 |

## 设计原理

### 1. 专门化 Agent 类型
**为什么**:
- 不同任务需要不同工具集
- 减少单个 Agent 的复杂度
- 提高任务完成质量

**常见 Agent 类型**:
| 类型 | 用途 | 工具 |
|------|------|------|
| Explore | 代码库探索 | Glob, Grep, Read |
| Bash | 命令执行 | Bash |
| Plan | 实现规划 | 所有只读工具 |

### 2. 并行执行
**为什么**:
- 多个独立任务可同时进行
- 减少总体等待时间
- 提高效率

### 3. 后台运行
**为什么**:
- 长时间任务不阻塞主流程
- 可以继续其他工作
- 通过文件检查进度

### 4. Agent 恢复
**为什么**:
- 保留之前的上下文
- 支持增量工作
- 避免重复探索

### 5. 结果不可见用户
**为什么**:
- 主 Agent 需要总结结果
- 避免信息过载
- 保持对话简洁

## 使用场景

### ✅ 应该使用
- 复杂的代码库探索
- 多轮搜索任务
- 需要自主决策的任务
- 并行执行多个独立任务

### ❌ 不应该使用
- 读取特定文件 (使用 Read)
- 简单的文件搜索 (使用 Glob)
- 搜索 2-3 个文件 (使用 Read)
- 简单的单步任务

## 示例

```markdown
# 探索代码库
Task(
  subagent_type: "Explore",
  description: "Find error handlers",
  prompt: "Search for all error handling patterns in the codebase"
)

# 并行执行
Task(subagent_type: "Bash", description: "Run tests", prompt: "cargo test")
Task(subagent_type: "Bash", description: "Check lint", prompt: "cargo clippy")
```

## Sage 实现差异

Sage 的 TaskTool 支持：
- 更多 Agent 类型
- 自定义工具集
- Agent 间通信
- 任务依赖管理
