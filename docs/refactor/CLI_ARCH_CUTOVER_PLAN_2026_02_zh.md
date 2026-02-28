# Sage CLI 架构收敛重构方案（无向后兼容）

> 日期：2026-02-09
> 范围：`crates/sage-cli`（主）+ 少量与其交互的 `sage-core/sage-tools` 引用说明
> 原则：**不做向后兼容**，直接删除旧路径与不可达模块

---

## 1. 背景与目标

当前 `sage-cli` 同时存在多条执行链路和多套命令系统，导致：

- 同样的 CLI 参数在不同运行路径下行为不一致
- 有完整实现但不可达的命令家族（维护成本高、误导开发者）
- 执行器初始化流程重复（配置、工具、MCP、session、input）
- 源码文档与真实命令解析/路由不一致

本方案目标：

1. **保留唯一执行主干**：`router -> commands::unified_execute -> UnifiedExecutor`
2. **UI 只做前端层**：rnk 负责渲染/输入，不再单独维护执行器构建链路
3. **删除不可达命令家族与遗留执行代码**
4. **帮助文档与真实行为一致**

---

## 2. 已确认问题（带证据）

### 2.1 参数语义分裂（Critical）

- `task/-c/-r` 等仅在 `non_interactive` 分支传入 unified 执行：`crates/sage-cli/src/router.rs:95-110`
- TTY 默认直接进入 app 模式：`crates/sage-cli/src/router.rs:112-120`
- app 模式不接收 `Cli` 参数：`crates/sage-cli/src/app.rs:410-412`

### 2.2 双执行链路重复（Critical）

- unified 路径：信号处理、MCP、session recording、resume 等：`crates/sage-cli/src/commands/unified/execute.rs:24-159`
- UI 路径走 `executor_factory`，初始化能力不齐：`crates/sage-cli/src/executor_factory.rs:11-33`

### 2.3 不可达命令家族（Critical）

`args.rs` 仅暴露：`Config/Trajectory/Tools/Doctor/Status/Usage`：`crates/sage-cli/src/args.rs:82-128`
`router.rs` 仅路由这几类：`crates/sage-cli/src/router.rs:17-30`

但以下模块具备完整实现且不可达：

- `crates/sage-cli/src/commands/mcp/*`
- `crates/sage-cli/src/commands/plugin/*`
- `crates/sage-cli/src/commands/update.rs`
- `crates/sage-cli/src/commands/models.rs`
- `crates/sage-cli/src/commands/eval.rs`

### 2.4 文档与行为不一致（High）

- `main.rs` 仍描述 `sage interactive/run/unified`：`crates/sage-cli/src/main.rs:20-37`
- 实际无这些子命令：`crates/sage-cli/src/args.rs:82-128`

### 2.5 工具列表命令语义不实（High）

- `tools` 命令描述“all available tools”：`crates/sage-cli/src/args.rs:97-99`
- 实现为硬编码短列表：`crates/sage-cli/src/commands/tools.rs:12-25`
- 实际工具全集来自 `sage-tools`：`crates/sage-tools/src/lib.rs:75-78`

### 2.6 trajectory 命令占位（High）

- 路由可达：`crates/sage-cli/src/router.rs:133-141`
- 实现全部为“temporarily disabled”：`crates/sage-cli/src/commands/trajectory.rs:8-47`

### 2.7 slash 行为不一致（High）

- 非 UI 路径中 `/resume`、`/model` 给出警告而非执行：`crates/sage-cli/src/commands/unified/session.rs:45-58`
- UI 路径能执行 `/resume`、动态 switch model：`crates/sage-cli/src/ui/rnk_app/executor.rs:118-179`

### 2.8 `/doctor` 使用硬编码配置路径（Medium）

- `"sage_config.json"` 被硬编码调用：
  - `crates/sage-cli/src/commands/unified/session.rs:72`
  - `crates/sage-cli/src/ui/rnk_app/executor.rs:201`

