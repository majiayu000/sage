# 错误处理分析报告

**生成日期:** 2026-01-21
**代码库:** Sage Agent
**分析范围:** 全部 Rust 源文件 (约 1010 个文件)

---

## 概述

本报告对 Sage 代码库的错误处理进行全面分析，识别潜在风险并提供改进建议。

### 统计摘要

| 类型 | 数量 | 风险等级 |
|------|------|----------|
| `unwrap()` 调用 | ~1948 | 需逐一评估 |
| `expect()` 调用 | 21 | 高风险 4 处 |
| `todo!()` 宏 | 4 | 中风险 |
| `panic!()` 宏 | 5 | 需评估 |

---

## 1. 高风险问题

### 1.1 生产代码中的 `expect()` 调用

#### 问题 1.1.1: HTTP 客户端初始化

**文件路径:** `/crates/sage-tools/src/tools/network/web_fetch.rs`
**行号:** 20

```rust
fn get_client() -> &'static reqwest::Client {
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client")  // 高风险
    })
}
```

**问题描述:**
- `expect()` 在生产代码中可能导致程序崩溃
- HTTP 客户端创建失败时应该返回错误而非 panic
- 这是网络工具的核心功能，失败会影响所有 web_fetch 操作

**建议修复方案:**
```rust
fn get_client() -> Result<&'static reqwest::Client, SageError> {
    CLIENT.get_or_try_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| SageError::http(format!("Failed to create HTTP client: {}", e)))
    })
}
```

---

#### 问题 1.1.2: 全局状态访问

**文件路径:** `/crates/sage-cli/src/ui/rnk_app.rs`
**行号:** 257-258

```rust
let state = GLOBAL_STATE.get().expect("State not initialized");
let cmd_tx = GLOBAL_CMD_TX.get().expect("Command channel not initialized");
```

**问题描述:**
- UI 线程中的 `expect()` 会导致整个应用崩溃
- 全局状态可能在竞态条件下未初始化
- 用户体验极差 - 应用突然退出无任何提示

**建议修复方案:**
```rust
let state = match GLOBAL_STATE.get() {
    Some(s) => s,
    None => {
        tracing::error!("Global state not initialized");
        return;  // 或显示错误消息
    }
};
```

---

#### 问题 1.1.3: 内存管理器初始化

**文件路径:** `/crates/sage-tools/src/tools/diagnostics/memory/types.rs`
**行号:** 43

```rust
let manager = MemoryManager::new(config)
    .await
    .expect("Failed to create default memory manager");
```

**问题描述:**
- 异步初始化中的 `expect()` 可能导致运行时崩溃
- 内存管理器配置问题应该优雅处理

**建议修复方案:**
```rust
let manager = MemoryManager::new(config)
    .await
    .map_err(|e| SageError::config(format!("Memory manager init failed: {}", e)))?;
```

---

### 1.2 UI 线程中的 `unwrap()` 调用

#### 问题 1.2.1: Spinner 动画线程

**文件路径:** `/crates/sage-cli/src/app.rs`
**行号:** 173

```rust
io::stdout().flush().unwrap();
```

**问题描述:**
- Spinner 线程中的 `unwrap()` 可能导致线程 panic
- 输出缓冲刷新失败在终端断开时可能发生
- 线程 panic 可能导致不一致的终端状态

**建议修复方案:**
```rust
if let Err(e) = io::stdout().flush() {
    tracing::debug!("Failed to flush stdout: {}", e);
    break;  // 优雅退出动画循环
}
```

---

#### 问题 1.2.2: ThinkingIndicator 线程

**文件路径:** `/crates/sage-cli/src/ui/indicators.rs`
**行号:** 86, 152

```rust
io::stdout().flush().unwrap();
```

**问题描述:**
- 与上述问题相同，但出现在多个位置
- 所有 indicator 线程都有相同风险

**建议修复方案:**
统一使用辅助函数处理输出:
```rust
fn safe_flush() -> bool {
    io::stdout().flush().is_ok()
}
```

---

### 1.3 正则表达式编译

#### 问题 1.3.1: 运行时正则编译

**文件路径:** `/crates/sage-tools/src/tools/monitoring/log_analyzer.rs`
**行号:** 多处

