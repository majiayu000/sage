# Tech Spec

## Linked Issue

GH-91

## Product Spec

Link to `product.md`.

## Codebase Context

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| CI workflow | `.github/workflows/ci.yml` | Runs build/test style checks | Required PR gates and platform matrix live here |
| Release workflow | `.github/workflows/release.yml` | Builds release artifacts | Needs preflight, checksums and install smoke |
| Security workflow | `.github/workflows/security.yml` | Runs audit/dependency checks | Should be release-blocking |
| Doc consistency | `.github/workflows/doc-consistency.yml`, `scripts/check_doc_consistency.py` | Validates docs consistency | Release should fail on generated/doc drift |
| Build helpers | `Makefile` | Central command entry points | Can host reusable local gates |
| Cargo metadata | `Cargo.toml`, `Cargo.lock` | Package versions and lock state | Version/tag consistency source |
| Release notes | `CHANGELOG.md` | User-facing release history | Must match release tag/version |
| Supply chain policy | `deny.toml` | Dependency policy | Required security gate |

## 设计方案

Future implementation should add release preflight before artifact publish:

- `scripts/release_preflight.py` or a Rust xtask equivalent.
- Workflow job that runs before artifact upload.
- Artifact verification job that downloads, checks checksum/signature and smoke-installs.
- A documented support matrix in release docs or README.

Preflight should be runnable locally and in CI with the same inputs.

## Release Gate Sketch

Gate groups:

- source/version: tag, Cargo workspace, package versions, changelog
- code quality: fmt, clippy, tests, doc consistency
- security: audit, deny, MSRV, dependency/license checks
- supply chain: pinned actions policy, checksum/signature manifest
- artifact smoke: unpack archive, run `sage --version`, minimal non-network smoke
- platform matrix: Linux/macOS/Windows support status

## Product-to-Test Mapping

| Product invariant | Implementation area | Verification |
| --- | --- | --- |
| Version/tag consistency | release preflight | mismatch fixture tests |
| Changelog entry | release preflight | missing-entry tests |
| Checksums/signing | release workflow | checksum manifest tests |
| Install smoke | release workflow | archive/cargo install smoke |
| Required gates | CI/workflow policy | workflow validation/dry-run |
| Platform support | docs/workflow matrix | support matrix test/check |

## 数据流

1. Release starts from a tag.
2. Preflight reads tag, Cargo metadata and changelog.
3. CI/security/doc gates run or are verified as required.
4. Release workflow builds artifacts for supported platforms.
5. Checksum/signature manifest is generated and verified.
6. Smoke job installs each artifact and runs version/minimal command checks.
7. Publish proceeds only after all gate groups pass.

## 备选方案

- Rely on manual release checklist: rejected because release correctness should be enforceable.
- Treat missing platform job as success: rejected because unsupported platforms must be explicit.
- Publish artifacts before checksum/smoke: rejected because broken artifacts would reach users.

## 风险

- CI cost: full matrix can be expensive, so PR path and release path need distinct budgets.
- Platform variance: Windows support may require explicit unsupported status before implementation.
- Supply chain: unpinned actions or unsigned artifacts weaken release trust.
- Drift: local release commands and GitHub workflows can diverge without shared preflight.

## 测试计划

- Release preflight fixture tests for version/tag/changelog mismatch.
- Workflow validation or dry-run for release and CI jobs.
- Checksum manifest verification test.
- Artifact smoke commands for Linux/macOS and Windows or documented unsupported path.
- Dirty generated file gate test.
- Completion check: `cargo check --workspace --all-targets --all-features`.

## 回滚方案

Keep new release gates additive until proven. If a release gate has a false positive, block release and patch the gate with a failing fixture rather than bypassing required checks.
