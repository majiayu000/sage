# Sage 工具系统架构设计 v2.0

## 1. 设计目标

### 1.1 核心目标
- **完全兼容 Claude Code**: 实现所有 26 个 Claude Code 工具
- **高度解耦**: 工具间无直接依赖，通过接口通信
- **可扩展**: 支持插件式工具注册和动态加载
- **类型安全**: 利用 Rust 类型系统确保正确性
- **可测试**: 每个组件可独立测试

### 1.2 设计原则
1. **单一职责**: 每个工具只做一件事
2. **依赖倒置**: 依赖抽象而非具体实现
3. **开闭原则**: 对扩展开放，对修改关闭
4. **组合优于继承**: 使用 trait 组合而非继承层次

## 2. 架构概览

```
┌─────────────────────────────────────────────────────────────────┐
│                        Tool System                               │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
│  │ Tool Traits │  │ Tool Schema │  │ Tool Result │              │
│  │  (core)     │  │  (types)    │  │  (types)    │              │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘              │
│         │                │                │                      │
│         └────────────────┼────────────────┘                      │
│                          ▼                                       │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                    Tool Registry                           │  │
│  │  - register(tool)                                          │  │
│  │  - get(name) -> Option<Arc<dyn Tool>>                     │  │
│  │  - list_by_category(category) -> Vec<ToolInfo>            │  │
│  └───────────────────────────────────────────────────────────┘  │
│                          │                                       │
│         ┌────────────────┼────────────────┐                      │
│         ▼                ▼                ▼                      │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
│  │  Executor   │  │ Permission  │  │  Context    │              │
│  │  (parallel) │  │  Handler    │  │  Provider   │              │
│  └─────────────┘  └─────────────┘  └─────────────┘              │
├─────────────────────────────────────────────────────────────────┤
│                      Tool Categories                             │
├─────────────┬─────────────┬─────────────┬───────────────────────┤
│ File Ops    │ Process     │ Task Mgmt   │ Planning              │
│ - Read      │ - Bash      │ - TaskCreate│ - EnterPlanMode       │
│ - Write     │ - Task      │ - TaskUpdate│ - ExitPlanMode        │
│ - Edit      │ - TaskOutput│ - TaskList  │                       │
│ - Glob      │ - TaskStop  │ - TaskGet   │                       │
│ - Grep      │             │ - TodoWrite │                       │
│ - Notebook  │             │             │                       │
├─────────────┼─────────────┼─────────────┼───────────────────────┤
│ Interaction │ Network     │ Extensions  │ Code Intelligence     │
│ - AskUser   │ - WebFetch  │ - Skill     │ - LSP                 │
│             │ - WebSearch │ - ToolSearch│                       │
├─────────────┼─────────────┼─────────────┼───────────────────────┤
│ Browser     │ Team        │             │                       │
│ - Computer  │ - Teammate  │             │                       │
│             │ - SendMsg   │             │                       │
└─────────────┴─────────────┴─────────────┴───────────────────────┘
```

## 3. 核心 Trait 设计

### 3.1 基础 Tool Trait (保持现有设计)

```rust
// crates/sage-core/src/tools/base/tool_trait.rs

#[async_trait]
pub trait Tool: Send + Sync {
    /// 工具唯一标识符
    fn name(&self) -> &str;

    /// LLM 可读的描述
    fn description(&self) -> &str;

    /// JSON Schema 定义
    fn schema(&self) -> ToolSchema;

    /// 执行工具
    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError>;
}
```

### 3.2 新增: 工具元数据 Trait

```rust
// crates/sage-core/src/tools/base/metadata.rs

/// 工具元数据，用于文档生成和 UI 显示
pub trait ToolMetadata: Tool {
    /// 工具版本 (语义化版本)
    fn version(&self) -> &str { "1.0.0" }

    /// 工具分类
    fn category(&self) -> ToolCategory;

    /// 详细的使用说明 (支持变量插值)
    fn usage_prompt(&self) -> &str { self.description() }

    /// 示例用法
    fn examples(&self) -> Vec<ToolExample> { vec![] }

    /// 相关工具 (用于推荐)
    fn related_tools(&self) -> Vec<&str> { vec![] }

    /// 是否为 Claude Code 兼容工具
    fn is_claude_code_compatible(&self) -> bool { false }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolCategory {
    FileOperations,
    ProcessExecution,
    TaskManagement,
    Planning,
    Interaction,
    Network,
    Extensions,
    CodeIntelligence,
    BrowserAutomation,
    TeamCollaboration,
    Custom(u32),
}

pub struct ToolExample {
    pub description: String,
    pub input: serde_json::Value,
    pub expected_output: Option<String>,
}
```

