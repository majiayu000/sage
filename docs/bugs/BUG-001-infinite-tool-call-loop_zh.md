# BUG-001: Agent 无限工具调用循环导致无输出

## 问题概述

Agent 在处理开放式分析任务时，陷入无限工具调用循环，持续探索代码库但从未输出最终结论。

## 复现场景

**用户输入：**
```
看下这个库 和这个库上一层目录下的openclaudecode 对比一下 有哪些可以改进的
你应该看openclaude的cli.js 而不是看md等 你先分析
```

**预期行为：** Agent 探索代码后输出对比分析报告

**实际行为：** Agent 连续调用 9 次工具，累计消耗 817,608 tokens，耗时 359.5 秒，但最终没有任何文本输出

## 问题分析

### 会话日志分析

Session ID: `a1524a75-7867-449d-a955-34d3ab30d8e9`

| 步骤 | inputTokens | outputTokens | content | 工具调用 |
|------|-------------|--------------|---------|----------|
| 2 | 23,291 | 70 | 有文本 | Glob, bash |
| 3 | 57,196 | 110 | 有文本 | 3x Glob |
| 4 | 57,357 | 143 | 空 | Read, 2x bash |
| 5 | 59,762 | 149 | 有文本 | Read, 3x Glob |
| 6 | 120,697 | 164 | 有文本 | Read, bash |
| 7 | 121,892 | 146 | 有文本 | Read, bash |
| 8 | 124,312 | 371 | 有文本 | Task subagent |
| 9 | 124,687 | 133 | 有文本 | 3x 工具 |
| 10 | 127,040 | 88 | **空** | Read, bash |

### 关键发现

1. **最后一条消息 content 为空字符串** - LLM 只输出工具调用，没有文本
2. **没有 error 类型的日志记录** - 不是程序错误
3. **Task subagent 被调用** - 第 8 条启动了子任务，可能长时间运行
4. **持续的工具调用模式** - Agent 不断获取更多信息但不总结

### 根本原因

**LLM 行为问题：** 模型在处理开放式对比任务时：
- 持续调用工具探索代码库
- 从未判断"信息已足够"
- 没有触发"停止探索、开始总结"的决策

这不是超时或程序错误，而是 Agent 决策逻辑缺陷。

## 影响

- 用户等待数分钟后没有得到任何有价值的输出
- 大量 token 消耗造成成本浪费
- 用户体验极差

## 建议修复方案

### 1. 添加最大工具调用次数限制

```rust
// 在 agent 配置中添加
pub struct AgentConfig {
    /// 单次响应最大连续工具调用次数
    pub max_consecutive_tool_calls: u32,  // 建议默认值: 10
}
```

当达到限制时，强制 agent 输出文本总结。

### 2. 连续空 content 检测

```rust
// 检测连续多次 content 为空的响应
if consecutive_empty_content_count >= 3 {
    // 注入系统提示：要求输出总结
    inject_summary_prompt();
}
```

### 3. Task subagent 超时控制

```rust
pub struct TaskConfig {
    /// 子任务最大执行时间（秒）
    pub subagent_timeout_secs: u64,  // 建议默认值: 120
}
```

### 4. 改进系统提示

在 system prompt 中添加明确指导：

```
当你已经收集了足够的信息来回答用户问题时，停止调用工具并直接输出分析结论。
不要无限制地探索代码库。如果你已经调用了超过 5 次工具，考虑是否该总结了。
```

### 5. 进度反馈机制

让用户知道 agent 在做什么：

```
[探索中] 已读取 5 个文件，发现 3 个关键模块...
[分析中] 正在对比架构差异...
```

## 相关文件

- `crates/sage-core/src/agent/` - Agent 执行逻辑
- `crates/sage-core/src/trajectory/entry.rs` - 日志记录格式
- `~/.sage/sessions/` - 会话存储位置

## 优先级

**高** - 直接影响用户体验和成本

## 状态

- [x] 问题确认
- [x] 根因分析
- [x] 方案设计
- [x] 代码实现（部分）
- [ ] 测试验证

## 已实现的改进

### 2026-01-08 修复

1. **错误消息记录到 messages.jsonl**
   - 新增 `EnhancedMessageType::Error` 类型
   - 新增 `EnhancedMessage::error()` 构造方法
   - 在 `execution_loop.rs` 中调用 `record_error_message()` 记录错误

2. **CLI 显示详细错误信息**
   - 显示错误类型（api_error, timeout_error, rate_limit_error 等）
   - 显示 provider 信息
   - 显示 session 日志路径