### 2.9 UI InputChannel handler 生命周期问题（Medium）

- UI 创建 `InputChannel` 时丢弃 handle：`crates/sage-cli/src/ui/rnk_app/mod.rs:359-367`
- 执行期请求输入依赖 handler 响应：`crates/sage-core/src/input/channel.rs:141-181`

---

## 3. 目标架构（Cutover 后）

### 3.1 单一路由规则

- 所有主执行（TTY/非TTY、print、resume）都走：
  - `router::route_main -> commands::unified_execute`
- rnk UI 仅作为 unified 的一种展示/输入模式（`OutputMode::Rnk` + `UiContext`）

### 3.2 保留模块

- `crates/sage-cli/src/router.rs`
- `crates/sage-cli/src/commands/unified/*`
- `crates/sage-cli/src/ui/*`（仅 UI 层）
- `crates/sage-cli/src/commands/config.rs`
- `crates/sage-cli/src/commands/diagnostics/*`

### 3.3 删除模块（无兼容）

- `crates/sage-cli/src/commands/mcp/**`
- `crates/sage-cli/src/commands/plugin/**`
- `crates/sage-cli/src/commands/update.rs`
- `crates/sage-cli/src/commands/models.rs`
- `crates/sage-cli/src/commands/eval.rs`
- `crates/sage-cli/src/app.rs`（legacy loop / demo / wrapper）
- `crates/sage-cli/src/executor_factory.rs`
- `crates/sage-cli/src/progress.rs`

并清理相关 `mod/use/re-export`。

---

## 4. 完整任务分解（Implementation Tasks）

## Task A：文档先行（本文件）

- [x] 在 `docs/refactor` 写明确切换方案
- [x] 固定“无向后兼容”删除清单
- [x] 给出风险与验收清单

## Task B：执行链路收敛

- [x] 改 `router`：TTY 不再旁路到 `app::run_app_mode`，统一进入 `unified_execute`
- [x] 在 unified 中支持 rnk UI 入口（保持 `UiContext` 能力）
- [x] 删除 `app.rs` / `executor_factory.rs`

## Task C：命令面清理

- [x] 删除不可达命令家族文件
- [x] 清理 `commands/mod.rs` 的 dead helper（`not_implemented`）
- [x] 保留可达命令面并修正文案

## Task D：一致性修复

- [x] `/doctor` 改为走当前 config 上下文，不再硬编码
- [x] 非 UI / UI 的 `/resume` `/model` 行为统一（至少语义一致）
- [x] `tools` 命令改为动态来源（非硬编码）
- [x] `trajectory`：要么删除命令入口，要么恢复最小可用实现（本次倾向删除入口）

## Task E：文档同步

- [x] 修正 `main.rs` 顶部模式说明
- [x] 修正 `args.rs` long_about 示例

## Task F：验证

- [x] `cargo fmt --all`
- [x] `cargo check -p sage-cli`
- [x] `cargo build -p sage-cli`
- [x] 冒烟：`sage`, `sage -p "..."`, `sage -c`, `sage -r <id>`, `sage tools`, `sage doctor`

---

## 5. 风险清单

1. UI 切换到统一链路后，`InputChannel` 请求/响应可能阻塞
2. 删除 legacy 文件后，可能残留引用导致编译失败
3. slash 命令语义统一时，UI 当前行为可能变化
4. tools/trajectory 命令策略调整会改变用户可见行为（这是预期的非兼容变更）

---

## 6. 验收标准

- `sage-cli` 编译通过且无新增 warning
- 不再存在不可达命令家族源码
- `router` 到执行器只有一条主链路
- 文档/帮助与真实行为一致
- `docs/refactor` 中有完整可执行任务与验收项

---

## 7. 备注

本方案不讨论向后兼容，不保留过渡 alias/shim，不保留 deprecated wrapper。
目标是最短路径把 CLI 收敛为一个可维护架构。