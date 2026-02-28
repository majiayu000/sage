# 快速优化指南

本文档提供可以立即实施的快速优化方案，每个都能在 1-2 天内完成。

## 🚀 快速修复 #1: 修复重复类型定义

### 问题
9 个类型在多个位置重复定义，违反 DRY 原则。

### 解决方案

#### 1. LspTool 重复

**当前状态:**
```rust
// crates/sage-tools/src/tools/code_intelligence/lsp/mod.rs:26
pub struct LspTool { /* ... */ }

// crates/sage-tools/src/tools/diagnostics/lsp.rs:40
pub struct LspTool { /* ... */ }
```

**修复方案 A: 重命名区分**
```rust
// crates/sage-tools/src/tools/code_intelligence/lsp/mod.rs
pub struct CodeIntelligenceLspTool { /* ... */ }

// crates/sage-tools/src/tools/diagnostics/lsp.rs
pub struct DiagnosticsLspTool { /* ... */ }
```

**修复方案 B: 合并为一个（如果功能相同）**
```rust
// crates/sage-tools/src/tools/lsp/mod.rs (新位置)
pub struct LspTool { /* ... */ }

// 在原位置重新导出
// crates/sage-tools/src/tools/code_intelligence/lsp/mod.rs
pub use crate::tools::lsp::LspTool;

// crates/sage-tools/src/tools/diagnostics/lsp.rs
pub use crate::tools::lsp::LspTool;
```

#### 2. RateLimiter 重复

**当前状态:**
```rust
// crates/sage-core/src/llm/rate_limiter/bucket.rs:23
pub struct RateLimiter { /* LLM 限流 */ }

// crates/sage-core/src/recovery/rate_limiter/limiter.rs:11
pub struct RateLimiter { /* 恢复限流 */ }
```

**修复方案: 重命名区分语义**
```rust
// crates/sage-core/src/llm/rate_limiter/bucket.rs
pub struct LlmRateLimiter {
    // LLM 请求限流逻辑
}

// crates/sage-core/src/recovery/rate_limiter/limiter.rs
pub struct RecoveryRateLimiter {
    // 恢复重试限流逻辑
}
```

#### 3. Session 重复

**当前状态:**
```rust
// crates/sage-core/src/session/types/session.rs:20
pub struct Session { /* 完整会话 */ }

// crates/sage-core/src/session/types/unified/header.rs:162
pub struct Session { /* 会话头 */ }
```

**修复方案: 重命名区分**
```rust
// crates/sage-core/src/session/types/session.rs
pub struct Session { /* 保持不变 */ }

// crates/sage-core/src/session/types/unified/header.rs
pub struct SessionHeader {
    // 会话头信息
}
```

### 实施步骤

```bash
# 1. 创建新分支
git checkout -b fix/duplicate-types

# 2. 重命名类型（使用 rust-analyzer 的重命名功能）
# 或手动查找替换

# 3. 运行测试确保没有破坏
cargo test

# 4. 运行 VibeGuard 验证
make guard

# 5. 提交
git add .
git commit -m "fix: resolve duplicate type definitions (RS-05)"
```

**预计时间:** 2-3 小时  
**风险:** 低（编译器会捕获所有错误）  
**收益:** 消除维护混淆，提高代码清晰度

---

## 🚀 快速修复 #2: 修复测试中的编译错误

### 问题
`CompactResult` 结构体添加了新字段，但测试未更新。

### 解决方案

```rust
// crates/sage-core/src/context/auto_compact/tests.rs
#[test]
fn test_compact_result_metrics() {
    let result = CompactResult {
        was_compacted: true,
        messages_before: 100,
        messages_after: 20,
        tokens_before: 50000,
        tokens_after: 10000,
        messages_compacted: 80,
        compacted_at: Some(Utc::now()),
        summary_preview: Some("Test summary...".to_string()),
        compact_id: Some(Uuid::new_v4()),
        // 添加缺失的字段
        boundary_message: None,
        summary_message: None,
        messages_to_keep: None,
    };

    assert_eq!(result.tokens_saved(), 40000);
    assert!((result.compression_ratio() - 0.2).abs() < 0.01);
}
```