### 3.3 新增: 工具上下文 Trait

```rust
// crates/sage-core/src/tools/base/context.rs

/// 工具执行上下文，提供共享资源访问
pub trait ToolContext: Send + Sync {
    /// 获取工作目录
    fn working_directory(&self) -> &Path;

    /// 获取会话 ID
    fn session_id(&self) -> Option<&str>;

    /// 获取用户配置
    fn config(&self) -> &ToolConfig;

    /// 获取其他工具的引用 (用于工具组合)
    fn get_tool(&self, name: &str) -> Option<Arc<dyn Tool>>;

    /// 发送事件
    fn emit_event(&self, event: ToolEvent);
}

/// 工具事件，用于监控和日志
#[derive(Debug, Clone)]
pub enum ToolEvent {
    Started { tool: String, call_id: String },
    Progress { tool: String, call_id: String, message: String },
    Completed { tool: String, call_id: String, duration_ms: u64 },
    Failed { tool: String, call_id: String, error: String },
}
```

### 3.4 新增: 工具提供者 Trait

```rust
// crates/sage-core/src/tools/base/provider.rs

/// 工具提供者，用于动态工具注册
pub trait ToolProvider: Send + Sync {
    /// 提供者名称
    fn name(&self) -> &str;

    /// 获取所有工具
    fn tools(&self) -> Vec<Arc<dyn Tool>>;

    /// 是否支持延迟加载
    fn supports_lazy_loading(&self) -> bool { false }

    /// 按需加载工具
    fn load_tool(&self, name: &str) -> Option<Arc<dyn Tool>> { None }
}

/// 内置工具提供者
pub struct BuiltinToolProvider;

/// MCP 工具提供者
pub struct McpToolProvider {
    servers: Vec<McpServerConfig>,
}

/// 自定义工具提供者
pub struct CustomToolProvider {
    tools: Vec<Arc<dyn Tool>>,
}
```

## 4. 工具注册表重构

```rust
// crates/sage-core/src/tools/registry.rs

pub struct ToolRegistry {
    /// 已注册的工具
    tools: DashMap<String, Arc<dyn Tool>>,

    /// 工具提供者
    providers: RwLock<Vec<Arc<dyn ToolProvider>>>,

    /// 分类索引
    category_index: DashMap<ToolCategory, Vec<String>>,

    /// 延迟加载的工具名称
    deferred_tools: DashSet<String>,

    /// 工具别名
    aliases: DashMap<String, String>,
}

impl ToolRegistry {
    /// 注册工具提供者
    pub fn register_provider(&self, provider: Arc<dyn ToolProvider>) {
        for tool in provider.tools() {
            self.register_tool(tool);
        }
        self.providers.write().push(provider);
    }

    /// 注册单个工具
    pub fn register_tool(&self, tool: Arc<dyn Tool>) {
        let name = tool.name().to_string();

        // 更新分类索引
        if let Some(metadata) = tool.as_any().downcast_ref::<dyn ToolMetadata>() {
            self.category_index
                .entry(metadata.category())
                .or_default()
                .push(name.clone());
        }

        self.tools.insert(name, tool);
    }

    /// 获取工具 (支持延迟加载)
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        // 先检查别名
        let actual_name = self.aliases.get(name)
            .map(|r| r.value().clone())
            .unwrap_or_else(|| name.to_string());

        // 尝试直接获取
        if let Some(tool) = self.tools.get(&actual_name) {
            return Some(tool.clone());
        }

        // 尝试延迟加载
        if self.deferred_tools.contains(&actual_name) {
            for provider in self.providers.read().iter() {
                if let Some(tool) = provider.load_tool(&actual_name) {
                    self.tools.insert(actual_name.clone(), tool.clone());
                    self.deferred_tools.remove(&actual_name);
                    return Some(tool);
                }
            }
        }

        None
    }

    /// 搜索工具 (用于 ToolSearch)
    pub fn search(&self, query: &str) -> Vec<ToolSearchResult> {
        // 实现模糊搜索
    }

    /// 按分类获取工具
    pub fn by_category(&self, category: ToolCategory) -> Vec<Arc<dyn Tool>> {
        self.category_index
            .get(&category)
            .map(|names| {
                names.iter()
                    .filter_map(|name| self.tools.get(name).map(|t| t.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }
}
```

## 5. 工具实现模板

### 5.1 标准工具模板

