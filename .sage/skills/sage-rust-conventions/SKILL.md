---
name: sage-rust-conventions
description: Sage 项目 Rust 代码规范，包含命名、错误处理、异步、测试等最佳实践
when_to_use: 当编写或审查 Sage 项目的 Rust 代码时使用
allowed_tools:
  - Read
  - Grep
  - Glob
  - Edit
  - Write
  - Bash
user_invocable: true
priority: 85
---

# Sage Rust 代码规范

## 命名规范 (RFC 430)

### 缩写词处理

**缩写词当作普通单词处理，不使用全大写**：

```rust
// ✓ 正确
struct LlmClient;
struct SseDecoder;
struct McpServer;
struct HttpRequest;
struct TlsConfig;
struct SqliteBackend;

// ✗ 错误
struct LLMClient;
struct SSEDecoder;
struct MCPServer;
struct HTTPRequest;
struct TLSConfig;
struct SQLiteBackend;
```

### 模块命名

```rust
// ✓ 正确 - snake_case
pub mod rate_limiter;
pub mod circuit_breaker;
pub mod llm_client;

// ✗ 错误
pub mod rateLimiter;
pub mod CircuitBreaker;
```

### 函数和变量

```rust
// ✓ 正确 - snake_case
fn parse_llm_response() { }
let api_key = config.get_api_key();

// ✗ 错误
fn parseLLMResponse() { }
let apiKey = config.getApiKey();
```

### 常量和静态变量

```rust
// ✓ 正确 - SCREAMING_SNAKE_CASE
const MAX_RETRIES: u32 = 5;
static DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

// ✗ 错误
const maxRetries: u32 = 5;
```

## 错误处理

### 使用 thiserror 定义错误

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ToolError {
    #[error("Tool '{0}' not found")]
    NotFound(String),

    #[error("Invalid parameters: {0}")]
    InvalidParams(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(#[from] std::io::Error),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}
```

### 使用 anyhow 处理应用错误

```rust
use anyhow::{Context, Result, bail, ensure};

fn load_config(path: &Path) -> Result<Config> {
    let content = fs::read_to_string(path)
        .context("Failed to read config file")?;

    let config: Config = serde_json::from_str(&content)
        .context("Failed to parse config JSON")?;

    ensure!(!config.api_key.is_empty(), "API key cannot be empty");

    Ok(config)
}
```

### 永远不要 unwrap

```rust
// ✓ 正确
let value = map.get("key").ok_or(Error::KeyNotFound)?;
let parsed = input.parse::<i32>().context("Invalid number")?;

// 只在测试或确定安全时使用 expect
let home = std::env::var("HOME").expect("HOME must be set");

// ✗ 错误
let value = map.get("key").unwrap();
let parsed = input.parse::<i32>().unwrap();
```

## 异步代码

### 使用 tokio 运行时

```rust
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    let result = fetch_data().await?;
    Ok(())
}

// 或在库代码中
pub async fn process() -> Result<()> {
    // 异步操作
    Ok(())
}
```

### async_trait 用于异步 trait

```rust
use async_trait::async_trait;

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn chat(&self, request: ChatRequest) -> Result<LlmResponse>;
}

#[async_trait]
impl LlmClient for AnthropicClient {
    async fn chat(&self, request: ChatRequest) -> Result<LlmResponse> {
        // 实现
    }
}
```

### 避免阻塞异步上下文

```rust
// ✓ 正确 - 使用 tokio 的异步 IO
use tokio::fs;

async fn read_file(path: &Path) -> Result<String> {
    fs::read_to_string(path).await.context("Failed to read file")
}

// ✓ 正确 - 对于 CPU 密集型任务使用 spawn_blocking
use tokio::task;

async fn compute_hash(data: Vec<u8>) -> Result<String> {
    task::spawn_blocking(move || {
        let hash = expensive_hash(&data);
        Ok(hash)
    }).await?
}

// ✗ 错误 - 在异步上下文中阻塞
async fn bad_read(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).context("Failed")  // 阻塞!
}
```

## 所有权和借用

### 优先使用引用

```rust
// ✓ 正确 - 使用 &str 而非 String
fn process_name(name: &str) -> Result<()> {
    println!("Processing: {}", name);
    Ok(())
}

// ✓ 正确 - 使用 &[T] 而非 Vec<T>
fn sum(numbers: &[i32]) -> i32 {
    numbers.iter().sum()
}
```

### 使用 Cow 处理可能克隆的场景

```rust
use std::borrow::Cow;