```rust
let pattern = Regex::new(r"...").unwrap();
```

**问题描述:**
- 正则表达式在运行时编译并 `unwrap()`
- 虽然静态正则表达式不太可能失败，但这不是最佳实践
- 如果正则有 bug，会导致运行时崩溃

**建议修复方案:**
使用 `lazy_static!` 或 `once_cell` 在编译时验证:
```rust
use once_cell::sync::Lazy;
static PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"...").expect("Invalid regex pattern")  // 启动时验证
});
```

---

## 2. 中风险问题

### 2.1 错误类型转换丢失信息

#### 问题 2.1.1: ToolError 转 SageError 丢失工具名

**文件路径:** `/crates/sage-core/src/tools/base/error.rs`
**行号:** 完整的 `From<ToolError>` 实现

```rust
impl From<ToolError> for SageError {
    fn from(err: ToolError) -> Self {
        match err {
            ToolError::InvalidArguments(msg) => SageError::tool("unknown", msg),
            ToolError::ExecutionFailed(msg) => SageError::tool("unknown", msg),
            ToolError::NotFound(msg) => SageError::tool("unknown", msg),
            ToolError::Cancelled => SageError::Cancelled,
            ToolError::Timeout(duration) => SageError::timeout(duration.as_secs()),
            ToolError::PermissionDenied(msg) => SageError::tool("unknown", msg),
            ToolError::Other(msg) => SageError::other(msg),
        }
    }
}
```

**问题描述:**
- 所有 `ToolError` 转换后工具名都变成 `"unknown"`
- 这导致错误日志和用户消息缺乏关键调试信息
- 例如: "Tool 'unknown' failed: file not found" 无法帮助定位问题

**建议修复方案:**
为 `ToolError` 添加工具名字段:
```rust
pub enum ToolError {
    InvalidArguments { tool: String, message: String },
    ExecutionFailed { tool: String, message: String },
    // ...
}

impl From<ToolError> for SageError {
    fn from(err: ToolError) -> Self {
        match err {
            ToolError::InvalidArguments { tool, message } =>
                SageError::tool(&tool, message),
            // ...
        }
    }
}
```

---

### 2.2 错误消息不够详细

#### 问题 2.2.1: 通用错误消息

**文件路径:** 多个文件
**位置示例:**
- `sage-tools/src/tools/vcs/git/operations/info.rs:84`
- `sage-tools/src/tools/vcs/git/operations/stash.rs:37`
- `sage-tools/src/tools/container/docker/commands.rs`

```rust
return Err(anyhow!("Invalid operation"));
```

**问题描述:**
- 错误消息缺乏上下文：什么操作？为什么无效？
- 用户无法根据此消息采取任何行动
- 调试时需要额外查看代码才能理解

**建议修复方案:**
```rust
return Err(anyhow!(
    "Invalid git info operation '{}': expected one of [status, log, diff]",
    operation
));
```

---

#### 问题 2.2.2: 超时错误缺乏上下文

**文件路径:**
- `/crates/sage-core/src/input/channel.rs:167`
- `/crates/sage-core/src/tools/executor.rs:115`

```rust
Err(_) => Err(SageError::timeout(timeout_duration.as_secs()))
```

**问题描述:**
- 超时错误只包含持续时间，不说明什么操作超时
- "Timeout after 30s" vs "Timeout waiting for user input after 30s"

**建议修复方案:**
```rust
SageError::timeout_with_context(
    timeout_duration.as_secs(),
    "waiting for tool execution to complete"
)
```

---

### 2.3 生产代码中的 `todo!()` 宏

**文件路径:**
- `/crates/sage-core/src/cache/semantic/encoder.rs`
- `/crates/sage-core/src/agent/unified/tool_confirmation.rs`
- `/crates/sage-core/src/mcp/discovery/scanner.rs`
- `/crates/sage-tools/src/tools/security/password.rs`

**问题描述:**
- `todo!()` 在运行时会 panic
- 这些是未完成的功能，但代码路径可能被意外触发
- 生产环境中应该返回适当的错误

**建议修复方案:**
```rust
// 不要这样
todo!("implement semantic encoding")

// 应该这样
return Err(SageError::other("Semantic encoding not yet implemented"));
```