```rust
// crates/sage-tools/src/tools/template.rs

/// 工具实现宏，减少样板代码
#[macro_export]
macro_rules! define_tool {
    (
        name: $name:expr,
        description: $desc:expr,
        category: $category:expr,
        version: $version:expr,
        parameters: [$($param:expr),* $(,)?],
        execute: $execute:expr $(,)?
    ) => {
        // 生成工具结构体和实现
    };
}

// 使用示例
define_tool! {
    name: "Read",
    description: "Reads a file from the local filesystem",
    category: ToolCategory::FileOperations,
    version: "2.0.14",
    parameters: [
        param!("file_path", String, required, "The absolute path to the file"),
        param!("offset", i64, optional, "Line number to start reading from"),
        param!("limit", i64, optional, "Number of lines to read"),
    ],
    execute: |call, ctx| async move {
        // 实现逻辑
    },
}
```

### 5.2 工具基类

```rust
// crates/sage-tools/src/tools/base_tool.rs

/// 通用工具基类，提供常用功能
pub struct BaseTool<F>
where
    F: Fn(&ToolCall, &dyn ToolContext) -> BoxFuture<'static, Result<ToolResult, ToolError>>
        + Send + Sync + 'static,
{
    name: String,
    description: String,
    schema: ToolSchema,
    category: ToolCategory,
    version: String,
    execute_fn: F,
    usage_prompt: String,
    examples: Vec<ToolExample>,
    related_tools: Vec<String>,
}

impl<F> BaseTool<F> {
    pub fn builder(name: impl Into<String>) -> BaseToolBuilder<F> {
        BaseToolBuilder::new(name)
    }
}
```

## 6. 缺失工具实现计划

### 6.1 ToolSearch (延迟加载工具搜索)

```rust
// crates/sage-tools/src/tools/extensions/tool_search.rs

pub struct ToolSearchTool {
    registry: Arc<ToolRegistry>,
}

impl Tool for ToolSearchTool {
    fn name(&self) -> &str { "ToolSearch" }

    fn description(&self) -> &str {
        "Search for or select deferred tools to make them available for use."
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let query = call.require_string("query")?;

        if query.starts_with("select:") {
            // 直接选择模式
            let tool_name = query.strip_prefix("select:").unwrap();
            self.load_tool(tool_name)
        } else if query.starts_with("+") {
            // 必需关键词模式
            let required = query.strip_prefix("+").unwrap();
            self.search_with_required(required)
        } else {
            // 关键词搜索模式
            self.search_keywords(&query)
        }
    }
}
```

### 6.2 LSP (Language Server Protocol)

```rust
// crates/sage-tools/src/tools/code_intelligence/lsp.rs

pub struct LspTool {
    servers: DashMap<String, LspClient>,
    config: LspConfig,
}

impl Tool for LspTool {
    fn name(&self) -> &str { "LSP" }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let operation = call.require_string("operation")?;
        let file_path = call.require_string("filePath")?;
        let line = call.require_i64("line")? as u32;
        let character = call.require_i64("character")? as u32;

        match operation.as_str() {
            "goToDefinition" => self.go_to_definition(file_path, line, character).await,
            "findReferences" => self.find_references(file_path, line, character).await,
            "hover" => self.hover(file_path, line, character).await,
            "documentSymbol" => self.document_symbol(file_path).await,
            "workspaceSymbol" => {
                let query = call.get_string("query").unwrap_or_default();
                self.workspace_symbol(&query).await
            }
            "goToImplementation" => self.go_to_implementation(file_path, line, character).await,
            "prepareCallHierarchy" => self.prepare_call_hierarchy(file_path, line, character).await,
            "incomingCalls" => self.incoming_calls(file_path, line, character).await,
            "outgoingCalls" => self.outgoing_calls(file_path, line, character).await,
            _ => Err(ToolError::InvalidArguments(format!("Unknown operation: {}", operation))),
        }
    }
}
```

### 6.3 Computer (浏览器自动化)

```rust
// crates/sage-tools/src/tools/browser/computer.rs

pub struct ComputerTool {
    browser: Arc<Mutex<Option<Browser>>>,
    config: ComputerConfig,
}

impl Tool for ComputerTool {
    fn name(&self) -> &str { "Computer" }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let action = call.require_string("action")?;

        match action.as_str() {
            "screenshot" => self.take_screenshot(call).await,
            "click" => self.click(call).await,
            "type" => self.type_text(call).await,
            "scroll" => self.scroll(call).await,
            "key" => self.press_key(call).await,
            "move" => self.move_mouse(call).await,
            "drag" => self.drag(call).await,
            _ => Err(ToolError::InvalidArguments(format!("Unknown action: {}", action))),
        }
    }
}
```

### 6.4 TeammateTool (团队协作)

