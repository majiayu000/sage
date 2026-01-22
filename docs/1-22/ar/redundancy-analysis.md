# Sage 项目冗余设计分析报告

**日期**: 2026-01-22
**分析范围**: sage-core, sage-cli, sage-tools

---

## 概述

通过对 Sage 项目的代码分析，发现了多处冗余设计问题，主要集中在配置结构体、存储 trait、错误类型等方面。本报告按优先级（P0-P3）列出问题并提供重构建议。

---

## P0: 重复的 RetryConfig/RateLimitConfig（严重）

### 问题描述

项目中存在多个功能相似但定义不同的重试和限流配置结构体：

| 结构体名 | 位置 | 字段 |
|---------|------|------|
| `RetryConfig` | `recovery/retry.rs:15` | max_attempts, max_duration, retry_unknown, retry_on_messages, no_retry_on_messages |
| `RetryConfig` | `storage/config.rs:57` | max_retries, initial_delay, max_delay, backoff_multiplier |
| `RateLimiterConfig` | `recovery/rate_limiter/types.rs:7` | requests_per_second, burst_size, max_concurrent, blocking, max_wait |
| `RateLimitConfig` | `llm/rate_limiter/types.rs:7` | requests_per_minute, burst_size, max_concurrent, enabled |
| `RateLimitConfig` | `config/provider/resilience.rs:38` | requests_per_minute, tokens_per_minute, max_concurrent_requests |

### 影响

- 维护困难：修改重试逻辑需要在多处同步
- 行为不一致：不同模块的重试行为可能不同
- 代码膨胀：重复的 Default 实现和 builder 方法

### 建议方案

1. 统一 `RetryConfig` 到 `recovery/retry.rs`，合并所有字段
2. 统一 `RateLimitConfig` 到 `recovery/rate_limiter/types.rs`
3. 其他模块通过 re-export 或类型别名使用

---

## P1: 重复的 Storage Trait（中等）

### 问题描述

项目定义了 5 个几乎相同的存储 trait：

```rust
// session/storage.rs:19
pub trait SessionStorage: Send + Sync {
    async fn save(&self, session: &Session) -> SageResult<()>;
    async fn load(&self, id: &SessionId) -> SageResult<Option<Session>>;
    async fn delete(&self, id: &SessionId) -> SageResult<()>;
    async fn list(&self) -> SageResult<Vec<SessionSummary>>;
    async fn exists(&self, id: &SessionId) -> SageResult<bool>;
}

// cache/storage.rs:16
pub trait CacheStorage: Send + Sync {
    async fn get(&self, key: &CacheKey) -> SageResult<Option<CacheEntry>>;
    async fn set(&self, key: CacheKey, entry: CacheEntry) -> SageResult<()>;
    async fn remove(&self, key: &CacheKey) -> SageResult<()>;
    async fn clear(&self) -> SageResult<()>;
}

// checkpoints/storage/mod.rs:22
pub trait CheckpointStorage: Send + Sync {
    async fn save(&self, checkpoint: &Checkpoint) -> SageResult<()>;
    async fn load(&self, id: &CheckpointId) -> SageResult<Option<Checkpoint>>;
    async fn delete(&self, id: &CheckpointId) -> SageResult<()>;
    async fn list(&self) -> SageResult<Vec<CheckpointSummary>>;
    async fn exists(&self, id: &CheckpointId) -> SageResult<bool>;
}

// memory/storage/trait.rs:9
pub trait MemoryStorage: Send + Sync {
    async fn store(&self, memory: Memory) -> Result<MemoryId, MemoryStorageError>;
    async fn get(&self, id: &MemoryId) -> Result<Option<Memory>, MemoryStorageError>;
    async fn delete(&self, id: &MemoryId) -> Result<(), MemoryStorageError>;
}
```

### 影响

- 代码重复：每个模块都有类似的 trait 定义
- 实现重复：Memory/File 存储实现高度相似
- 测试重复：每个存储都需要类似的测试

### 建议方案

暂不重构，因为各 trait 有特定的方法（如 CheckpointStorage 的 store_content/load_content）。
但可以考虑提取通用的 `KeyValueStorage<K, V>` trait 作为基础。

---