---

## 3. 错误类型设计问题

### 3.1 错误分类使用字符串匹配

**文件路径:** `/crates/sage-core/src/recovery/mod.rs`

```rust
fn classify_error(error: &SageError) -> ErrorClass {
    match error {
        SageError::Http { message, .. } => {
            if message.contains("timeout") || message.contains("503") {
                ErrorClass::Transient
            } else if message.contains("401") || message.contains("403") {
                ErrorClass::Permanent
            } else {
                ErrorClass::Unknown
            }
        }
        // ...
    }
}
```

**问题描述:**
- 通过字符串包含来分类错误是脆弱的
- 错误消息格式变化会破坏分类逻辑
- 无法精确匹配错误类型

**建议修复方案:**
为 HTTP 错误添加结构化状态码:
```rust
pub enum SageError {
    Http {
        status_code: Option<u16>,  // 添加结构化字段
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

fn classify_error(error: &SageError) -> ErrorClass {
    match error {
        SageError::Http { status_code: Some(code), .. } => {
            match code {
                408 | 429 | 500..=599 => ErrorClass::Transient,
                401 | 403 | 404 => ErrorClass::Permanent,
                _ => ErrorClass::Unknown,
            }
        }
        // ...
    }
}
```

---

### 3.2 错误层级不一致

**问题描述:**
- `sage-core` 使用 `SageError` (thiserror)
- `sage-tools` 大量使用 `anyhow::Error`
- 这导致错误类型在边界处需要转换，可能丢失信息

**文件路径:** `/crates/sage-tools/` 全局

**建议修复方案:**
- 在 `sage-tools` 中定义 `ToolError` 变体覆盖所有工具错误场景
- 使用 `thiserror` 定义结构化错误
- 保持 `anyhow` 仅用于临时错误或顶层聚合

---

## 4. 错误恢复机制问题

### 4.1 恢复系统未充分利用

**文件路径:** `/crates/sage-core/src/recovery/mod.rs`

**观察:**
代码库中存在完善的恢复机制:
- `CircuitBreaker` - 熔断器
- `RateLimiter` - 限流器
- `RecoverableError` - 可恢复错误标记
- `BackoffStrategy` - 退避策略

**问题描述:**
- 这些机制主要在 LLM 调用中使用
- 工具执行和文件操作未使用恢复机制
- 网络工具（web_fetch）未使用熔断器

**建议修复方案:**
1. 为 `web_fetch` 添加熔断器支持
2. 为文件操作添加重试逻辑（针对临时锁定等）
3. 统一所有外部调用的错误恢复策略

---

### 4.2 静默吞没错误

**文件路径:**
- `/crates/sage-core/src/tools/parallel_executor/executor/executor.rs:194`
- `/crates/sage-core/src/tools/batch_executor.rs:185`
- `/crates/sage-core/src/cache/storage.rs:325`

```rust
Err(_) => {
    // 错误被静默忽略
}
```

**问题描述:**
- 错误信息完全丢失，无法调试
- 可能隐藏严重问题
- 违反"尽早失败"原则

**建议修复方案:**
```rust
Err(e) => {
    tracing::warn!(error = %e, "Operation failed, continuing with degraded mode");
    // 采取适当的降级措施
}
```

---

## 5. 错误传播链问题

### 5.1 缺乏 `.context()` 的裸 `?` 操作符

**问题描述:**
代码中存在约 30+ 处直接使用 `?` 而不添加上下文:

```rust
let content = fs::read_to_string(path)?;  // 不知道读取什么文件失败
```

**建议修复方案:**
```rust
let content = fs::read_to_string(path)
    .context(format!("Failed to read config file: {}", path.display()))?;
```

---

### 5.2 良好实践示例

以下位置展示了正确的错误传播:

**文件路径:**
- `/crates/sage-core/src/agent/unified/llm_orchestrator.rs`
- `/crates/sage-core/src/agent/unified/session_recording.rs`
- `/crates/sage-core/src/agent/subagent/runner.rs`

```rust
LlmClient::new(provider, provider_config, model_params)
    .context(format!("Failed to create LLM client for: {}", provider_name))?;
```

---