**更好的方案: 使用构建器模式**
```rust
impl CompactResult {
    pub fn builder() -> CompactResultBuilder {
        CompactResultBuilder::default()
    }
}

// 测试中使用
let result = CompactResult::builder()
    .was_compacted(true)
    .messages_before(100)
    .messages_after(20)
    .build();
```

**预计时间:** 30 分钟  
**风险:** 无  
**收益:** 修复编译错误

---

## 🚀 快速修复 #3: 减少简单的 unwrap

### 问题
配置加载中有多个 unwrap 调用。

### 解决方案

**之前:**
```rust
// crates/sage-core/src/settings/locations.rs
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap()  // ❌ 可能 panic
        .join("sage")
}
```

**之后:**
```rust
use anyhow::{Context, Result};

pub fn config_dir() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .context("Failed to determine config directory")?;
    Ok(dir.join("sage"))
}

// 或提供默认值
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from(".sage"))
        .join("sage")
}
```

**批量查找和修复:**
```bash
# 查找所有非测试代码中的 unwrap
rg "\.unwrap\(\)" --type rust \
   --glob '!**/tests/**' \
   --glob '!**/test.rs' \
   --glob '!**/*_test.rs' \
   crates/sage-core/src/

# 优先修复这些高风险区域:
# - config/
# - settings/
# - session/manager.rs
# - tools/background_registry.rs
```

**预计时间:** 4-6 小时  
**风险:** 低  
**收益:** 显著提高稳定性

---

## 🚀 快速修复 #4: 添加 .vibeguard-duplicate-types-allowlist

### 问题
文档示例中的类型重复触发警告。

### 解决方案

创建允许列表文件：

```bash
# .vibeguard-duplicate-types-allowlist
# 文档示例中的类型（不是真实代码）
ProviderConfig
TimeoutConfig
```

**预计时间:** 5 分钟  
**风险:** 无  
**收益:** 清理 VibeGuard 输出

---

## 🚀 快速修复 #5: 优化简单的克隆

### 问题
函数参数不必要地获取所有权。

### 解决方案

**模式 1: 字符串参数**
```rust
// 之前
pub fn process_message(msg: String) {
    println!("{}", msg);
}

// 调用时
process_message(msg.clone()); // ❌ 不必要的克隆

// 之后
pub fn process_message(msg: &str) {
    println!("{}", msg);
}

// 调用时
process_message(&msg); // ✅ 无克隆
```

**模式 2: 配置对象**
```rust
// 之前
pub fn validate_config(config: Config) -> Result<()> {
    // 只读取，不修改
}

// 之后
pub fn validate_config(config: &Config) -> Result<()> {
    // 只读取，不修改
}
```

**查找候选项:**
```bash
# 查找接受 String 参数的函数
rg "fn \w+\([^)]*: String" --type rust crates/sage-core/src/

# 查找接受值参数的函数
rg "fn \w+\([^)]*: \w+Config\)" --type rust crates/sage-core/src/
```

**预计时间:** 2-3 小时  
**风险:** 低（编译器会指导修复）  
**收益:** 减少内存分配

---

## 🚀 快速修复 #6: 拆分一个大文件

### 示例: 拆分 terraform.rs (479 行)

**步骤 1: 分析文件结构**
```bash
# 查看文件中的主要组件
rg "^(pub )?struct|^(pub )?enum|^(pub )?fn" \
   crates/sage-tools/src/tools/infrastructure/terraform.rs
```

**步骤 2: 创建子模块结构**
```bash
mkdir -p crates/sage-tools/src/tools/infrastructure/terraform
```

**步骤 3: 拆分文件**
```rust
// terraform/mod.rs (主入口)
mod commands;
mod state;
mod validation;
mod types;

pub use commands::*;
pub use state::*;
pub use types::*;

// terraform/commands.rs
// 移动所有命令执行相关代码

// terraform/state.rs
// 移动状态管理代码

// terraform/validation.rs
// 移动验证逻辑

// terraform/types.rs
// 移动类型定义
```

**步骤 4: 验证**
```bash
cargo build
cargo test
```

**预计时间:** 2-3 小时  
**风险:** 低  
**收益:** 提高可维护性

---

## 🚀 快速修复 #7: 添加基础集成测试

### 创建第一个集成测试

