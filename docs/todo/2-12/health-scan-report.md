# 代码库健康诊断报告 2026-02-12

> 由 `/auto-optimize self scan` 生成

---

## 健康指标

| 指标 | 当前值 | 阈值 | 状态 |
|------|--------|------|------|
| 类型重复率 | 0/总类型数 | 0 | ✅ |
| 文件膨胀率 | 306/1009 (30%) | ≤5% | ❌ |
| Clippy 警告 | 98 errors (75 dead_code + 1 large_enum_variant + 级联) | 0 | ❌ |
| 测试通过率 | sage-tools 361/361, sage-sdk 11/11 | 100% | ⚠️ (workspace 级别有预存错误) |
| `as` 窄化转换 | 26 处 | 0 | ❌ |
| `tokio::spawn` 无 JoinHandle | 4 处 | 0 | ❌ |
| 非测试代码 unwrap() | 796 处 | ≤100 | ❌ |
| `#[allow(dead_code)]` | 73 处 | 0 | ⚠️ |

---

## P0 — 立即修复（安全/正确性风险）

### P0-1: `f64 as i64` 无 NaN/Infinity 检查
- **位置**: `crates/sage-core/src/telemetry/metrics/gauge.rs:29,40,51`
- **问题**: `(value * self.scale) as i64` — 当 value 为 NaN 或 Infinity 时行为未定义
- **修复**: 添加 `is_finite()` 检查，无效值返回默认值 0

### P0-2: RwLock poison unwrap（UI 桥接层）
- **位置**: `crates/sage-core/src/ui/bridge/adapter.rs:21,52,154,163`
- **问题**: `.write().unwrap()` / `.read().unwrap()` — 任何 panic 导致 RwLock 中毒后级联崩溃
- **修复**: 使用 `.write().unwrap_or_else(|e| e.into_inner())` 或返回 Result

### P0-3: 整数溢出导致 backoff 缩小
- **位置**: `crates/sage-core/src/recovery/backoff.rs:146`
- **问题**: `attempt as i32` — 当 attempt > i32::MAX 时溢出，`base.powi(attempt as i32)` 结果错误，backoff 时间可能缩小而非增长
- **修复**: 使用 `i32::try_from(attempt).unwrap_or(i32::MAX)` 并 clamp 结果

### P0-4: restart_count 溢出 + Infinity 风险
- **位置**: `crates/sage-core/src/recovery/supervisor/task_supervisor.rs:205`
- **问题**: `restart_count as i32` 窄化转换 + `powi` 可能产生 Infinity
- **修复**: 同 P0-3，使用 `try_from` + `is_finite()` 检查

### P0-5: 全局配置 RwLock poison unwrap
- **位置**: `crates/sage-tools/src/config.rs:253,261`
- **问题**: `GLOBAL_CONFIG.read().unwrap()` — 与 P0-2 相同模式
- **修复**: 使用 poison recovery 或返回 Result

### P0-6: Bash 工具测试 schema name 大小写不匹配
- **位置**: `crates/sage-tools/src/tools/process/bash/mod.rs:312`
- **问题**: 测试断言 `"bash"` 但 `name()` 返回 `"Bash"`，测试实际会失败
- **修复**: 修正断言为 `"Bash"` 或修正 `name()` 返回值

---

## P1 — 本轮修复（代码质量/资源泄漏）

### P1-1: `as` 窄化转换（26 处）
- **位置**: 分布在 sage-core 和 sage-tools 中
- **典型**: `pid as i32`, `len as u32`, `size as i64`
- **修复**: 逐一替换为 `TryFrom`/`try_from`，违反 CLAUDE.md NEVER 规则

### P1-2: tokio::spawn 无 JoinHandle（4 处）
- **位置**:
  - `crates/sage-cli/src/ui/rnk_app/mod.rs:368,378,394` — TUI 核心任务（tick、event、render）
  - `crates/sage-core/src/agent/subagent/executor/background.rs:24`
- **问题**: fire-and-forget spawn，资源泄漏 + panic 静默吞没
- **修复**: 存储 JoinHandle，在 Drop 中 abort

### P1-3: sage-eval crate 未加入 workspace
- **位置**: `crates/sage-eval/` 存在但未在根 `Cargo.toml` workspace members 中声明
- **问题**: 不参与 `cargo test --workspace`，可能有未发现的编译错误
- **修复**: 加入 workspace 或删除

### P1-4: 幽灵模块 auth/ 和 ide/（~46KB 从未编译）
- **位置**:
  - `crates/sage-core/src/auth/` — 认证模块，未在 `lib.rs` 中声明
  - `crates/sage-core/src/ide/` — IDE 集成模块，未在 `lib.rs` 中声明
- **问题**: 约 46KB 代码从未被编译或测试，属于死代码
- **修复**: 如果计划使用则在 lib.rs 中声明并修复编译错误；否则删除