## 6. 低风险问题

### 6.1 ProgressStyle `unwrap()`

**文件路径:** `/crates/sage-cli/src/commands/interactive/onboarding.rs`
**行号:** 55

```rust
bar.set_style(ProgressStyle::default_spinner()
    .tick_chars("...")
    .template("{spinner:.blue} {msg}")
    .unwrap());
```

**问题描述:**
- 静态模板字符串的 `unwrap()` 风险较低
- 但仍应在启动时验证而非运行时

**建议修复方案:**
使用 `lazy_static` 预编译样式。

---

### 6.2 测试代码中的 `unwrap()`

**位置:** `/crates/*/tests/` 和 `#[cfg(test)]` 模块

**评估:** 低风险 - 测试代码中使用 `unwrap()` 是可接受的，因为:
- 测试失败应该 panic
- 提供清晰的失败点
- 不影响生产代码

---

## 7. 优化建议

### 7.1 短期改进 (1-2 周)

| 优先级 | 任务 | 影响 |
|--------|------|------|
| P0 | 移除生产代码中的 4 个 `expect()` | 防止运行时崩溃 |
| P0 | 修复 UI 线程中的 `unwrap()` | 提高稳定性 |
| P1 | 为 `ToolError` 添加工具名字段 | 改善错误诊断 |
| P1 | 替换 `todo!()` 为适当错误返回 | 防止意外 panic |

### 7.2 中期改进 (1-2 月)

| 优先级 | 任务 | 影响 |
|--------|------|------|
| P2 | 统一 `sage-tools` 使用 `ToolError` | 类型安全 |
| P2 | 为 HTTP 错误添加状态码字段 | 精确错误分类 |
| P2 | 实现错误代码系统 | 用户友好错误 |
| P2 | 添加缺失的 `.context()` 调用 | 改善调试体验 |

### 7.3 长期改进 (3+ 月)

| 优先级 | 任务 | 影响 |
|--------|------|------|
| P3 | 实现结构化错误日志 | 生产可观测性 |
| P3 | 添加错误遥测 | 主动问题发现 |
| P3 | 创建错误处理文档 | 开发者体验 |
| P3 | 扩展恢复机制到所有外部调用 | 系统弹性 |

---

## 错误处理检查清单

### 必须完成

- [ ] 移除所有生产路径中的 `expect()`
- [ ] 移除所有生产路径中不必要的 `unwrap()`
- [ ] 替换所有 `todo!()` 宏
- [ ] 修复 `ToolError` 转换丢失工具名问题

### 应该完成

- [ ] 为所有错误添加可操作的上下文
- [ ] 消除生产路径中的静默错误吞没
- [ ] 用户错误消息清晰且可操作
- [ ] 内部错误被适当记录

### 最佳实践

- [ ] 区分可重试和致命错误
- [ ] 超时错误说明什么操作超时
- [ ] 使用结构化字段而非字符串匹配进行错误分类
- [ ] 所有外部调用都有恢复策略

---

## 附录: 错误处理模式参考

### 推荐模式

```rust
// 1. 使用 context 添加上下文
file.read_to_string(&mut content)
    .context(format!("Failed to read {}", path.display()))?;

// 2. 使用 map_err 转换错误类型
let client = reqwest::Client::builder()
    .build()
    .map_err(|e| SageError::http(format!("Client creation failed: {}", e)))?;

// 3. 使用 match 处理可恢复错误
match operation() {
    Ok(result) => result,
    Err(e) if e.is_transient() => {
        tracing::warn!("Transient error, retrying: {}", e);
        retry_operation()?
    }
    Err(e) => return Err(e.into()),
}

// 4. 使用 Option 处理可选失败
let config = load_config().ok();  // 明确表示忽略错误是有意的
```

### 避免的模式

```rust
// 1. 避免: 裸 unwrap
let data = file.read().unwrap();  // 可能 panic

// 2. 避免: 静默忽略错误
let _ = save_cache();  // 错误被丢弃

// 3. 避免: 通用错误消息
return Err(anyhow!("Operation failed"));  // 无上下文

// 4. 避免: 不必要的 expect
let value = map.get("key").expect("key should exist");  // 使用 ok_or_else
```