```rust
// crates/sage-core/tests/agent_basic_test.rs
use sage_core::{
    agent::UnifiedAgent,
    config::SageConfig,
};

#[tokio::test]
async fn test_agent_initialization() {
    let config = SageConfig::default();
    let agent = UnifiedAgent::new(config).await;
    
    assert!(agent.is_ok());
}

#[tokio::test]
async fn test_agent_simple_task() {
    let config = SageConfig::default();
    let mut agent = UnifiedAgent::new(config).await.unwrap();
    
    let result = agent.execute("echo 'hello'").await;
    
    assert!(result.is_ok());
}
```

**预计时间:** 1-2 小时  
**风险:** 无  
**收益:** 开始建立测试基础

---

## 🚀 快速修复 #8: 优化一个嵌套锁

### 示例: 简化 signal_handler.rs

**之前 (4 次锁获取):**
```rust
pub async fn start() {
    let state1 = app_state.lock().await;
    // 使用 state1
    
    let state2 = config.lock().await;
    // 使用 state2
    
    let state3 = session.lock().await;
    // 使用 state3
    
    let state4 = registry.lock().await;
    // 使用 state4
}
```

**之后 (克隆后释放锁):**
```rust
pub async fn start() {
    // 快速获取所需数据
    let (state_data, config_data, session_data, registry_data) = {
        let s1 = app_state.lock().await;
        let s2 = config.lock().await;
        let s3 = session.lock().await;
        let s4 = registry.lock().await;
        (s1.clone(), s2.clone(), s3.clone(), s4.clone())
    }; // 所有锁在这里释放
    
    // 使用克隆的数据，无锁
    process(&state_data, &config_data, &session_data, &registry_data);
}
```

**预计时间:** 1-2 小时  
**风险:** 低  
**收益:** 减少锁竞争

---

## 📋 快速修复检查清单

### 第 1 天
- [ ] 修复 CompactResult 测试编译错误 (30 分钟)
- [ ] 创建 .vibeguard-duplicate-types-allowlist (5 分钟)
- [ ] 修复 3-5 个简单的 unwrap (2 小时)
- [ ] 优化 5-10 个不必要的克隆 (2 小时)

### 第 2 天
- [ ] 重命名 2-3 个重复类型 (3 小时)
- [ ] 拆分 1 个大文件 (2 小时)
- [ ] 添加 2-3 个基础集成测试 (2 小时)

### 第 3 天
- [ ] 优化 2-3 个嵌套锁模式 (3 小时)
- [ ] 运行完整测试套件 (30 分钟)
- [ ] 更新文档 (1 小时)

**总预计时间:** 2-3 天  
**预期收益:**
- ✅ 修复所有编译错误
- ✅ 减少 20-30 个 unwrap 调用
- ✅ 减少 10-20 个不必要的克隆
- ✅ 解决 3-5 个重复类型
- ✅ 拆分 1-2 个大文件
- ✅ 添加 5-10 个新测试

---

## 🛠️ 自动化脚本

### 查找优化机会

```bash
#!/bin/bash
# scripts/find-optimization-opportunities.sh

echo "=== 查找不必要的克隆 ==="
rg "\.clone\(\)" --type rust crates/sage-core/src/ | wc -l

echo "=== 查找 unwrap 调用 ==="
rg "\.unwrap\(\)|\.expect\(" --type rust \
   --glob '!**/tests/**' \
   crates/sage-core/src/ | wc -l

echo "=== 查找大文件 (>450 行) ==="
find crates -name "*.rs" -type f ! -path "*/tests/*" \
   -exec wc -l {} + | sort -rn | head -20

echo "=== 查找嵌套锁 ==="
rg "\.lock\(\)|\.read\(\)|\.write\(\)" --type rust \
   crates/sage-core/src/ -A 10 | \
   rg "\.lock\(\)|\.read\(\)|\.write\(\)" | wc -l
```

### 运行所有快速检查

```bash
#!/bin/bash
# scripts/quick-check.sh

set -e

echo "🔍 运行快速检查..."

echo "1. 编译检查..."
cargo check --all-targets

echo "2. Clippy 检查..."
cargo clippy -- -D warnings

echo "3. VibeGuard 检查..."
make guard

echo "4. 测试..."
cargo test --lib

echo "✅ 所有检查通过!"
```

---

## 📚 相关资源

- [主优化报告](../OPTIMIZATION_RECOMMENDATIONS.md)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)

---

*最后更新: 2026-02-23*
