# Sage Agent 对话模式改进

## 问题描述

之前的交互模式存在以下问题：
1. **每次输入都创建新任务**：每句话都会触发 `execute_task` 函数，创建全新的任务执行上下文
2. **没有对话状态保持**：Agent 无法记住之前的对话内容和上下文
3. **无法进行迭代修改**：用户无法对之前的回答进行修正或补充
4. **轨迹文件独立**：每个任务都生成独立的轨迹文件，缺乏连续性

## 解决方案

### 1. 新增对话会话管理器 (`ConversationSession`)

```rust
struct ConversationSession {
    /// 当前对话消息历史
    messages: Vec<LLMMessage>,
    /// 当前任务元数据
    task: Option<TaskMetadata>,
    /// 当前 Agent 执行状态
    execution: Option<AgentExecution>,
    /// 会话元数据
    metadata: HashMap<String, serde_json::Value>,
}
```

**功能**：
- `add_user_message()` - 添加用户消息到对话历史
- `add_assistant_message()` - 添加 AI 回复到对话历史
- `is_new_conversation()` - 检查是否为新对话
- `reset()` - 重置对话会话
- `get_summary()` - 获取对话摘要

### 2. 修改交互循环逻辑

**之前**：
```rust
_ => {
    // 每次都创建新任务
    match execute_task(&console, &sdk, input).await {
        // ...
    }
}
```

**现在**：
```rust
_ => {
    // 使用对话模式处理
    match handle_conversation(&console, &sdk, &mut conversation, input).await {
        // ...
    }
}
```

### 3. 新增对话处理函数

#### `handle_conversation()`
- 判断是新对话还是继续对话
- 管理对话状态和消息历史
- 调用相应的执行函数

#### `execute_conversation_task()`
- 处理新对话的第一条消息
- 创建任务元数据并执行

#### `execute_conversation_continuation()`
- 处理对话的后续消息
- 包含完整的对话上下文
- 保持对话的连续性

### 4. 新增交互命令

- `new` / `new-task` - 开始新对话（清除之前的上下文）
- `conversation` / `conv` - 显示当前对话摘要

### 5. 更新帮助信息

新的帮助信息明确说明了对话模式的工作方式：

```
🗣️  Conversation Mode:
Any other input will be treated as part of an ongoing conversation.
The AI will remember previous messages and context within the same conversation.
Use 'new' to start fresh if you want to change topics completely.

Example conversation:
  You: Create a hello world Python script
  AI: [Creates the script]
  You: Now add error handling to it
  AI: [Modifies the existing script with error handling]
```

## 使用示例

### 场景 1：连续对话
```
sage: Create a Python function to calculate fibonacci numbers
AI: [创建 fibonacci 函数]

sage: Add memoization to make it more efficient
AI: [修改函数添加记忆化]

sage: Now write unit tests for this function
AI: [为函数编写测试]
```

### 场景 2：开始新话题
```
sage: Create a Python function to calculate fibonacci numbers
AI: [创建 fibonacci 函数]

sage: new
✓ Started new conversation. Previous context cleared.

sage: Help me set up a React project
AI: [开始全新的 React 项目设置任务]
```

## 技术细节

### 对话上下文管理
- 使用 `Vec<LLMMessage>` 存储完整的对话历史
- 每次 AI 回复后自动添加到消息历史
- 支持 system、user、assistant 消息类型

### 真正的对话延续机制
- **新对话**：创建新的 `AgentExecution` 并执行任务
- **继续对话**：使用 `Agent::continue_execution()` 方法在现有执行上下文中添加新的用户消息
- **核心改进**：不再每次都创建新任务，而是在同一个执行上下文中继续对话

### Agent 层面的改进
- 新增 `Agent::continue_execution()` trait 方法
- 在 `BaseAgent` 中实现真正的对话延续逻辑
- 保持完整的执行历史和上下文

### SDK 层面的支持
- 新增 `SageAgentSDK::continue_execution()` 方法
- 支持在现有 `AgentExecution` 上继续执行

### 错误处理
- 保持原有的错误处理机制
- 对话失败不会影响会话状态
- 支持重试和错误恢复

## 优势

1. **自然对话体验**：用户可以像聊天一样与 AI 交互
2. **上下文保持**：AI 能记住之前的对话内容
3. **迭代改进**：支持对之前的结果进行修改和完善
4. **灵活控制**：用户可以选择继续对话或开始新话题
5. **向后兼容**：保持原有的所有功能和命令

## 文件修改

### 核心 Agent 层面
- `crates/sage-core/src/agent/base.rs`
  - 新增 `Agent::continue_execution()` trait 方法
  - 在 `BaseAgent` 中实现真正的对话延续逻辑
  - 支持在现有执行上下文中添加新用户消息

### SDK 层面
- `crates/sage-sdk/src/client.rs`
  - 新增 `SageAgentSDK::continue_execution()` 方法
  - 支持对话延续而不是重新创建任务

### CLI 交互层面
- `crates/sage-cli/src/commands/interactive.rs`
  - 新增 `ConversationSession` 结构体
  - 修改主交互循环
  - 重写对话处理函数使用真正的延续机制
  - 更新帮助信息
  - 清理未使用的代码

## 测试建议

1. 测试基本对话功能
2. 测试 `new` 命令重置功能
3. 测试错误处理和恢复
4. 测试长对话的性能
5. 测试轨迹文件生成
