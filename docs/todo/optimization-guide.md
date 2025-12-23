# Sage Agent 代码优化指南

本文档定义了三个并行优化任务，每个任务由独立的子 agent 执行。

## 任务概览

| 任务 | 负责人 | 优先级 | 预估工作量 |
|-----|-------|-------|----------|
| 任务 1: 拆分超大文件 | Agent 1 | 高 | 重度 |
| 任务 2: 消除 unwrap/expect | Agent 2 | 高 | 中度 |
| 任务 3: 统一依赖和错误处理 | Agent 3 | 中 | 轻度 |

---

## 任务 1: 拆分超大文件

### 目标
将超过 200 行的文件拆分为更小的模块，遵循项目 CLAUDE.md 中的规则。

### 需要拆分的文件

#### 1.1 `crates/sage-core/src/config/validation.rs` (899 行)

拆分为：
```
config/validation/
├── mod.rs           # 模块导出和公共接口
├── provider.rs      # 提供者验证 (validate_provider_*)
├── model.rs         # 模型参数验证 (validate_model_*)
├── limits.rs        # 限制验证 (validate_*_limits)
├── tools.rs         # 工具配置验证 (validate_tool_*)
└── logging.rs       # 日志配置验证 (validate_logging_*)
```

#### 1.2 `crates/sage-core/src/tools/parallel_executor.rs` (855 行)

拆分为：
```
tools/parallel_executor/
├── mod.rs           # 模块导出和 ParallelToolExecutor 结构
├── core.rs          # 核心执行逻辑 (execute, execute_batch)
├── semaphore.rs     # 信号量管理
├── permission.rs    # 权限检查逻辑
└── stats.rs         # ExecutorStats 和统计
```

#### 1.3 `crates/sage-core/src/checkpoints/manager.rs` (875 行)

拆分为：
```
checkpoints/manager/
├── mod.rs           # 模块导出和 CheckpointManager 结构
├── lifecycle.rs     # 生命周期管理 (create, restore, delete)
├── storage.rs       # 存储操作
└── cache.rs         # 缓存管理
```

### 拆分规则

1. 每个新文件不超过 200 行
2. 保持公共 API 不变（向后兼容）
3. 使用 `pub use` 重新导出，确保外部代码无需修改
4. 添加模块级文档注释

### 验证方法

```bash
# 确保编译通过
cargo check

# 确保测试通过
cargo test -p sage-core
```

---

## 任务 2: 消除 unwrap/expect

### 目标
将代码中的 `unwrap()` 和 `expect()` 调用替换为正确的错误处理。

### 重点文件

按优先级排序：

1. **高优先级**（核心执行路径）：
   - `crates/sage-core/src/agent/unified/*.rs`
   - `crates/sage-core/src/llm/providers/*.rs`
   - `crates/sage-core/src/tools/executor.rs`

2. **中优先级**（工具实现）：
   - `crates/sage-tools/src/tools/network/*.rs`
   - `crates/sage-tools/src/tools/file_ops/*.rs`
   - `crates/sage-tools/src/tools/process/*.rs`

3. **低优先级**（可暂时保留测试中的 unwrap）：
   - `*_tests.rs` 文件中的 unwrap 可以保留
   - 但建议使用 `.expect("描述性消息")`

### 替换规则

```rust
// 规则 1: 用 ? 操作符替换
// 改前
let result = operation().unwrap();
// 改后
let result = operation()?;

// 规则 2: 用 ok_or_else 替换 Option 的 unwrap
// 改前
let value = option.unwrap();
// 改后
let value = option.ok_or_else(|| SageError::other("描述信息"))?;

// 规则 3: 用 map_err 提供上下文
// 改前
let data = serde_json::from_str(&json).unwrap();
// 改后
let data = serde_json::from_str(&json)
    .map_err(|e| SageError::parse(format!("JSON 解析失败: {}", e)))?;

// 规则 4: 测试代码保留但改进消息
// 改前
let result = operation().unwrap();
// 改后
let result = operation().expect("operation should succeed in test context");
```

### 添加 Clippy 规则

在 `clippy.toml` 中添加：
```toml
# 禁止在非测试代码中使用 unwrap
disallowed-methods = [
    { path = "core::result::Result::unwrap", reason = "使用 ? 或 expect 替代" },
    { path = "core::option::Option::unwrap", reason = "使用 ok_or_else 替代" },
]
```

### 验证方法

```bash
# 搜索剩余的 unwrap（排除测试）
rg "\.unwrap\(\)" crates/ --type rust | grep -v "_test" | grep -v "#\[test\]" | wc -l

# 确保编译通过
cargo check
```

---

## 任务 3: 统一依赖和错误处理

### 3.1 统一依赖

#### 目标
将 `lazy_static` 替换为 `once_cell`，统一异步初始化模式。

#### 需要修改的文件

```bash
# 查找使用 lazy_static 的文件
rg "lazy_static" crates/ --type rust -l
```

#### 替换规则

```rust
// 改前
use lazy_static::lazy_static;
lazy_static! {
    static ref CONFIG: Arc<Config> = Arc::new(Config::default());
}

// 改后
use once_cell::sync::Lazy;
static CONFIG: Lazy<Arc<Config>> = Lazy::new(|| Arc::new(Config::default()));
```

#### 更新 Cargo.toml

```toml
# 删除
lazy_static = "1.4"

# 保留
once_cell = { workspace = true }
```

### 3.2 简化错误构造函数

#### 目标
使用 Builder 模式减少 `error.rs` 中的重复代码。

#### 位置
`crates/sage-core/src/error.rs`

#### 实现

```rust
// 添加 ErrorBuilder
pub struct SageErrorBuilder {
    kind: ErrorKind,
    message: String,
    context: Option<String>,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl SageErrorBuilder {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            context: None,
            source: None,
        }
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    pub fn with_source(mut self, source: impl std::error::Error + Send + Sync + 'static) -> Self {
        self.source = Some(Box::new(source));
        self
    }

    pub fn build(self) -> SageError {
        // 构建最终错误
    }
}

// 便捷方法
impl SageError {
    pub fn builder(kind: ErrorKind, message: impl Into<String>) -> SageErrorBuilder {
        SageErrorBuilder::new(kind, message)
    }
}
```

### 验证方法

```bash
# 确保没有 lazy_static 引用
rg "lazy_static" crates/ --type rust

# 确保编译通过
cargo check

# 运行测试
cargo test
```

---

## 执行顺序

1. **阶段 1**: 三个任务并行执行
2. **阶段 2**: 合并更改，运行完整测试
3. **阶段 3**: 代码审查和优化

## 成功标准

- [ ] 所有文件 ≤ 200 行
- [ ] unwrap/expect 减少 80% 以上
- [ ] 移除所有 lazy_static 依赖
- [ ] `cargo check` 通过
- [ ] `cargo test` 通过
- [ ] `cargo clippy` 无警告
