# Sage Storage Trait 统一性分析报告

**日期**: 2026-01-22
**结论**: 不建议统一

---

## 1. 各 Trait 方法对比表

### 1.1 Trait 概览

| Trait | 文件位置 | 主要用途 |
|-------|---------|---------|
| `SessionStorage` | `session/storage.rs:19` | 会话持久化 |
| `CacheStorage` | `cache/storage.rs:16` | 缓存存储 |
| `CheckpointStorage` | `checkpoints/storage/mod.rs:22` | 检查点存储 |
| `MemoryStorage` | `memory/storage/trait.rs:9` | 记忆存储 |
| `DatabaseBackend` | `storage/backend/trait.rs:11` | 数据库后端 |

### 1.2 方法签名详细对比

| 操作类型 | SessionStorage | CacheStorage | CheckpointStorage | MemoryStorage | DatabaseBackend |
|---------|---------------|--------------|-------------------|---------------|-----------------|
| **保存/写入** | `save(&Session)` | `set(CacheKey, CacheEntry)` | `save(&Checkpoint)` | `store(Memory) -> MemoryId` | `execute(sql, params)` |
| **读取** | `load(&SessionId) -> Option<Session>` | `get(&CacheKey) -> Option<CacheEntry>` | `load(&CheckpointId) -> Option<Checkpoint>` | `get(&MemoryId) -> Option<Memory>` | `query(sql, params)` |
| **删除** | `delete(&SessionId)` | `remove(&CacheKey)` | `delete(&CheckpointId)` | `delete(&MemoryId)` | N/A (via execute) |
| **存在检查** | `exists(&SessionId) -> bool` | N/A | `exists(&CheckpointId) -> bool` | N/A | N/A |
| **列表** | `list() -> Vec<SessionSummary>` | N/A | `list() -> Vec<CheckpointSummary>` | `list(offset, limit) -> Vec<Memory>` | N/A |
| **清空** | N/A | `clear()` | N/A | `clear()` | N/A |
| **统计** | N/A | `statistics() -> StorageStatistics` | N/A | `count() -> usize` | N/A |
| **特殊方法** | N/A | `cleanup_expired()` | `latest()`, `store_content()`, `load_content()` | `search(&MemoryQuery)`, `update(Memory)` | `ping()`, `transaction()`, `version()`, `is_connected()`, `close()`, `backend_type()` |

---

## 2. 分析结论：不建议统一

### 2.1 相似度评估

| 对比组合 | 共同方法数 | 总方法数 | 相似度 |
|---------|-----------|---------|--------|
| SessionStorage vs CheckpointStorage | 5 | 8 | 62.5% |
| SessionStorage vs MemoryStorage | 3 | 10 | 30% |
| SessionStorage vs CacheStorage | 2 | 9 | 22% |
| CacheStorage vs MemoryStorage | 3 | 11 | 27% |
| DatabaseBackend vs 其他 | 0 | - | 0% |

**平均相似度约为 28%，远低于 70% 的统一阈值。**

### 2.2 不建议统一的原因

1. **语义差异显著**: 各 trait 服务于完全不同的领域
2. **特殊方法无法泛化**: 占各 trait 方法的 30-60%
3. **错误类型不统一**: SageResult / MemoryStorageError / DatabaseError
4. **泛型 trait 复杂性代价高**: trait object 困难
5. **现有设计已经合理**: 遵循 Interface Segregation Principle

---

## 3. 替代建议

### 3.1 提取通用工具函数

```rust
// crates/sage-core/src/storage/utils.rs
pub async fn ensure_dir(path: &Path) -> SageResult<()> { ... }
pub async fn save_json<T: Serialize>(path: &Path, value: &T) -> SageResult<()> { ... }
pub async fn load_json<T: DeserializeOwned>(path: &Path) -> SageResult<Option<T>> { ... }
```

### 3.2 统一命名规范

| 当前命名 | 建议 |
|---------|------|
| `load` / `get` | 统一为 `get` |
| `remove` / `delete` | 统一为 `delete` |

---

## 4. 总结

| 评估维度 | 结论 |
|---------|------|
| 方法相似度 | 28%（低于 70% 阈值） |
| 语义一致性 | 低 |
| 特殊方法占比 | 30-60% |
| 统一收益 | 低 |
| 统一成本 | 高 |
| **最终建议** | **不统一** |
