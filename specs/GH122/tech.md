# Tech Spec

## Linked Issue

GH-122

## Product Spec

`specs/GH122/product.md`

## Codebase Context

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| 权限缓存持久化 | `crates/sage-core/src/tools/permission/cache.rs` | `load_from_file(...).unwrap_or_default()` 后 `save_to_file` 覆盖 | 静默清空规则集的根因 |
| /init 命令 | `crates/sage-core/src/commands/executor/handlers/basic.rs` | `let _ = fs::write(...)` 后固定返回成功文案 | 谎报成功 |

## 设计方案

1. `persist_decision`：文件存在时改用 `load_from_file(settings_path)?`，错误路径先 `tracing::error!`（含路径与原因）再传播；仅当文件不存在时才 `Settings::default()`。
2. `execute_init`：`fs::write` 失败时提前返回 `CommandResult::local("Failed to write .sage/settings.json: {e}")`。

注：`set_with_persistence` 当前无生产调用方（PermissionCache 属于未接线的 parallel_executor 栈，由 GH-125 统一处置）；本修复收紧公共 API 语义并加回归测试，防止未来接线时继承数据丢失缺陷。

## Product-to-Test Mapping

| Product invariant | Implementation area | Verification |
| --- | --- | --- |
| P1 | `persist_decision` 错误传播 | `test_persist_decision_preserves_corrupted_settings_file` |
| P2 | 文件缺失分支 | `test_persist_decision_creates_settings_when_missing` |
| P3 | session cache 先行写入 | 同 P1 测试断言 `cache.get(...) == Some(true)` |
| P4 | `execute_init` 错误分支 | 代码审查 + 既有 /init 测试回归 |

## 数据流

输入：权限 key、allow/deny 布尔。持久化目标：`.sage/settings.local.json`（loader 原子写由 SettingsLoader 负责）。

## 备选方案

- 解析失败时备份原文件再重建（`settings.local.json.bak`）：多一层状态，且用户无感知时仍是静默降级，拒绝。

## 风险

- Security: 行为只收紧（拒绝覆盖），无新面。
- Compatibility: `SageResult` 签名未变，仅错误路径语义变化。
- Performance: 无。
- Maintenance: 注释解释了为什么不允许 unwrap_or_default。

## 测试计划

- [ ] Unit tests: 上表两个新测试 + 既有 13 个 cache 测试回归。
- [ ] Integration tests: workspace 测试回归。
- [ ] Manual verification: 无需（无可达 UI 路径）。

## 回滚方案

revert 单一 commit。
