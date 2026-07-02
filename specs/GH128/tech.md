# Tech Spec

## Linked Issue

GH-128

## Product Spec

`specs/GH128/product.md`

## Codebase Context

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| result type | `crates/sage-sdk/src/client/result/core.rs` | `ExecutionResult` has `pub config_used: Config` | 暴露内部 Config 与潜在凭据结构 |
| options | `crates/sage-sdk/src/client/options/run_options.rs`，`unified_options.rs` | 两个类型字段几乎相同，`UnifiedRunOptions` 多 `non_interactive` | 漂移来源 |
| execution | `crates/sage-sdk/src/client/execution/run.rs`，`unified.rs` | `run*` 与 `execute_unified*` 都驱动 unified executor | public 入口重复 |
| docs/version | `crates/sage-sdk/src/lib.rs`，`version.rs`，root `CLAUDE.md` | SDK 文档承诺 deprecated API；仓库规则禁止 | 文档冲突 |

## 设计方案

1. **窄化结果视图**：新增 SDK 自有类型，例如 `ExecutionConfigSummary { provider: Option<String>, model: Option<String>, working_directory: Option<PathBuf>, max_steps: Option<u32>, non_interactive: bool }`。`ExecutionResult` 持有 `config_summary` 或 accessor，不再 public 暴露 `Config`。
2. **单一 options 类型**：保留 `RunOptions`，加入 `non_interactive: bool` 与 `with_non_interactive`。删除 `UnifiedRunOptions` public export、模块和示例引用。不得添加 deprecated alias。
3. **单一执行入口族**：保留 `run` / `run_with_options` 作为 public 高层入口；如需要 streaming/input handle，保留一个明确命名的 advanced 方法，但参数也使用 `RunOptions`。删除重复 `execute_unified*` public 入口或转为 crate-private helper。
4. **内部迁移**：所有 SDK call site 更新到 `RunOptions`。`ExecutionResult::new` 接受 outcome + summary，summary 在执行前从 safe config fields 构造。
5. **文档策略**：更新 `lib.rs` 与 `version.rs`，移除 deprecated maintenance 说明和 deprecation macro 示例。文档说明 0.x breaking changes 会直接移除旧 API，并在 changelog/release notes 记录。
6. **安全检查**：新增测试确认 `ExecutionConfigSummary` 不包含 API key/base_url header/secret-bearing provider params。

## Product-to-Test Mapping

| Product invariant | Implementation area | Verification |
| --- | --- | --- |
| P1 | `ExecutionResult` + summary type | compile test/search check 无 `pub config_used: Config`；summary 不含 secret fields |
| P2 | `RunOptions` | `UnifiedRunOptions` export 删除，builder tests 覆盖 non_interactive |
| P3 | execution module | public API examples 只用一族方法 |
| P4/P5 | docs/version | search check 无 deprecated policy 承诺/宏导出 |

## 数据流

SDK user -> `RunOptions` -> internal unified executor -> `ExecutionOutcome` -> safe summary from selected config/options -> `ExecutionResult { outcome, config_summary }`。

## 备选方案

- 保留 `UnifiedRunOptions` 作为 type alias：违反 No Backward Compatibility。
- 把 `config_used` 改成 private `Config` accessor：仍把内部类型绑定到 public API，不满足目标。

## 风险

- Compatibility: public API breaking，必须更新 examples/docs/tests 和版本说明。
- Security: summary 构造白名单字段，不能从 `Config` 序列化后删字段。
- Maintenance: SDK public exports 要和 docs/examples 同步。

## 测试计划

- [ ] Unit tests: `RunOptions::with_non_interactive`，summary white-list。
- [ ] Doc tests/examples: 只使用保留的 public API。
- [ ] Search checks: `UnifiedRunOptions`、`config_used: Config`、deprecated policy 文案清零。

## 回滚方案

回滚 SDK API 收敛会恢复重复入口；不得通过 deprecated alias 部分回滚。
