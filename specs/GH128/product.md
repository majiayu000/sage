# Product Spec

## Linked Issue

GH-128

## 用户问题

sage-sdk 公共 API 暴露内部 `Config`，并同时提供 `RunOptions` / `UnifiedRunOptions` 和两组执行入口。内部配置字段变化会变成 SDK 破坏性变更，选项类型会漂移，文档里的 deprecation policy 又和仓库 No Backward Compatibility 规则冲突。

## 目标

- `ExecutionResult` 只暴露 SDK 自有的窄配置视图，不直接公开 `sage_core::config::Config`。
- SDK 只保留一个运行选项类型和一族执行方法。
- SDK 文档与仓库破坏性变更策略一致，不承诺 deprecated API 维护周期。

## 非目标

- 不改变 SDK 的执行语义与默认行为。
- 不新增 provider 配置能力。
- 不引入 deprecated alias 或兼容 shim。

## Behavior Invariants

1. SDK 用户无法通过 `ExecutionResult` 直接读取内部完整 `Config` 或凭据承载结构。
2. 运行选项只存在一个 public 类型，包含 working_directory、max_steps、metadata、non_interactive 等必要字段。
3. 执行入口只保留一族，内部统一驱动 `UnifiedExecutor`。
4. 删除的 public API 不用 `#[deprecated]` 包装；按仓库规则直接更新引用和版本说明。
5. 文档不再声明维护 deprecated APIs 跨 MINOR 兼容。

## 验收标准

- [ ] `ExecutionResult` 不再有 `pub config_used: Config`。
- [ ] 只保留一个 RunOptions 类型与一族执行方法。
- [ ] `lib.rs` / `version.rs` 的弃用政策与 No Backward Compatibility 一致。
- [ ] SemVer/CHANGELOG 记录破坏性 SDK API 调整。

## 边界情况

- 用户仍需要知道实际 provider/model：窄视图可暴露 provider id、model id、working_dir、status，但不能暴露 secret-bearing config。
- metadata 需要保持可传递到执行层。
- non_interactive 默认值保持 false，避免默认行为变化。

## 发布说明

SDK public API 破坏性收敛：移除重复 options/入口和 deprecated policy，`ExecutionResult` 改为窄化配置摘要。

## 开放问题

- 版本号升 MINOR 还是 MAJOR？按当前 crate 0.x 语义，建议在 PR 中明确记录为 breaking minor 或直接按维护者版本策略更新。
