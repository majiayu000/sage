# 代码复杂度分析报告

**分析日期**: 2026-01-21
**代码库**: sage
**分析范围**: 全部 Rust 源文件

---

## 目录

1. [概述](#概述)
2. [过大的文件（超过 500 行）](#过大的文件超过-500-行)
3. [过长的函数（超过 50 行）](#过长的函数超过-50-行)
4. [过深的嵌套（超过 3 层）](#过深的嵌套超过-3-层)
5. [复杂的条件逻辑](#复杂的条件逻辑)
6. [过多的参数（超过 5 个）](#过多的参数超过-5-个)
7. [重构优先级建议](#重构优先级建议)

---

## 概述

本次分析共检查了 sage 代码库中的所有 Rust 源文件。以下是发现的主要复杂度问题汇总：

| 类别 | 问题数量 | 严重程度 |
|------|----------|----------|
| 过大的文件 | 9 个 | 中 |
| 过长的函数 | 15+ 个 | 高 |
| 过深的嵌套 | 8+ 处 | 高 |
| 复杂条件逻辑 | 6+ 处 | 中 |
| 过多参数 | 2 处 | 低 |

---

## 过大的文件（超过 500 行）

按行数降序排列：

### 1. `rnk_app.rs` - 754 行

**文件路径**: `/crates/sage-cli/src/ui/rnk_app.rs`

**复杂度指标**:
- 行数: 754
- 函数数量: 15+
- 主要职责: 终端 UI 应用实现

**问题分析**:
- 文件承担过多职责：UI 渲染、事件处理、状态管理、消息格式化
- 包含多个超过 50 行的函数
- 嵌套层级深的事件处理逻辑

**重构建议**:
```
rnk_app.rs (754 行)
├── app_state.rs      # 状态管理和更新逻辑
├── event_handler.rs  # 键盘和事件处理
├── message_format.rs # 消息格式化
└── app.rs           # 主应用入口（约 150 行）
```

---

### 2. `diagnostics.rs` - 678 行

**文件路径**: `/crates/sage-cli/src/commands/diagnostics.rs`

**复杂度指标**:
- 行数: 678
- 函数数量: 10+
- 主要职责: CLI 诊断命令实现

**问题分析**:
- `doctor()`、`status()`、`usage()` 等命令全部在同一文件
- 每个命令函数超过 70 行
- 数据收集和格式化逻辑耦合

**重构建议**:
```
diagnostics/
├── mod.rs           # 模块入口
├── doctor.rs        # 健康检查命令
├── status.rs        # 状态命令
├── usage.rs         # 使用统计命令
└── formatters.rs    # 共享格式化逻辑
```

---

### 3. `resolved.rs` - 583 行

**文件路径**: `/crates/sage-core/src/config/credential/resolved.rs`

**复杂度指标**:
- 行数: 583
- 测试代码占比: ~60%
- 主要职责: 凭证解析与来源追踪

**问题分析**:
- 文件结构良好，但测试代码过多
- 生产代码约 230 行，符合规范

**重构建议**:
- 将测试代码移至独立的 `tests/` 目录
- 当前结构可接受，优先级低

---

### 4. `strategy.rs` - 550 行

**文件路径**: `/crates/sage-core/src/output/strategy.rs`

**复杂度指标**:
- 行数: 550
- 策略实现数量: 5 个
- 主要职责: 输出策略模式实现

**问题分析**:
- 包含 5 个输出策略实现（Streaming、Batch、JSON、Silent、Rnk）
- 每个策略约 50-80 行
- `ThinkingAnimation` 独立功能嵌入文件中

**重构建议**:
```
output/
├── mod.rs              # 导出和 OutputMode 枚举
├── strategy.rs         # OutputStrategy trait 定义
├── streaming.rs        # StreamingOutput 实现
├── batch.rs            # BatchOutput 实现
├── json.rs             # JsonOutput 实现
├── silent.rs           # SilentOutput 实现
├── rnk.rs              # RnkOutput 实现
└── animation.rs        # ThinkingAnimation 独立模块
```

---

### 5. `tool_orchestrator.rs` - 550 行

**文件路径**: `/crates/sage-core/src/agent/unified/tool_orchestrator.rs`

**复杂度指标**:
- 行数: 550
- 公共方法数量: 15+
- 主要职责: 工具执行编排（三阶段模型）

**问题分析**:
- 三个执行阶段函数各超过 60 行
- `execution_phase` 中 `SupervisionResult` 匹配逻辑复杂
- 检查点管理逻辑与执行逻辑耦合

**重构建议**:
```
tool_orchestrator/
├── mod.rs              # 主编排器结构
├── phases/
│   ├── pre_execution.rs
│   ├── execution.rs
│   └── post_execution.rs
├── checkpoint.rs       # 检查点管理逻辑
└── context.rs          # ToolExecutionContext
```

---

### 6. `system_prompt.rs` - 522 行

**文件路径**: `/crates/sage-core/src/prompts/system_prompt.rs`

**复杂度指标**:
- 行数: 522
- 常量字符串数量: 5+
- 主要职责: 系统提示词定义

**问题分析**:
- 包含大量静态字符串常量
- 字符串内容难以维护和国际化
- `build_main_prompt()` 函数组装多个提示词片段

**重构建议**:
- 考虑将提示词模板移至外部文件（`.txt` 或 `.md`）
- 使用 `include_str!` 宏加载
- 或保持现状，此类文件行数多但复杂度低

---

### 7. `todo_write.rs` - 515 行

**文件路径**: `/crates/sage-tools/src/tools/task_mgmt/todo_write.rs`

**复杂度指标**:
- 行数: 515
- 文档字符串占比: ~50%
- 主要职责: Todo 任务管理工具

**问题分析**:
- `description()` 方法包含约 100 行内联文档字符串
- 实际业务逻辑约 250 行
- 结构清晰，主要是文档体积大

**重构建议**:
- 将工具描述文档移至外部 markdown 文件
- 使用 `include_str!("todo_write.md")` 加载
- 优先级低，当前可接受

---

### 8. `builder.rs` - 504 行

**文件路径**: `/crates/sage-core/src/prompts/builder.rs`

**复杂度指标**:
- 行数: 504
- 方法数量: 20+
- 主要职责: 系统提示词 Builder 模式

**问题分析**:
- 使用流畅接口（Fluent API）设计
- 每个方法短小精悍（5-15 行）
- 整体结构良好

**重构建议**:
- 当前结构可接受
- 如需拆分，可按功能分组（基础配置、工具配置、上下文配置）
- 优先级低

---

### 9. `provider_registry.rs` - 501 行

**文件路径**: `/crates/sage-core/src/config/provider_registry.rs`

**复杂度指标**:
- 行数: 501
- Provider 定义数量: 7 个
- 主要职责: LLM Provider 注册与缓存

**问题分析**:
- `embedded_providers()` 函数约 170 行，包含所有内置 Provider 定义
- 数据定义硬编码在代码中
- 测试代码约占 90 行

**重构建议**:
- 将 Provider 定义移至 JSON/TOML 配置文件
- 使用 `serde` 反序列化加载
- 代码仅保留加载和缓存逻辑

---

## 过长的函数（超过 50 行）

按行数降序排列：

### 1. `embedded_providers()` - 170 行

**位置**: `/crates/sage-core/src/config/provider_registry.rs:220-390`

**复杂度指标**:
- 行数: 170
- 圈复杂度: 1（无分支）
- 认知复杂度: 低

**问题分析**:
- 纯数据初始化函数
- 无控制流复杂度
- 但难以维护和扩展

**重构建议**:
```rust
// 重构前
fn embedded_providers(&self) -> Vec<ProviderInfo> {
    vec![
        ProviderInfo { id: "anthropic".to_string(), ... },
        ProviderInfo { id: "openai".to_string(), ... },
        // 170 行数据定义
    ]
}

// 重构后
fn embedded_providers(&self) -> Vec<ProviderInfo> {
    const PROVIDERS_JSON: &str = include_str!("../data/providers.json");
    serde_json::from_str(PROVIDERS_JSON).expect("valid embedded providers")
}
```

---

### 2. `app()` - 117 行

**位置**: `/crates/sage-cli/src/ui/rnk_app.rs:253-370`

**复杂度指标**:
- 行数: 117
- 圈复杂度: 高（多层嵌套）
- 认知复杂度: 高

**问题分析**:
- UI 渲染和事件处理混合
- 键盘事件处理嵌套 4+ 层
- 多个 `match` 和 `if let` 嵌套

**重构建议**:
```rust
// 重构前
fn app(frame: &mut Frame, state: &mut AppState) {
    // 117 行混合逻辑
}

// 重构后
fn app(frame: &mut Frame, state: &mut AppState) {
    render_layout(frame, state);
    // 事件处理移至独立函数
}

fn render_layout(frame: &mut Frame, state: &AppState) { ... }
fn handle_keyboard_event(state: &mut AppState, key: KeyEvent) { ... }
```

---

### 3. `usage()` - 111 行

**位置**: `/crates/sage-cli/src/commands/diagnostics.rs:243-354`

**复杂度指标**:
- 行数: 111
- 圈复杂度: 中
- 认知复杂度: 中

**问题分析**:
- 数据收集、计算、格式化在同一函数
- 多层循环遍历会话数据
- 错误处理分散

**重构建议**:
```rust
// 拆分为三个函数
async fn usage(sessions_dir: &Path, json_output: bool) -> Result<()> {
    let stats = collect_usage_stats(sessions_dir).await?;
    if json_output {
        print_json_stats(&stats)?;
    } else {
        print_formatted_stats(&stats);
    }
    Ok(())
}

async fn collect_usage_stats(dir: &Path) -> Result<UsageStats> { ... }
fn print_json_stats(stats: &UsageStats) -> Result<()> { ... }
fn print_formatted_stats(stats: &UsageStats) { ... }
```

---

### 4. `run_rnk_app()` - 92 行

**位置**: `/crates/sage-cli/src/ui/rnk_app.rs:635-727`

**复杂度指标**:
- 行数: 92
- 圈复杂度: 中
- 认知复杂度: 中

**问题分析**:
- 应用初始化和主循环入口
- 异步 spawn 和 channel 设置
- 错误处理和清理逻辑

**重构建议**:
```rust
// 拆分初始化和运行
async fn run_rnk_app(...) -> Result<()> {
    let (state, channels) = initialize_app(...)?;
    let result = run_main_loop(state, channels).await;
    cleanup().await;
    result
}
```

---

### 5. `execution_phase()` - 84 行

**位置**: `/crates/sage-core/src/agent/unified/tool_orchestrator.rs:317-401`

**复杂度指标**:
- 行数: 84
- 圈复杂度: 高（6 个 match 分支）
- 认知复杂度: 高

**问题分析**:
- `SupervisionResult` 匹配有 6 个分支
- 每个分支包含日志记录和结果转换
- 重试逻辑隐含在 `Restarted` 分支

**重构建议**:
```rust
// 提取结果转换逻辑
fn handle_supervision_result(
    &self,
    result: SupervisionResult,
    tool_call: &ToolCall,
    cancel_token: CancellationToken,
) -> ToolResult {
    match result {
        SupervisionResult::Completed => self.execute_tool_direct(...).await,
        SupervisionResult::Restarted { attempt } => {
            self.log_restart(tool_call, attempt);
            self.execute_tool_direct(...).await
        }
        // 其他分支
    }
}
```

---

### 6. `format_message()` - 81 行

**位置**: `/crates/sage-cli/src/ui/rnk_app.rs:131-212`

**复杂度指标**:
- 行数: 81
- 圈复杂度: 中
- 认知复杂度: 中

**问题分析**:
- 处理多种 `MessageContent` 变体
- 嵌套 match 处理不同内容类型
- 字符串拼接逻辑分散

**重构建议**:
```rust
// 为每种内容类型提供独立格式化函数
fn format_message(msg: &Message) -> Vec<Line> {
    msg.content.iter()
        .flat_map(|content| format_content(content))
        .collect()
}

fn format_content(content: &MessageContent) -> Vec<Line> {
    match content {
        MessageContent::Text { text } => format_text(text),
        MessageContent::ToolUse { name, input, .. } => format_tool_use(name, input),
        MessageContent::ToolResult { content, .. } => format_tool_result(content),
        // ...
    }
}
```

---

### 7. `executor_loop()` - 75 行

**位置**: `/crates/sage-cli/src/ui/rnk_app.rs:558-632`

**复杂度指标**:
- 行数: 75
- 圈复杂度: 中
- 认知复杂度: 中

**问题分析**:
- 命令处理循环
- `tokio::select!` 多分支
- 状态更新和消息发送混合

**重构建议**:
- 提取命令处理逻辑为独立函数
- 使用状态机模式管理执行状态

---

### 8. `doctor()` - 71 行

**位置**: `/crates/sage-cli/src/commands/diagnostics.rs:73-144`

**复杂度指标**:
- 行数: 71
- 圈复杂度: 中
- 认知复杂度: 低

**问题分析**:
- 健康检查逻辑清晰
- 但所有检查项在同一函数
- 添加新检查项需修改函数

**重构建议**:
```rust
// 使用检查项列表模式
async fn doctor() -> Result<()> {
    let checks: Vec<Box<dyn HealthCheck>> = vec![
        Box::new(ConfigCheck),
        Box::new(ApiKeyCheck),
        Box::new(NetworkCheck),
        // 易于扩展
    ];

    for check in checks {
        check.run().await?;
    }
    Ok(())
}
```

---

### 9. `pre_execution_phase()` - 70 行

**位置**: `/crates/sage-core/src/agent/unified/tool_orchestrator.rs:218-288`

**复杂度指标**:
- 行数: 70
- 圈复杂度: 中
- 认知复杂度: 中

**问题分析**:
- 检查点创建和 Hook 执行混合
- 多层条件判断
- 错误处理详细但分散

**重构建议**:
```rust
async fn pre_execution_phase(&self, ...) -> SageResult<PreExecutionResult> {
    self.create_checkpoint_if_needed(tool_call).await;
    self.execute_pre_hooks(tool_call, context, cancel_token).await
}

async fn create_checkpoint_if_needed(&self, tool_call: &ToolCall) {
    // 检查点逻辑独立
}

async fn execute_pre_hooks(&self, ...) -> SageResult<PreExecutionResult> {
    // Hook 执行逻辑独立
}
```

---

### 10. `post_execution_phase()` - 67 行

**位置**: `/crates/sage-core/src/agent/unified/tool_orchestrator.rs:423-490`

**复杂度指标**:
- 行数: 67
- 圈复杂度: 中
- 认知复杂度: 中

**问题分析**:
- 回滚逻辑和 Hook 执行混合
- 条件嵌套处理失败情况

**重构建议**:
- 提取 `handle_failure_rollback()` 函数
- 提取 `execute_post_hooks()` 函数

---

### 11. `message_receiver()` - 67 行

**位置**: `/crates/sage-core/src/mcp/client.rs:127-194`

**复杂度指标**:
- 行数: 67
- 圈复杂度: 高
- 认知复杂度: 高

**问题分析**:
- `tokio::select!` 内嵌套 `match`
- 消息解析和分发逻辑复杂
- 错误处理分散

**重构建议**:
```rust
async fn message_receiver(mut reader: ..., tx: ...) {
    loop {
        tokio::select! {
            result = reader.next_line() => {
                match result {
                    Some(Ok(line)) => self.handle_message(line, &tx).await,
                    Some(Err(e)) => self.handle_error(e, &tx).await,
                    None => break,
                }
            }
        }
    }
}

async fn handle_message(&self, line: String, tx: &Sender) {
    // 消息处理逻辑独立
}
```

---

## 过深的嵌套（超过 3 层）

### 1. `app()` 键盘事件处理 - 5 层嵌套

**位置**: `/crates/sage-cli/src/ui/rnk_app.rs:253-370`

**嵌套结构**:
```
match event {                           // 第 1 层
    Event::Key(key) => {               // 第 2 层
        match key.code {               // 第 3 层
            KeyCode::Enter => {        // 第 4 层
                if !input.is_empty() { // 第 5 层
                    // 处理逻辑
                }
            }
        }
    }
}
```

**重构建议**:
- 使用 early return 减少嵌套
- 提取 `handle_key_event()` 函数
- 使用 `if let` 链式匹配

---

### 2. `message_receiver()` - 4 层嵌套

**位置**: `/crates/sage-core/src/mcp/client.rs:127-194`

**嵌套结构**:
```
loop {                                    // 第 1 层
    tokio::select! {                     // 第 2 层
        result = reader.next_line() => { // 第 3 层
            match result {               // 第 4 层
                Some(Ok(line)) => {      // 第 5 层（含 if 判断）
                    if let Ok(msg) = serde_json::from_str(&line) {
                        // 处理
                    }
                }
            }
        }
    }
}
```

**重构建议**:
- 提取消息解析函数
- 使用 `?` 操作符简化错误处理

---

### 3. `format_message()` - 4 层嵌套

**位置**: `/crates/sage-cli/src/ui/rnk_app.rs:131-212`

**嵌套结构**:
```
for content in &msg.content {           // 第 1 层
    match content {                     // 第 2 层
        MessageContent::ToolResult { content, .. } => {  // 第 3 层
            for item in content {       // 第 4 层
                match item {            // 第 5 层
                    // 处理
                }
            }
        }
    }
}
```

**重构建议**:
- 为每种内容类型创建独立格式化函数
- 使用迭代器链式调用

---

### 4. `usage()` 统计收集 - 4 层嵌套

**位置**: `/crates/sage-cli/src/commands/diagnostics.rs:243-354`

**嵌套结构**:
```
for entry in entries {                   // 第 1 层
    if let Ok(session) = load_session(&entry) {  // 第 2 层
        for message in &session.messages {       // 第 3 层
            if let Some(usage) = &message.usage {  // 第 4 层
                // 累加统计
            }
        }
    }
}
```

**重构建议**:
- 使用 `filter_map` 和 `flat_map` 简化
- 提取 `collect_session_stats()` 函数

---

### 5. `execution_phase()` Supervision 处理 - 4 层嵌套

**位置**: `/crates/sage-core/src/agent/unified/tool_orchestrator.rs:317-401`

**嵌套结构**:
```
match supervision_result {              // 第 1 层
    SupervisionResult::Stopped { error } => {  // 第 2 层
        tracing::warn!(...);           // 第 3 层
        ToolResult::error(...)         // 第 4 层（含格式化）
    }
}
```

**重构建议**:
- 提取结果转换辅助函数
- 使用模式匹配 guard

---

## 复杂的条件逻辑

### 1. `format_message()` 多类型分发

**位置**: `/crates/sage-cli/src/ui/rnk_app.rs:131-212`

**问题描述**:
- 处理 5+ 种 `MessageContent` 变体
- 每种变体有不同的格式化逻辑
- 部分变体包含嵌套内容需递归处理

**复杂度评估**: 高

**重构建议**:
- 实现 `Display` trait 或自定义 `Format` trait
- 使用访问者模式处理不同类型
- 每种类型独立的格式化函数

---

### 2. `execution_phase()` SupervisionResult 匹配

**位置**: `/crates/sage-core/src/agent/unified/tool_orchestrator.rs:358-400`

**问题描述**:
- 6 种 `SupervisionResult` 变体
- 每种变体需要不同的日志记录和结果转换
- 部分变体需要重试执行

**复杂度评估**: 中高

**重构建议**:
```rust
impl SupervisionResult {
    fn into_tool_result(self, orchestrator: &ToolOrchestrator, call: &ToolCall) -> ToolResult {
        // 在 SupervisionResult 上实现转换方法
    }
}
```

---

### 3. `wrap_text_with_prefix()` 换行逻辑

**位置**: `/crates/sage-cli/src/ui/rnk_app.rs`

**问题描述**:
- 处理文本换行、前缀、缩进
- 多个条件判断边界情况
- Unicode 字符宽度计算

**复杂度评估**: 中

**重构建议**:
- 使用 `textwrap` crate 简化
- 提取独立的文本处理模块

---

### 4. `pre_execution_phase()` 检查点逻辑

**位置**: `/crates/sage-core/src/agent/unified/tool_orchestrator.rs:218-253`

**问题描述**:
```rust
if self.checkpoint_config.enabled {
    if let Some(manager) = &self.checkpoint_manager {
        if manager.should_checkpoint_for_tool(&tool_call.name) {
            let affected_files = self.extract_affected_files(tool_call);
            if !affected_files.is_empty() {
                // 创建检查点
            }
        }
    }
}
```

**复杂度评估**: 中

**重构建议**:
- 使用 guard clause（早返回）
- 提取 `should_create_checkpoint()` 谓词函数

---

## 过多的参数（超过 5 个）

本次分析未发现明显的参数过多问题。大多数函数参数在 3-5 个范围内，符合最佳实践。

### 潜在优化点

#### 1. `execute_tool_call()` - 4 个参数

**位置**: `/crates/sage-core/src/agent/unified/tool_orchestrator.rs:523-549`

```rust
pub async fn execute_tool_call(
    &self,
    tool_call: &ToolCall,
    context: &ToolExecutionContext,
    cancel_token: CancellationToken,
) -> SageResult<ToolResult>
```

**评估**: 参数数量可接受，但可考虑使用 `ExecutionRequest` 结构体封装。

#### 2. `on_tool_result()` - 3 个参数 + 隐式 self

**位置**: `/crates/sage-core/src/output/strategy.rs`

```rust
fn on_tool_result(&self, success: bool, output: Option<&str>, error: Option<&str>);
```

**评估**: 可考虑使用 `ToolResultEvent` 结构体封装。

---

## 重构优先级建议

基于复杂度影响和维护成本，建议按以下优先级进行重构：

### 高优先级（应尽快处理）

| 文件/函数 | 问题 | 建议 |
|-----------|------|------|
| `rnk_app.rs` | 754 行，多个长函数 | 拆分为 4 个模块 |
| `app()` | 117 行，5 层嵌套 | 提取事件处理函数 |
| `diagnostics.rs` | 678 行 | 按命令拆分模块 |
| `execution_phase()` | 84 行，复杂匹配 | 提取结果转换方法 |

### 中优先级（计划内处理）

| 文件/函数 | 问题 | 建议 |
|-----------|------|------|
| `strategy.rs` | 550 行，5 个实现 | 每个策略独立文件 |
| `tool_orchestrator.rs` | 550 行 | 按阶段拆分 |
| `embedded_providers()` | 170 行数据 | 移至配置文件 |
| `format_message()` | 81 行，嵌套深 | 按类型拆分格式化 |

### 低优先级（可接受现状）

| 文件/函数 | 问题 | 原因 |
|-----------|------|------|
| `resolved.rs` | 583 行 | 主要是测试代码 |
| `system_prompt.rs` | 522 行 | 静态字符串，复杂度低 |
| `todo_write.rs` | 515 行 | 文档占比大 |
| `builder.rs` | 504 行 | 方法短小，结构良好 |

---

## 总结

sage 代码库整体质量良好，代码组织清晰。主要复杂度问题集中在：

1. **UI 层**（`rnk_app.rs`）- 事件处理和渲染逻辑耦合，嵌套深
2. **CLI 命令**（`diagnostics.rs`）- 多个命令实现在同一文件
3. **工具编排**（`tool_orchestrator.rs`）- 三阶段模型可进一步解耦
4. **输出策略**（`strategy.rs`）- 多个实现可拆分为独立文件

建议优先处理 UI 层的复杂度问题，因为这是用户交互的核心模块，维护频率较高。其他模块可在后续迭代中逐步优化。

**预计重构工作量**：
- 高优先级：3-5 天
- 中优先级：2-3 天
- 低优先级：1 天（可选）
