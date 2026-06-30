# Product Spec

## Linked Issue

GH-91

## 用户问题

Sage release 需要在发布前证明 source、tag、Cargo versions、changelog、artifacts、checksums、install smoke 和安全检查都一致。当前缺少跨平台支持矩阵和供应链 gate，容易出现 tag/version 不一致、artifact 不可安装或 unsupported platform 暗中失败。

## 目标

- 定义跨平台支持矩阵，明确支持或不支持的平台。
- 在 release 前校验 tag、workspace/package version 和 changelog entry。
- 生成 artifact checksum/signing contract，并对发布产物做 install smoke。
- 将 audit、deny、MSRV、format、clippy、test 和 doc consistency 纳入 required gate。
- 对 action pinning 或等价供应链策略作出明确要求。

## 非目标

- 不把所有慢测试放进最短 PR path。
- 不把 unsupported platform failure 当作隐式支持。
- 不在本 PR 实现发布自动化行为。
- 不包含桌面 app、IDE 入口或 app-server client。

## Behavior Invariants

1. Release tag 必须与 workspace/package version 一致。
2. Changelog 必须有对应 release entry。
3. Release artifacts 必须有 checksum manifest，并通过 unpack/install smoke。
4. Required gates 失败时不得发布。
5. Unsupported platform 必须显式声明，不得静默跳过。
6. CI/release workflow 修改后必须保持 clean worktree 和 generated file consistency。

## 验收标准

- [ ] 支持矩阵声明 Linux/macOS/Windows 状态，或明确 Windows unsupported。
- [ ] Release preflight 校验 tag、Cargo versions 和 changelog。
- [ ] Artifact 生成 checksum/signature，并对 archives 和 `cargo install sage-cli` 做 smoke。
- [ ] Required gates 覆盖 audit/deny/MSRV/fmt/clippy/test/doc consistency。
- [ ] Action pinning 或等价 policy 有文档化规则和验证。
- [ ] 覆盖 version mismatch、missing changelog、missing checksum、install smoke failure 测试或 dry-run。

## 边界情况

- Tag 指向版本 `v1.2.3` 但 Cargo.toml 是 `1.2.2`：release preflight fail。
- Windows 未支持：CI/release 文档必须明确，不让用户误以为通过。
- Checksum manifest 缺少某个 artifact：release fail。
- Generated docs 或 lockfile 产生 dirty diff：gate fail 并打印路径。

## 发布说明

本 PR 仅添加 GH-91 focused spec。实现 PR 需要说明 required check policy、release preflight、artifact verification 和 platform support matrix。