fn normalize_path(path: &str) -> Cow<'_, str> {
    if path.contains("//") {
        Cow::Owned(path.replace("//", "/"))
    } else {
        Cow::Borrowed(path)
    }
}
```

### 避免不必要的 clone

```rust
// ✓ 正确
fn process(data: &Data) {
    // 使用引用
}

// ✗ 错误
fn process(data: Data) {
    // 强制调用者 clone
}

// ✓ 正确 - 需要所有权时使用 Into
fn store<S: Into<String>>(s: S) {
    let owned: String = s.into();
}
```

## 文件组织

### 模块结构

```rust
// mod.rs - 公开接口
//! 模块描述

mod implementation;
mod types;
mod tests;

pub use implementation::MainStruct;
pub use types::{Config, Error};
```

### 文件大小限制

**每个文件 < 200 行**，超过则拆分：

```
// 不好 - 一个大文件
tools/executor.rs (800 行)

// 好 - 拆分为子模块
tools/
├── executor/
│   ├── mod.rs       (50 行)
│   ├── runner.rs    (150 行)
│   ├── scheduler.rs (120 行)
│   └── result.rs    (80 行)
```

## 测试

### 测试组织

```rust
// 单元测试放在同一文件
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_case() {
        // Arrange
        let input = create_input();

        // Act
        let result = function_under_test(&input);

        // Assert
        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn test_async_function() {
        let result = async_function().await;
        assert!(result.is_ok());
    }
}
```

### 测试命名

```rust
#[test]
fn test_parse_valid_json() { }

#[test]
fn test_parse_invalid_json_returns_error() { }

#[test]
fn test_empty_input_returns_default() { }
```

### 使用 assert 宏

```rust
// 基本断言
assert!(condition);
assert_eq!(actual, expected);
assert_ne!(actual, unexpected);

// 带消息
assert!(result.is_ok(), "Expected Ok, got {:?}", result);

// 模式匹配
assert!(matches!(result, Ok(Value::String(_))));
```

## 文档

### 公共 API 必须文档化

```rust
/// 执行工具调用
///
/// # Arguments
///
/// * `call` - 工具调用请求
///
/// # Returns
///
/// 成功时返回工具执行结果，失败时返回错误
///
/// # Errors
///
/// * `ToolError::NotFound` - 工具不存在
/// * `ToolError::PermissionDenied` - 权限不足
///
/// # Examples
///
/// ```
/// let result = executor.execute(&call).await?;
/// println!("Output: {}", result.output);
/// ```
pub async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
    // ...
}
```

### 模块级文档

```rust
//! # 工具执行模块
//!
//! 提供工具注册、权限检查和执行功能。
//!
//! ## 功能
//!
//! - 工具注册表管理
//! - 权限验证
//! - 沙箱执行
//!
//! ## 示例
//!
//! ```
//! use sage_core::tools::{ToolRegistry, ToolExecutor};
//!
//! let registry = ToolRegistry::new();
//! let executor = ToolExecutor::new(registry);
//! ```
```

## 常用模式

### Builder 模式

```rust
pub struct ConfigBuilder {
    api_key: Option<String>,
    timeout: Duration,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self {
            api_key: None,
            timeout: Duration::from_secs(30),
        }
    }

    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn build(self) -> Result<Config, ConfigError> {
        let api_key = self.api_key.ok_or(ConfigError::MissingApiKey)?;
        Ok(Config {
            api_key,
            timeout: self.timeout,
        })
    }
}
```

### 类型状态模式

```rust
pub struct Request<State> {
    url: String,
    _state: PhantomData<State>,
}

pub struct NoBody;
pub struct WithBody;

impl Request<NoBody> {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            _state: PhantomData,
        }
    }

    pub fn body(self, body: impl Serialize) -> Request<WithBody> {
        Request {
            url: self.url,
            _state: PhantomData,
        }
    }
}

impl Request<WithBody> {
    pub async fn send(self) -> Result<Response> {
        // 只有有 body 的请求才能发送
    }
}
```

## 禁止事项

1. **不要使用 `unwrap()`** - 使用 `?` 或 `expect("reason")`
2. **不要阻塞异步上下文** - 使用 tokio 的异步 API
3. **不要过度 clone** - 优先使用引用
4. **不要写超过 200 行的文件** - 拆分为子模块
5. **不要使用全大写缩写** - `LlmClient` 而非 `LLMClient`
6. **不要忽略错误** - 处理或传播每个 Result
7. **不要添加不必要的 pub** - 最小化公开 API
