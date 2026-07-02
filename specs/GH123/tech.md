# Tech Spec

## Linked Issue

GH-123

## Product Spec

`specs/GH123/product.md`

## Codebase Context

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| CLI slash commands | `crates/sage-cli/src/commands/unified/slash_commands.rs` | 三个 `Err(_)` 分支静默丢弃错误并回退静态列表 | 唯一可达的实时模型拉取路径 |

## 设计方案

提取 `static_models` 闭包（静态回退列表）与 `fallback_on_error` 闭包（`tracing::warn!` 后回退），三个 provider 分支的 `Err(e)` 统一走 `fallback_on_error(&e)`，未知 provider 分支直接走 `static_models()`。不改变返回类型与控制流。

## Product-to-Test Mapping

| Product invariant | Implementation area | Verification |
| --- | --- | --- |
| P1 | `fallback_on_error` 闭包 | 代码审查 + clippy（tracing 宏静态检查）；行为为日志副作用，无单测 |
| P2/P3 | match 分支结构不变 | `cargo test -p sage-cli`、`cargo check --workspace` |

## 数据流

输入：provider 名、base_url、api_key。输出：模型 id 列表；新增 stderr/log 输出（warn）。无持久化。

## 备选方案

- 在 UI 输出「使用静态回退列表」提示：更重的 UX 变更，留给 GH-124 一并设计。

## 风险

- Security: 日志仅含 provider 名与错误 Display，不含 api_key。
- Compatibility: 无 API 变更。
- Performance: 无。
- Maintenance: 闭包消除了三份重复回退代码。

## 测试计划

- [ ] Unit tests: 无新增（纯日志副作用）。
- [ ] Integration tests: 既有 sage-cli 测试回归。
- [ ] Manual verification: 断网执行 `/model`，观察 warn 日志与静态列表。

## 回滚方案

revert 单一 commit 即可。