3. **API 调试日志**
   - 设置 `SAGE_DEBUG_API=1` 启用
   - 请求保存到 `$SAGE_DEBUG_DIR/glm_request_*.json`
   - 错误响应保存到 `$SAGE_DEBUG_DIR/glm_error_*.json`
   - 默认目录：`/tmp/sage_debug`

### 使用方法

```bash
# 启用 API 调试日志
SAGE_DEBUG_API=1 sage "your task"

# 指定调试目录
SAGE_DEBUG_API=1 SAGE_DEBUG_DIR=~/sage_debug sage "your task"
```

## 第二次复现

**Session ID**: `69c9d4cf-6c80-447d-9aa3-21a9fba915a3`

**执行统计：**
- 22 steps（工具调用）
- 724,904 input tokens
- 2,136 output tokens
- 660.7 秒（11 分钟）
- 状态：**Failed**
- 消息数：38 条

**关键发现：**
- Agent 创建了 TodoWrite，计划"总结关键点"
- 但在执行总结之前会话就 Failed 了
- 最后一条消息 `content: ""` + TodoWrite 调用

**可能的失败原因：**
1. 超时限制（660s 太长）
2. Token 限制（724k tokens）
3. 模型返回异常

## 详细时间线分析（第二次复现）

### 关键时间点

| 时间 | 事件 | 说明 |
|------|------|------|
| 07:33:13 | 用户提问 | 对比两个项目 |
| 07:39:24 | 首次响应 | **延迟 6 分钟！** Task subagent 执行 |
| 07:39:24 - 07:42:49 | 大量工具调用 | 20+ 次 bash/Read/Glob |
| 07:43:00 | TodoWrite | 准备"总结关键点" |
| 07:43:14 | 最后一条消息 | TodoWrite 设置 in_progress |
| ??? | **Failed** | 未能输出总结 |

### 发现的子问题

1. **Task subagent 低效**
   - 启动了 2 个 Explore subagent
   - 耗时 6 分钟
   - 但结果似乎没被使用（主 agent 后续重复探索）

2. **多次空 content**
   - 消息 24, 26, 28, 29, 35, 37, 38 的 content 都是空字符串
   - LLM 只输出工具调用，不输出解释文字

3. **准备总结时失败**
   - 消息 38 设置 todo "总结关键点" 为 in_progress
   - 下一步应该输出总结文本
   - 但会话直接 Failed，没有后续消息

### 结论

这不是纯粹的"无限循环"，而是**复合问题**：

1. **Subagent 效率问题** - 启动了 2 个 Explore subagent，耗时 6 分钟但结果未被有效使用
2. **重复劳动** - 主 agent 后续重复了 subagent 已经做过的探索工作
3. **在即将输出结论时失败** - Agent 已完成所有准备工作，TodoWrite 设置了 "总结关键点" 为 in_progress，但下一次 LLM 调用失败

### 失败的直接原因分析

从代码分析（`crates/sage-core/src/agent/unified/`）：

1. **execution_loop.rs:115-148** - 当 `execute_step` 返回 `Err(e)` 时：
   - 记录 error 到 session
   - 设置 `ExecutionOutcome::Failed`
   - 但**错误内容没有被持久化到 messages.jsonl**（只记录了 "execution_error" 类型）

2. **step_execution.rs:66-67** - LLM 调用失败点：
   ```rust
   let llm_response = select! {
       response = self.llm_client.chat(messages, Some(tool_schemas)) => {
           response?  // <-- 这里如果失败会返回 Err
       }
       ...
   };
   ```

3. **glm.rs:103-118** - GLM API 错误处理：
   - HTTP 请求失败 → `SageError::llm("GLM API request failed")`
   - 非 2xx 状态码 → `SageError::llm("GLM API error (status X)")`
   - 响应解析失败 → `SageError::llm("Failed to parse GLM response")`

### 推测的失败原因

最可能的情况：**GLM API 返回了错误或超时**

证据：
1. 消息 38 的 `inputTokens: 46638` 不算很大，不太可能是 token 限制
2. 耗时 660s 但最后几条消息间隔很短（5秒），说明不是超时
3. 最可能是 GLM API 返回了错误（如 rate limit、server error）

### 缺失的信息

**问题：错误详情没有被记录到用户可见的日志中**

建议添加：
1. 在 `messages.jsonl` 中记录完整错误信息
2. 保存最后一次 API 请求/响应用于调试
3. CLI 显示具体的失败原因

## 参考

- 问题会话 1：`~/.sage/sessions/a1524a75-7867-449d-a955-34d3ab30d8e9/`
- 问题会话 2：`~/.sage/sessions/69c9d4cf-6c80-447d-9aa3-21a9fba915a3/`
- 发现时间：2026-01-08
- 使用模型：glm-4.7
