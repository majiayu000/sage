# BUG-002: LLM 响应写入延迟（已排除 - 实为 BUG-001）

## 状态：已排除

**经进一步调查，此问题实际上是 BUG-001（无限工具调用循环）的另一次复现。**

消息最终被写入了（17 条 → 24 条），只是写入有延迟。

---

## 原始问题描述（保留供参考）

Agent 在处理用户请求时，UI 显示正在执行工具（耗时 207.8s），但 LLM 响应从未被写入 `messages.jsonl` 日志文件。

## 复现场景

**Session ID**: `69c9d4cf-6c80-447d-9aa3-21a9fba915a3`

**用户输入（第三次提问）：**
```
我希望你对比一下两个项目 就是 sage 有哪些 可以从openclaudecode学习的呢？
```

**UI 显示：**
```
AI Response:
我来帮你对比 sage 和 openclaudecode 这两个项目，找出 sage 可以从 openclaudecode
学习的地方。
首先让我探索一下这两个项目的结构和特点。

◐ Executing tools (207.8s)
```

**实际日志状态：**
- `messages.jsonl` 共 17 行
- 最后一行是 `user` 类型消息
- 没有任何 `assistant` 响应记录
- 没有 `llm_request` / `llm_response` / `tool_call` 记录

## 问题分析

### 与 BUG-001 的区别

| 维度 | BUG-001 | BUG-002 |
|------|---------|---------|
| 消息记录 | 有多条 assistant 记录 | 完全没有 assistant 记录 |
| 问题类型 | 工具循环，无文本输出 | 响应未持久化 |
| 根因 | LLM 行为问题 | 程序/IO 问题 |

### 可能原因

1. **流式响应处理异常**
   - 流式响应开始但未正确完成
   - 消息在流式传输中丢失

2. **写入时机问题**
   - 可能只在完整响应后才写入
   - 长时间运行的响应未触发写入

3. **异步写入失败**
   - 文件 IO 操作被阻塞
   - 写入任务被取消

4. **内存缓冲未刷新**
   - 响应在内存中但未刷新到磁盘
   - 程序异常退出导致数据丢失

## 时间线分析

```
07:29:30 - 第 16 条消息（assistant 完成 openClaudecode 分析）
07:33:13 - 第 17 条消息（user 提问对比两个项目）
07:33:?? - AI 开始响应（UI 可见）
07:36:?? - 检查时仍在执行（207.8s = ~3.5 分钟）
```

用户消息记录了，但 AI 响应完全没有记录。

## 需要调查的代码

### 会话写入逻辑

```rust
// 可能在 crates/sage-core/src/session/ 目录
// 检查消息写入的时机和条件
```

### 流式响应处理

```rust
// 检查流式响应是否正确触发消息持久化
// 是否只在响应完成后才写入
```

### 关键问题

1. 什么时候触发消息写入？
   - 每次工具调用后？
   - 每次 LLM 响应块后？
   - 只在完整响应后？

2. 长时间运行的响应如何处理？
   - 是否有超时机制？
   - 超时后消息是否保存？

## 建议修复方案

### 1. 增量写入

每次收到 LLM 响应块时立即写入：

```rust
// 伪代码
fn on_llm_response_chunk(&mut self, chunk: ResponseChunk) {
    // 立即追加到日志
    self.session_log.append_partial(chunk);
    self.session_log.flush();  // 确保写入磁盘
}
```

### 2. 定期刷新

设置定期刷新机制：

```rust
// 每 N 秒强制刷新一次
let flush_interval = Duration::from_secs(5);
```

### 3. 工具调用记录

在工具调用前后都记录：

```rust
fn execute_tool(&mut self, tool_call: ToolCall) -> ToolResult {
    // 记录工具调用开始
    self.log_tool_call_start(&tool_call);

    let result = tool_call.execute();

    // 记录工具调用结果
    self.log_tool_call_result(&result);

    result
}
```

### 4. 心跳/进度日志

对于长时间运行的任务，记录进度：

```rust
// 每 30 秒记录一次心跳
{"type": "heartbeat", "elapsed_secs": 30, "status": "executing_tool", "tool": "Bash"}
```

## 影响

- 用户无法恢复中断的会话
- 调试困难（无法追溯问题）
- 可能丢失重要的执行记录

## 优先级

**高** - 影响会话恢复和调试能力

## 状态

- [x] 问题确认
- [x] 初步分析
- [ ] 代码定位
- [ ] 方案设计
- [ ] 代码实现
- [ ] 测试验证

## 参考

- 问题会话：`~/.sage/sessions/69c9d4cf-6c80-447d-9aa3-21a9fba915a3/`
- 关联 Bug：BUG-001（工具循环问题）
- 发现时间：2026-01-08
- 使用模型：glm-4.7