### P1-5: sandbox/executor/builder.rs 整个模块死代码
- **位置**: `crates/sage-core/src/sandbox/executor/builder.rs` (96 行)
- **问题**: 整个 builder 模块未被任何代码引用
- **修复**: 删除或集成到 sandbox 执行流程中

### P1-6: large_enum_variant clippy 错误
- **位置**: `crates/sage-core/src/session/types/unified.rs:~line 1`
- **问题**: `SessionRecordPayload` 枚举变体大小差异过大（488 bytes），触发 clippy 错误
- **修复**: 对大变体使用 `Box<SessionMessage>`

### P1-7: 75 个 dead_code clippy 警告
- **位置**: 分布在 sage-core 各模块
- **问题**: 大量未使用的 pub 函数/结构体，触发 clippy dead_code lint
- **修复**: 删除确认无用的代码，或标记为 `pub(crate)` 降低可见性

---

## P2 — 下轮修复（技术债务/可维护性）

### P2-1: 预存测试编译错误（101+ errors）
- **位置**:
  - `crates/sage-core/src/agent/subagent/registry/tests.rs`
  - `crates/sage-core/src/agent/subagent/executor/tests.rs`
  - `examples/builtin_agents_demo.rs`
- **问题**: `.await` 调用在非 async 方法上，101 个编译错误
- **说明**: 这些错误早于当前工作，但阻止了 workspace 级别的测试

### P2-2: 文件膨胀（306 个文件超 200 行）
- **严重程度**: 30% 的文件超过 200 行阈值
- **典型大文件**:
  - `unified.rs` (899 行)
  - 多个 `mod.rs` 超过 500 行
- **修复**: 按职责拆分为子模块，目标每文件 ≤200 行

### P2-3: 非测试代码 unwrap() 过多（796 处）
- **问题**: 生产代码中大量 `.unwrap()` 调用，任何一处 panic 都会导致进程崩溃
- **修复**: 分批替换为 `?` 操作符或 `unwrap_or_else`，优先处理 IO/网络相关路径

### P2-4: `#[allow(dead_code)]` 标记过多（73 处）
- **问题**: 大量代码被标记为允许死代码，掩盖了真正的未使用代码
- **修复**: 逐一审查，删除确认无用的代码，移除不必要的 allow 标记

### P2-5: input/mod.rs 测试中缺少变量定义
- **位置**: `crates/sage-core/src/input/mod.rs:103`
- **问题**: `test_non_interactive_channel` 中使用了未定义的 `request` 变量
- **修复**: 添加 `let request = InputRequest::simple("Question?");`

---

## 建议修复顺序

### Round 1: P0 安全修复（预计 30 分钟）
1. P0-1 gauge.rs f64 转换
2. P0-3 + P0-4 backoff/supervisor 整数溢出
3. P0-2 + P0-5 RwLock poison 处理
4. P0-6 Bash 测试修复
5. 运行 `cargo test --lib -p sage-tools -p sage-sdk` 验证

### Round 2: P1 代码质量（预计 2 小时）
1. P1-6 Box large enum variant
2. P1-7 清理 dead_code（解决 75 个 clippy 错误）
3. P1-1 替换 26 处 `as` 窄化转换
4. P1-2 修复 4 处 tokio::spawn 泄漏
5. P1-4 处理幽灵模块（删除或激活）
6. P1-5 删除 sandbox builder 死代码
7. P1-3 处理 sage-eval
8. 运行 `cargo clippy --all-targets -- -D warnings` 验证零警告

### Round 3: P2 技术债务（预计 3 小时）
1. P2-1 修复预存测试编译错误
2. P2-5 修复 input 测试
3. P2-2 拆分最大的 10 个文件
4. P2-3 批量替换高风险路径的 unwrap()
5. P2-4 审查 allow(dead_code) 标记
6. 运行 `cargo test --lib --workspace` 验证全量通过

---

## 附录：扫描方法

本报告由 `auto-optimize` skill 的 Phase 1 (SCAN) + Phase 2 (DIAGNOSE) 生成：

- **自动化检测**: clippy、类型重复 grep、文件行数统计、禁止模式 grep
- **AI 深度扫描**: 3 个并行子 agent（结构分析、语义重复、模式违规）
- **手动验证**: 对 AI 发现的问题进行代码级确认

### Layer 0 类型系统评估

类型重复率为 0%，说明 `crate::types/` 的 Layer 0 策略运作良好。CLAUDE.md 中的 NEVER 规则有效防止了跨模块类型重复。

### 与上次扫描对比

| 指标 | 2026-02-08 | 2026-02-12 | 趋势 |
|------|-----------|-----------|------|
| Clippy 警告 | 0 | 98 | ⬆️ 恶化（新增 dead_code lint） |
| 类型重复 | 0 | 0 | ➡️ 稳定 |
| sage-tools 测试 | 361 pass | 361 pass | ➡️ 稳定 |
| sage-sdk 测试 | 11 pass | 11 pass | ➡️ 稳定 |