## P1: 分散的 Error 类型（中等）

### 问题描述

项目有 19 个独立的 Error 枚举：

| Error 类型 | 位置 |
|-----------|------|
| `SageError` | `error/types.rs` (主错误) |
| `RateLimitError` | `recovery/rate_limiter/types.rs:98` |
| `CircuitBreakerError` | `recovery/circuit_breaker/types.rs` |
| `MemoryStorageError` | `memory/storage/error.rs` |
| `McpError` | `mcp/error.rs` |
| `LearningError` | `learning/engine/error.rs` |
| `BuilderError` | `builder/error.rs` |
| `LifecycleError` | `agent/lifecycle/error.rs` |
| `DatabaseError` | `storage/backend/types.rs` |
| 等... | |

### 影响

- 错误处理不一致：有些返回 `SageResult`，有些返回模块特定错误
- 错误转换繁琐：需要大量 `From` 实现
- 用户体验差：错误信息格式不统一

### 建议方案

1. 保留模块特定错误用于内部使用
2. 确保所有模块错误都能转换为 `SageError`
3. 统一公共 API 返回 `SageResult<T>`

---

## P2: 重复的 Memory/File Storage 实现（中等）

### 问题描述

项目有多个几乎相同的内存/磁盘存储实现：

| 类型 | 位置 | 模式 |
|------|------|------|
| `MemoryStorage` | `cache/storage.rs:37` | `Arc<Mutex<LruCache>>` |
| `MemorySessionStorage` | `session/storage.rs:169` | `Arc<RwLock<HashMap>>` |
| `MemoryCheckpointStorage` | `checkpoints/storage/memory_storage.rs` | `Arc<RwLock<HashMap>>` |
| `DiskStorage` | `cache/storage.rs:171` | File + Index |
| `FileSessionStorage` | `session/storage.rs:37` | JSON files |
| `FileCheckpointStorage` | `checkpoints/storage/file_storage.rs` | JSON files |

### 影响

- 代码重复：相似的锁模式、序列化逻辑
- 维护困难：bug 修复需要在多处同步

### 建议方案

暂不重构，因为各实现有特定需求（如 LruCache vs HashMap）。
可以考虑提取通用的序列化/反序列化工具函数。

---

## P3: Builder 模式重复（轻微）

### 问题描述

几乎每个 Config 都实现了相同模式的 builder 方法：

```rust
impl SomeConfig {
    pub fn with_field1(mut self, value: T1) -> Self {
        self.field1 = value;
        self
    }
    pub fn with_field2(mut self, value: T2) -> Self {
        self.field2 = value;
        self
    }
}
```

### 影响

- 样板代码多
- 容易遗漏某些字段的 builder 方法

### 建议方案

考虑使用 `derive_builder` crate 或自定义宏自动生成。
但由于影响较小，优先级最低。

---

## 重构计划

| 阶段 | 任务 | 预期效果 |
|------|------|----------|
| Phase 1 | 统一 RetryConfig | 消除 2 个重复定义 |
| Phase 2 | 统一 RateLimitConfig | 消除 3 个重复定义 |
| Phase 3 | 审查 Error 类型转换 | 确保错误处理一致 |
| Phase 4 | 评估 Storage 抽象 | 决定是否需要重构 |

---

## 附录：相关文件列表

### Config 文件
- `crates/sage-core/src/recovery/retry.rs`
- `crates/sage-core/src/recovery/backoff.rs`
- `crates/sage-core/src/recovery/rate_limiter/types.rs`
- `crates/sage-core/src/storage/config.rs`
- `crates/sage-core/src/llm/rate_limiter/types.rs`
- `crates/sage-core/src/config/provider/resilience.rs`

### Storage 文件
- `crates/sage-core/src/session/storage.rs`
- `crates/sage-core/src/cache/storage.rs`
- `crates/sage-core/src/checkpoints/storage/mod.rs`
- `crates/sage-core/src/memory/storage/trait.rs`

### Error 文件
- `crates/sage-core/src/error/types.rs`
- `crates/sage-core/src/recovery/rate_limiter/types.rs`
- `crates/sage-core/src/memory/storage/error.rs`
- `crates/sage-core/src/mcp/error.rs`