```rust
// crates/sage-tools/src/tools/team/teammate.rs

pub struct TeammateTool {
    team_manager: Arc<TeamManager>,
}

impl Tool for TeammateTool {
    fn name(&self) -> &str { "TeammateTool" }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let operation = call.require_string("operation")?;

        match operation.as_str() {
            "spawnTeam" => self.spawn_team(call).await,
            "discoverTeams" => self.discover_teams().await,
            "requestJoin" => self.request_join(call).await,
            "approveJoin" => self.approve_join(call).await,
            "rejectJoin" => self.reject_join(call).await,
            "cleanup" => self.cleanup().await,
            _ => Err(ToolError::InvalidArguments(format!("Unknown operation: {}", operation))),
        }
    }
}
```

### 6.5 SendMessageTool (消息发送)

```rust
// crates/sage-tools/src/tools/team/send_message.rs

pub struct SendMessageTool {
    message_bus: Arc<MessageBus>,
}

impl Tool for SendMessageTool {
    fn name(&self) -> &str { "SendMessageTool" }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let msg_type = call.require_string("type")?;

        match msg_type.as_str() {
            "message" => self.send_direct_message(call).await,
            "broadcast" => self.broadcast(call).await,
            "request" => self.send_request(call).await,
            "response" => self.send_response(call).await,
            _ => Err(ToolError::InvalidArguments(format!("Unknown type: {}", msg_type))),
        }
    }
}
```

## 7. 目录结构

```
crates/sage-tools/src/
├── lib.rs                      # 导出所有工具
├── tools/
│   ├── mod.rs                  # 工具模块聚合
│   ├── base_tool.rs            # 工具基类
│   ├── macros.rs               # 工具定义宏
│   │
│   ├── file_ops/               # 文件操作 (已实现)
│   │   ├── mod.rs
│   │   ├── read/
│   │   ├── write/
│   │   ├── edit/
│   │   ├── glob/
│   │   ├── grep/
│   │   └── notebook_edit/
│   │
│   ├── process/                # 进程执行 (已实现)
│   │   ├── mod.rs
│   │   ├── bash/
│   │   ├── task/
│   │   ├── task_output/
│   │   └── kill_shell/
│   │
│   ├── task_mgmt/              # 任务管理 (已实现)
│   │   ├── mod.rs
│   │   ├── task_create/
│   │   ├── task_update/
│   │   ├── task_list/
│   │   ├── task_get/
│   │   └── todo_write/
│   │
│   ├── planning/               # 规划模式 (已实现)
│   │   ├── mod.rs
│   │   ├── enter_plan_mode/
│   │   └── exit_plan_mode/
│   │
│   ├── interaction/            # 用户交互 (已实现)
│   │   ├── mod.rs
│   │   └── ask_user/
│   │
│   ├── network/                # 网络工具 (已实现)
│   │   ├── mod.rs
│   │   ├── web_fetch/
│   │   └── web_search/
│   │
│   ├── extensions/             # 扩展工具
│   │   ├── mod.rs
│   │   ├── skill.rs            # (已实现)
│   │   └── tool_search.rs      # (待实现)
│   │
│   ├── code_intelligence/      # 代码智能 (待实现)
│   │   ├── mod.rs
│   │   └── lsp.rs
│   │
│   ├── browser/                # 浏览器自动化 (待实现)
│   │   ├── mod.rs
│   │   └── computer.rs
│   │
│   └── team/                   # 团队协作 (待实现)
│       ├── mod.rs
│       ├── teammate.rs
│       ├── send_message.rs
│       └── team_manager.rs
│
└── providers/                  # 工具提供者
    ├── mod.rs
    ├── builtin.rs              # 内置工具提供者
    ├── mcp.rs                  # MCP 工具提供者
    └── custom.rs               # 自定义工具提供者
```

## 8. 实现优先级

### Phase 1: 基础架构 (高优先级)
1. [ ] 重构 ToolMetadata trait
2. [ ] 重构 ToolRegistry 支持延迟加载
3. [ ] 实现 ToolProvider 接口
4. [ ] 添加工具定义宏

### Phase 2: 缺失工具 (高优先级)
1. [ ] ToolSearch - 延迟加载工具搜索
2. [ ] LSP - Language Server Protocol 集成

### Phase 3: 高级功能 (中优先级)
1. [ ] Computer - 浏览器自动化
2. [ ] TeammateTool - 团队管理
3. [ ] SendMessageTool - 消息发送

### Phase 4: 优化 (低优先级)
1. [ ] 工具文档自动生成
2. [ ] 工具使用统计
3. [ ] 工具性能监控

## 9. 测试策略

### 9.1 单元测试
- 每个工具独立测试
- Mock 外部依赖
- 覆盖所有参数组合

### 9.2 集成测试
- 工具组合测试
- 端到端流程测试
- 性能基准测试

### 9.3 兼容性测试
- Claude Code prompt 兼容性
- 参数格式兼容性
- 输出格式兼容性
