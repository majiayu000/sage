# 发布流程

本文定义 Sage 发布前必须通过的支持矩阵和发布 gate。

## 支持矩阵

| 平台 | Target | 发布归档 | 状态 |
| --- | --- | --- | --- |
| Linux | `x86_64-unknown-linux-gnu` | `tar.gz` | 支持 |
| Linux | `x86_64-unknown-linux-musl` | `tar.gz` | 支持 |
| Linux | `aarch64-unknown-linux-gnu` | `tar.gz` | 支持，通过 `qemu-aarch64` smoke |
| macOS | `aarch64-apple-darwin` | `tar.gz` | 支持，在 ARM64 runner 上 smoke |
| macOS | `x86_64-apple-darwin` | `tar.gz` | 支持，在 Intel runner 上 smoke |
| Windows | 原生 Windows 归档 | 无 | 不支持 |

机器可读的事实源是 `.github/release-support.json`。

## Versioning

Release tag 必须使用 `vMAJOR.MINOR.PATCH`，可带 prerelease 后缀。Tag version 必须匹配 workspace package version、root package version、每个 workspace package version，以及显式内部 path dependency version。

## Changelog

每个 release tag 在创建 draft release 之前，必须在 `CHANGELOG.md` 中有匹配的 `## [MAJOR.MINOR.PATCH]` 条目。

## Deployment

Deployment 指发布已验证的 GitHub release archives，并且对 stable release 发布 `cargo install sage-cli` 所需的 Sage 自有 crates。Stable release 在公开 GitHub release 前，必须先通过公开的 `cargo install sage-cli --version <version>` smoke。

## 必需 Gate

发布 workflow 在 publish 前 fail closed。Tag 发布必须通过：

- release preflight：tag、Cargo workspace 版本、path dependency 版本、changelog 条目、支持矩阵和 workflow policy
- quality gates：fmt、clippy、workspace tests、文档一致性和 clean worktree 状态
- security gates：cargo audit、cargo deny、过期依赖检查、MSRV 和 action policy
- artifact verification：每个归档的 checksum、聚合 checksum manifest、release manifest、归档内容，并对每个支持的 archive target 执行 `sage --version` smoke
- Cargo install smoke：发布前检查本地 `sage-cli` package，并在 stable GitHub release 公开前检查已发布的 `sage-cli` crate

## 本地命令

```bash
make release-gate
make release-self-test
make release-preflight TAG=v0.13.57
make release-artifact-smoke VERSION=v0.13.57 ARTIFACT_DIR=release-artifacts
make release-smoke VERSION=v0.13.57
```

## Artifact Verification

每个 release archive 必须包含 `sage`、`LICENSE` 和 `NOTICE`。Release workflow 会为每个 archive 上传匹配的 `.sha256` 文件，写入 `SHA256SUMS`，写入 `release-manifest.json`，并为 archive 和 checksum sidecar 创建 GitHub artifact attestations。

每个支持的 archive 都会在发布公开前执行 executable smoke。macOS 原生目标在匹配的 ARM64 或 Intel runner 上运行，Linux x86 目标在 Ubuntu runner 上运行，Linux ARM64 通过安装 AArch64 sysroot 后的 `qemu-aarch64` 运行。

## 不支持的平台

原生 Windows release archives 在恢复专门的 Windows packaging 和 smoke 路径之前明确不支持。Windows 用户应使用 WSL2、源码构建，或在 crate 可用时使用 `cargo install sage-cli`。

不支持的平台必须在 `.github/release-support.json` 中显式声明；不能把缺失平台当成静默成功。

## Branch Protection

仓库设置应要求稳定的聚合检查 `CI / Required gates` 和 `Security Audit / Required security gates`。这些聚合 job 依赖 fmt、clippy、test、build、documentation、release policy、clean worktree、audit、deny、过期依赖、MSRV 和 workflow policy 等单项 job。
