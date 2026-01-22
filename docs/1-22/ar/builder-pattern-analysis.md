# Sage Builder Pattern 分析报告

**日期**: 2026-01-22
**结论**: 不建议引入 derive_builder

---

## 1. 统计数据

| 指标 | 数值 |
|------|------|
| Builder 方法总数 | 503 |
| 使用 builder 的结构体 | ~50+ |
| 主要模块分布 | config, recovery, session, tools, agent |

## 2. 模块分布

| 模块 | Builder 方法数 | 主要类型 |
|------|---------------|---------|
| `sage-core/config` | ~60 | `ProviderConfig`, `CredentialResolver` |
| `sage-core/recovery` | ~45 | `RetryConfig`, `CircuitBreaker`, `RateLimitConfig` |
| `sage-core/session` | ~40 | `SessionMetadata`, `BranchConfig` |
| `sage-core/tools` | ~50 | `ToolExecutor`, `ParallelExecutor` |
| `sage-core/agent` | ~35 | `ExecutionOptions`, `SubagentConfig` |
| 其他 | ~273 | 各种类型 |

## 3. 当前实现模式

```rust
impl RetryConfig {
    // 标准 builder 方法
    pub fn with_max_attempts(mut self, n: u32) -> Self {
        self.max_attempts = n;
        self
    }

    // 工厂方法 - derive_builder 无法生成
    pub fn for_storage() -> Self { ... }
    pub fn for_llm() -> Self { ... }
    pub fn aggressive() -> Self { ... }
}
```

## 4. 不建议引入 derive_builder 的原因

### 4.1 自定义逻辑无法自动生成

很多 builder 方法包含自定义逻辑：

```rust
pub fn with_tools(mut self, tools: Vec<ToolSchema>) -> Self {
    for tool in &tools {
        self.variables.add_tool(&tool.name);  // 自定义逻辑
    }
    self.tools = tools;
    self
}
```

### 4.2 工厂方法是核心 API

项目大量使用工厂方法，这是 derive_builder 无法提供的：

- `RetryConfig::for_storage()`
- `RetryConfig::for_llm()`
- `RateLimitConfig::for_provider("openai")`
- `CircuitBreakerConfig::aggressive()`

### 4.3 当前实现已经一致

- 命名规范：所有方法使用 `with_` 前缀
- 返回类型：始终返回 `Self`
- 所有权：使用 consuming `mut self` 模式
- 文档：builder 方法都有文档注释

### 4.4 依赖成本

- derive_builder 增加编译时间
- 增加学习成本
- 宏生成的代码难以调试

## 5. 替代建议

### 5.1 保持现状

当前实现是惯用的 Rust 风格，无需改变。

### 5.2 如果需要减少样板代码

可以考虑创建项目内部的简单宏：

```rust
macro_rules! builder_field {
    ($name:ident, $type:ty) => {
        pub fn $name(mut self, value: impl Into<$type>) -> Self {
            self.$name = value.into();
            self
        }
    };
}
```

但这不是必需的，因为当前代码量可控。

## 6. 总结

| 评估维度 | 结论 |
|---------|------|
| Builder 方法数量 | 503（较多但可控） |
| 自定义逻辑占比 | ~30%（无法自动生成） |
| 工厂方法依赖 | 高（核心 API） |
| 当前一致性 | 好 |
| derive_builder 收益 | 低 |
| derive_builder 成本 | 中（依赖、学习、调试） |
| **最终建议** | **不引入** |
