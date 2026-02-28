# Sage Agent 统一执行循环设计文档

## 基于 Claude Code 逆向分析的完整设计

**版本**: 1.0
**日期**: 2025-12-20
**作者**: AI Assistant
**参考**: OpenClaudeCode 反编译分析

---

## 目录

1. [概述](#1-概述)
2. [Claude Code 架构分析](#2-claude-code-架构分析)
3. [核心机制详解](#3-核心机制详解)
4. [Sage 实现设计](#4-sage-实现设计)
5. [数据结构定义](#5-数据结构定义)
6. [API 设计](#6-api-设计)
7. [实现步骤](#7-实现步骤)

---

## 1. 概述

### 1.1 当前问题

Sage Agent 当前采用 "exit-resume" 模式：
- 当需要用户输入时，循环退出并返回 `WaitingForInput` 状态
- 外部代码收集用户输入后，调用 `continue_execution()` 恢复执行
- 问题：模型只输出文本（不调用工具）时，循环继续空转

### 1.2 目标设计

采用 Claude Code 风格的 "blocking-wait" 模式：
- 执行循环在需要用户输入时 **阻塞等待**，而不是退出
- 通过 Channel 机制实现循环与 UI 层的通信
- 工具可以声明 `requires_user_interaction` 来触发阻塞

### 1.3 核心优势

| 方面 | Exit-Resume 模式 | Blocking-Wait 模式 |
|------|-----------------|-------------------|
| 状态管理 | 复杂，需要保存/恢复状态 | 简单，状态在循环中保持 |
| 代码结构 | 分散在多处 | 集中在循环内 |
| 错误处理 | 需要跨调用传递 | 自然的 try/catch |
| 用户体验 | 可能有延迟感 | 即时响应 |

---

## 2. Claude Code 架构分析

### 2.1 整体架构

```
┌─────────────────────────────────────────────────────────────┐
│                     Claude Code Architecture                 │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐     │
│  │  UI Layer   │◄───│   State     │◄───│  Execution  │     │
│  │  (React)    │    │   (Redux)   │    │    Loop     │     │
│  └──────┬──────┘    └──────┬──────┘    └──────┬──────┘     │
│         │                  │                  │             │
│         │    Callbacks     │   Async Gen      │             │
│         └──────────────────┴──────────────────┘             │
│                            │                                │
│                   ┌────────▼────────┐                       │
│                   │   Permission    │                       │
│                   │     System      │                       │
│                   └────────┬────────┘                       │
│                            │                                │
│         ┌──────────────────┼──────────────────┐             │
│         ▼                  ▼                  ▼             │
│  ┌────────────┐    ┌────────────┐    ┌────────────┐        │
│  │   Tools    │    │   Hooks    │    │   Config   │        │
│  │ (with req  │    │ (can mod   │    │  (rules)   │        │
│  │  user int) │    │  decision) │    │            │        │
│  └────────────┘    └────────────┘    └────────────┘        │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 执行循环核心代码

**位置**: `agents/agents_004.js` (lines 1306-1426)

```javascript
async function* runAgentAsync({
    agentDefinition: A,
    promptMessages: Q,
    toolUseContext: B,
    ...
}) {
    // 主消息处理循环
    for await (let message of J$({
        messages: H,
        systemPrompt: v,
        userContext: E,
        ...
    })) {
        if (message.type === "assistant" ||
            message.type === "user" ||
            message.type === "progress") {
            x.push(message);
            yield message;  // 返回给 UI 层
        }
    }
}
```

**关键特点**:
1. 使用 `async function*` (异步生成器)
2. `for await` 循环等待消息
3. `yield` 将消息返回给调用者

### 2.3 权限阻塞机制

**位置**: `tools/tools_018.js` (lines 1236-1396)

```javascript
function useToolPermissionHandler(A, Q) {
    return useCallback(async (B, G, Z, I, Y, J) => {
        // 创建阻塞 Promise
        return new Promise((W) => {
            // 检查权限
            const permResult = await A.checkPermissions(G, B);

            if (permResult.behavior === "ask") {
                // 添加到权限请求队列
                addToQueue({
                    tool: A,
                    input: G,
                    permissionResult: permResult,

                    // 这些回调会 resolve Promise
                    onAllow(modifiedInput, rules) {
                        W({ behavior: "allow", updatedInput: modifiedInput });
                    },
                    onReject(reason) {
                        W({ behavior: "deny", message: reason });
                    }
                });
                // Promise 阻塞在这里，直到 onAllow/onReject 被调用
            }
        });
    }, []);
}
```

### 2.4 AskUserQuestion 工具

**位置**: `tools/tools_017.js` (lines 1492-1525)

```javascript
const askUserQuestionTool = {
    name: "AskUserQuestion",

    // 关键: 声明需要用户交互
    requiresUserInteraction() {
        return true;
    },

    // 总是返回 "ask" 行为
    async checkPermissions(input) {
        return {
            behavior: "ask",
            message: "Answer questions?",
            updatedInput: input
        };
    },

    inputSchema: {
        questions: z.array(questionSchema).min(1).max(4),
        answers: z.record(z.string(), z.string()).optional()
    },

    outputSchema: {
        questions: z.array(questionSchema),
        answers: z.record(z.string(), z.string())
    },

    async call({ questions, answers = {} }) {
        return { data: { questions, answers } };
    },

    mapToolResultToToolResultBlockParam({ answers }, toolUseId) {
        return {
            type: "tool_result",
            tool_use_id: toolUseId,
            content: `User answered: ${formatAnswers(answers)}`
        };
    }
};
```

### 2.5 权限行为类型

| 行为 | 说明 | 触发条件 |
|------|------|---------|
| `allow` | 自动允许执行 | 配置规则、Hook 允许 |
| `deny` | 自动拒绝执行 | 配置规则禁止 |
| `ask` | 需要用户确认 | 工具需要交互、规则要求确认 |
| `passthrough` | 透传，不做检查 | 默认行为 |

---

## 3. 核心机制详解

### 3.1 Promise 阻塞模式

```
┌─────────────────────────────────────────────────────────────┐
│                   Promise Blocking Flow                      │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Execution Loop                    UI Layer                 │
│       │                               │                     │
│       │ 1. Tool needs permission      │                     │
│       ▼                               │                     │
│  ┌─────────────┐                      │                     │
│  │ new Promise │                      │                     │
│  │   (resolve) │                      │                     │
│  └──────┬──────┘                      │                     │
│         │                             │                     │
│         │ 2. Add to queue             │                     │
│         ▼                             │                     │
│  ┌─────────────┐                      │                     │
│  │   BLOCKED   │◄─────────────────────┤                     │
│  │   waiting   │                      │                     │
│  └──────┬──────┘                      │                     │
│         │                             │                     │
│         │            3. User interacts│                     │
│         │                             ▼                     │
│         │                      ┌─────────────┐              │
│         │                      │ onAllow() / │              │
│         │                      │ onReject()  │              │
│         │                      └──────┬──────┘              │
│         │                             │                     │
│         │◄────────────────────────────┘                     │
│         │ 4. Promise resolves                               │
│         ▼                                                   │
│  ┌─────────────┐                                            │
│  │  RESUMED    │                                            │
│  │  execution  │                                            │
│  └─────────────┘                                            │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 权限检查流程

```
┌─────────────────────────────────────────────────────────────┐
│                 Permission Check Pipeline                    │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Tool Execution Request                                     │
│         │                                                   │
│         ▼                                                   │
│  ┌─────────────────┐                                        │
│  │ 1. Deny Rules   │──deny──► Return Deny                   │
│  └────────┬────────┘                                        │
│           │ pass                                            │
│           ▼                                                 │
│  ┌─────────────────┐                                        │
│  │ 2. Ask Rules    │──ask───► Queue for Confirmation        │
│  └────────┬────────┘                                        │
│           │ pass                                            │
│           ▼                                                 │
│  ┌─────────────────┐                                        │
│  │ 3. Tool.check   │                                        │
│  │   Permissions() │──ask───► Queue for Confirmation        │
│  └────────┬────────┘                                        │
│           │ pass/allow                                      │
│           ▼                                                 │
│  ┌─────────────────┐                                        │
│  │ 4. requiresUser │                                        │
│  │   Interaction?  │──yes + ask──► Queue for Confirmation   │
│  └────────┬────────┘                                        │
│           │ no                                              │
│           ▼                                                 │
│  ┌─────────────────┐                                        │
│  │ 5. Bypass Mode? │──yes──► Return Allow                   │
│  └────────┬────────┘                                        │
│           │ no                                              │
│           ▼                                                 │
│  ┌─────────────────┐                                        │
│  │ 6. Allow Rules  │──allow──► Return Allow                 │
│  └────────┬────────┘                                        │
│           │ none                                            │
│           ▼                                                 │
│     Return Ask (Default)                                    │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 3.3 用户输入收集流程

```
┌─────────────────────────────────────────────────────────────┐
│              AskUserQuestion Data Flow                       │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  1. Claude calls AskUserQuestion tool                       │
│     └──► input: { questions: [...] }                        │
│                    │                                        │
│                    ▼                                        │
│  2. checkPermissions() returns { behavior: "ask" }          │
│                    │                                        │
│                    ▼                                        │
│  3. Permission Queue receives request                       │
│     └──► UI renders question form                           │
│                    │                                        │
│                    ▼                                        │
│  4. User answers questions                                  │
│     └──► state: { answers: { "Q1": "A1", ... } }           │
│                    │                                        │
│                    ▼                                        │
│  5. User clicks "Submit"                                    │
│     └──► onAllow(modifiedInput, [])                        │
│          modifiedInput = { questions, answers }             │
│                    │                                        │
│                    ▼                                        │
│  6. Tool.call() executes with answers                       │
│     └──► return { data: { questions, answers } }           │
│                    │                                        │
│                    ▼                                        │
│  7. mapToolResultToToolResultBlockParam()                   │
│     └──► { type: "tool_result", content: "User answered..." }
│                    │                                        │
│                    ▼                                        │
│  8. Claude sees result, continues execution                 │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 4. Sage 实现设计

### 4.1 架构概览

```
┌─────────────────────────────────────────────────────────────┐
│                    Sage Architecture                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────────────────────────────────────────────┐   │
│  │                    CLI Layer                          │   │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐      │   │
│  │  │Interactive │  │    Run     │  │   Unified  │      │   │
│  │  │   Mode     │  │   Mode     │  │   Command  │      │   │
│  │  └─────┬──────┘  └─────┬──────┘  └─────┬──────┘      │   │
│  │        └───────────────┴───────────────┘              │   │
│  │                        │                              │   │
│  └────────────────────────┼──────────────────────────────┘   │
│                           │                                  │
│                           ▼                                  │
│  ┌──────────────────────────────────────────────────────┐   │
│  │                  InputChannel                         │   │
│  │  ┌─────────────┐              ┌─────────────┐        │   │
│  │  │   Request   │◄────────────►│   Response  │        │   │
│  │  │   Channel   │              │   Channel   │        │   │
│  │  │  (tx → rx)  │              │  (tx → rx)  │        │   │
│  │  └─────────────┘              └─────────────┘        │   │
│  └──────────────────────────────────────────────────────┘   │
│                           │                                  │
│                           ▼                                  │
│  ┌──────────────────────────────────────────────────────┐   │
│  │                 Unified Executor                      │   │
│  │  ┌─────────────────────────────────────────────┐     │   │
│  │  │              Execution Loop                  │     │   │
│  │  │                                              │     │   │
│  │  │  while !completed && step < max_steps {      │     │   │
│  │  │      response = llm.call(messages);          │     │   │
│  │  │                                              │     │   │
│  │  │      if response.needs_user_input() {        │     │   │
│  │  │          // BLOCK waiting for input          │     │   │
│  │  │          input = channel.request_input();    │     │   │
│  │  │          messages.push(user_msg(input));     │     │   │
│  │  │      }                                       │     │   │
│  │  │                                              │     │   │
│  │  │      for tool_call in response.tool_calls { │     │   │
│  │  │          if tool.requires_user_interaction() │     │   │
│  │  │              // BLOCK waiting for permission │     │   │
│  │  │              perm = channel.request_perm();  │     │   │
│  │  │          }                                   │     │   │
│  │  │          result = tool.execute(tool_call);   │     │   │
│  │  │      }                                       │     │   │
│  │  │  }                                           │     │   │
│  │  └─────────────────────────────────────────────┘     │   │
│  └──────────────────────────────────────────────────────┘   │
│                           │                                  │
│                           ▼                                  │
│  ┌──────────────────────────────────────────────────────┐   │
│  │                    Tool System                        │   │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐      │   │
│  │  │AskUserQues │  │   Bash     │  │   Edit     │      │   │
│  │  │   tion     │  │            │  │            │      │   │
│  │  │[req_inter] │  │            │  │            │      │   │
│  │  └────────────┘  └────────────┘  └────────────┘      │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 4.2 核心组件

#### 4.2.1 InputChannel

```rust
/// 用于执行循环与 UI 层之间的双向通信
pub struct InputChannel {
    /// 发送输入请求
    request_tx: mpsc::Sender<InputRequest>,
    /// 接收用户响应
    response_rx: mpsc::Receiver<InputResponse>,
    /// 默认超时时间
    default_timeout: Option<Duration>,
    /// 自动响应器（非交互模式）
    auto_responder: Option<Box<dyn Fn(&InputRequest) -> InputResponse + Send + Sync>>,
}

/// 用户侧的通道句柄
pub struct InputChannelHandle {
    /// 接收输入请求
    pub request_rx: mpsc::Receiver<InputRequest>,
    /// 发送用户响应
    pub response_tx: mpsc::Sender<InputResponse>,
}
```

#### 4.2.2 InputRequest

```rust
#[derive(Debug, Clone)]
pub enum InputRequest {
    /// 需要用户回答问题
    UserQuestion {
        id: String,
        questions: Vec<Question>,
    },
    /// 需要用户确认权限
    PermissionRequest {
        id: String,
        tool_name: String,
        description: String,
        input: serde_json::Value,
    },
    /// 需要自由文本输入（模型只输出文本时）
    FreeTextInput {
        id: String,
        prompt: String,
        last_response: String,
    },
}
```

#### 4.2.3 InputResponse

```rust
#[derive(Debug, Clone)]
pub enum InputResponse {
    /// 用户回答了问题
    QuestionAnswers {
        request_id: String,
        answers: HashMap<String, String>,
    },
    /// 用户授权了权限
    PermissionGranted {
        request_id: String,
        modified_input: Option<serde_json::Value>,
    },
    /// 用户拒绝了权限
    PermissionDenied {
        request_id: String,
        reason: Option<String>,
    },
    /// 用户提供了自由文本输入
    FreeText {
        request_id: String,
        text: String,
    },
    /// 用户取消了操作
    Cancelled {
        request_id: String,
    },
}
```

### 4.3 工具 Trait 扩展

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    // ... 现有方法 ...

    /// 是否需要用户交互
    /// 返回 true 时，工具执行会阻塞等待用户输入
    fn requires_user_interaction(&self) -> bool {
        false  // 默认不需要
    }

    /// 检查权限
    /// 返回 PermissionBehavior 决定如何处理
    async fn check_permissions(
        &self,
        input: &serde_json::Value,
        context: &ToolContext,
    ) -> PermissionResult {
        PermissionResult::Allow  // 默认允许
    }
}

#[derive(Debug, Clone)]
pub enum PermissionResult {
    /// 自动允许
    Allow,
    /// 自动拒绝
    Deny { message: String },
    /// 需要用户确认
    Ask { message: String, suggestions: Vec<PermissionSuggestion> },
}
```

### 4.4 执行选项

```rust
#[derive(Debug, Clone)]
pub struct ExecutionOptions {
    /// 执行模式
    pub mode: ExecutionMode,
    /// 最大步数
    pub max_steps: u32,
    /// 执行超时
    pub execution_timeout: Option<Duration>,
    /// 用户输入超时
    pub prompt_timeout: Option<Duration>,
    /// 是否记录轨迹
    pub record_trajectory: bool,
}

#[derive(Debug, Clone)]
pub enum ExecutionMode {
    /// 交互模式 - 阻塞等待用户输入
    Interactive,
    /// 非交互模式 - 使用自动响应
    NonInteractive { auto_response: AutoResponse },
    /// 批处理模式 - 不允许用户交互
    Batch,
}

#[derive(Debug, Clone)]
pub enum AutoResponse {
    /// 默认响应
    Default,
    /// 总是允许
    AlwaysAllow,
    /// 总是拒绝
    AlwaysDeny,
    /// 自定义响应函数
    Custom(Arc<dyn Fn(&InputRequest) -> InputResponse + Send + Sync>),
}
```

---

## 5. 数据结构定义

### 5.1 Question 结构

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    /// 问题文本
    pub question: String,
    /// 简短标签 (最多 12 字符)
    pub header: String,
    /// 选项列表 (2-4 个)
    pub options: Vec<QuestionOption>,
    /// 是否允许多选
    #[serde(default)]
    pub multi_select: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionOption {
    /// 选项标签
    pub label: String,
    /// 选项描述
    pub description: String,
}
```

### 5.2 权限建议

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionSuggestion {
    /// 建议类型
    pub suggestion_type: SuggestionType,
    /// 工具名称
    pub tool_name: String,
    /// 规则内容
    pub rule_content: String,
    /// 权限行为
    pub behavior: PermissionBehavior,
    /// 保存位置
    pub destination: RuleDestination,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestionType {
    AddRule,
    RemoveRule,
    ModifyRule,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleDestination {
    Session,
    LocalSettings,
    UserSettings,
    ProjectSettings,
}
```

---

## 6. API 设计

### 6.1 InputChannel API

```rust
impl InputChannel {
    /// 创建新的 InputChannel
    pub fn new(buffer_size: usize) -> (Self, InputChannelHandle);

    /// 创建非交互模式的 Channel
    pub fn non_interactive(auto_response: AutoResponse) -> Self;

    /// 请求用户输入（阻塞）
    pub async fn request_input(&self, request: InputRequest) -> SageResult<InputResponse>;

    /// 请求用户输入（带超时）
    pub async fn request_input_with_timeout(
        &self,
        request: InputRequest,
        timeout: Duration,
    ) -> SageResult<InputResponse>;
}

impl InputChannelHandle {
    /// 获取下一个输入请求
    pub async fn next_request(&mut self) -> Option<InputRequest>;

    /// 发送响应
    pub async fn respond(&self, response: InputResponse) -> SageResult<()>;
}
```

### 6.2 UnifiedExecutor API

```rust
impl UnifiedExecutor {
    /// 创建执行器
    pub fn new(
        config: Config,
        options: ExecutionOptions,
        input_channel: Option<InputChannel>,
    ) -> SageResult<Self>;

    /// 执行任务
    pub async fn execute_task(
        &mut self,
        task: TaskMetadata,
    ) -> SageResult<ExecutionOutcome>;

    /// 执行单个步骤
    async fn execute_step(&mut self) -> SageResult<AgentStep>;

    /// 处理用户问题（阻塞）
    async fn handle_user_question(
        &self,
        questions: Vec<Question>,
    ) -> SageResult<HashMap<String, String>>;

    /// 处理权限请求（阻塞）
    async fn handle_permission_request(
        &self,
        tool_name: &str,
        input: &serde_json::Value,
    ) -> SageResult<PermissionResult>;
}
```

### 6.3 SDK API 更新

```rust
impl SageAgentSDK {
    /// 使用 InputChannel 执行任务
    pub async fn execute_with_channel(
        &self,
        task: &str,
        options: ExecutionOptions,
    ) -> SageResult<(ExecutionResult, InputChannelHandle)>;

    /// 简单执行（自动处理输入）
    pub async fn run(&self, task: &str) -> SageResult<ExecutionResult>;

    /// 带选项执行
    pub async fn run_with_options(
        &self,
        task: &str,
        options: RunOptions,
    ) -> SageResult<ExecutionResult>;
}
```

---

## 7. 实现步骤

### Phase 1: 核心基础设施

1. **创建 `input/mod.rs`**
   - 定义 `InputChannel` 和 `InputChannelHandle`
   - 实现 `InputRequest` 和 `InputResponse` 枚举
   - 实现阻塞等待和超时机制

2. **创建 `agent/options.rs`**
   - 定义 `ExecutionOptions` 和 `ExecutionMode`
   - 实现 `AutoResponse` 枚举

3. **更新 `outcome.rs`**
   - 移除 `WaitingForInput` (如果存在)
   - 确保 `NeedsUserInput` 正确实现

### Phase 2: 统一执行器

4. **创建 `agent/unified.rs`**
   - 实现 `UnifiedExecutor` 结构
   - 实现阻塞式执行循环
   - 集成 `InputChannel`

5. **更新 `tools/base.rs`**
   - 添加 `requires_user_interaction()` 方法
   - 添加 `check_permissions()` 方法

6. **更新 `ask_user.rs`**
   - 实现 `requires_user_interaction()` 返回 `true`
   - 实现正确的 `check_permissions()`
   - 更新 `execute()` 使用 `InputChannel`

### Phase 3: CLI 整合

7. **创建 `commands/unified.rs`**
   - 合并 `run` 和 `interactive` 命令逻辑
   - 实现用户输入处理循环
   - 处理 `InputChannelHandle`

8. **更新 `main.rs`**
   - 路由到统一命令

9. **更新 signal handler**
   - 处理中断时的 Channel 清理

### Phase 4: SDK 更新

10. **更新 `client.rs`**
    - 添加 `execute_with_channel()` 方法
    - 更新现有 API

11. **更新 examples**
    - 添加交互模式示例

12. **添加集成测试**
    - 测试阻塞/恢复流程
    - 测试非交互模式

### Phase 5: 清理

13. **删除旧代码**
    - 移除 `WaitingForInput` 相关代码
    - 清理 `ConversationSession` (如果不再需要)

14. **更新文档**
    - API 文档
    - 用户指南

---

## 附录 A: Claude Code 关键代码位置

| 组件 | 文件 | 行号 | 说明 |
|------|------|------|------|
| 主执行循环 | agents_004.js | 1306-1426 | async generator loop |
| 权限检查 | tools_022.js | 1483-1557 | permission pipeline |
| 阻塞点 | tools_022.js | 1517 | requiresUserInteraction check |
| AskUserQuestion | tools_017.js | 1492-1525 | tool definition |
| 权限处理器 | tools_018.js | 1236-1396 | Promise-based blocking |
| UI 组件 | mcp_016.js | 1-170 | React components |
| 状态管理 | mcp_016.js | 72-158 | useReducer pattern |

---

## 附录 B: 对比表

| 特性 | Claude Code | Sage (当前) | Sage (目标) |
|------|-------------|-------------|-------------|
| 执行模式 | Blocking-wait | Exit-resume | Blocking-wait |
| 通信机制 | Promise + Callback | ExecutionOutcome | Channel |
| UI 集成 | React state | CLI prompt | Channel handle |
| 非交互模式 | Auto-responder | Config | AutoResponse |
| 权限系统 | 完整 | 部分 | 完整 |
| 工具交互标记 | requiresUserInteraction | 无 | requires_user_interaction |

---

## 附录 C: 测试用例

### C.1 交互模式测试

```rust
#[tokio::test]
async fn test_interactive_question_flow() {
    let (channel, handle) = InputChannel::new(10);
    let executor = UnifiedExecutor::new(config, options, Some(channel))?;

    // 启动执行器
    let exec_handle = tokio::spawn(async move {
        executor.execute_task(task).await
    });

    // 模拟用户响应
    if let Some(request) = handle.next_request().await {
        match request {
            InputRequest::UserQuestion { id, questions } => {
                handle.respond(InputResponse::QuestionAnswers {
                    request_id: id,
                    answers: [("Q1".into(), "Answer1".into())].into(),
                }).await?;
            }
            _ => panic!("Unexpected request type"),
        }
    }

    let result = exec_handle.await??;
    assert!(result.is_success());
}
```

### C.2 非交互模式测试

```rust
#[tokio::test]
async fn test_non_interactive_auto_response() {
    let channel = InputChannel::non_interactive(AutoResponse::AlwaysAllow);
    let executor = UnifiedExecutor::new(config, options, Some(channel))?;

    let result = executor.execute_task(task).await?;
    assert!(result.is_success());
}
```

### C.3 超时测试

```rust
#[tokio::test]
async fn test_input_timeout() {
    let options = ExecutionOptions {
        prompt_timeout: Some(Duration::from_millis(100)),
        ..Default::default()
    };

    let (channel, _handle) = InputChannel::new(10);
    let executor = UnifiedExecutor::new(config, options, Some(channel))?;

    let result = executor.execute_task(task).await;
    assert!(matches!(result, Err(SageError::Timeout { .. })));
}
```

---

*文档结束*
