# Release Process

This document defines the supported release matrix and the gates that must pass before a Sage release is published.

## Support Matrix

| Platform | Target | Release archive | Status |
| --- | --- | --- | --- |
| Linux | `x86_64-unknown-linux-gnu` | `tar.gz` | Supported |
| Linux | `x86_64-unknown-linux-musl` | `tar.gz` | Supported |
| Linux | `aarch64-unknown-linux-gnu` | `tar.gz` | Supported, smoked with `qemu-aarch64` |
| macOS | `aarch64-apple-darwin` | `tar.gz` | Supported, smoked on an ARM64 runner |
| macOS | `x86_64-apple-darwin` | `tar.gz` | Supported, smoked on an Intel runner |
| Windows | Native Windows archives | None | Unsupported |

The machine-readable source of truth is `.github/release-support.json`.

## Versioning

Release tags must use `vMAJOR.MINOR.PATCH` with an optional prerelease suffix. The tag version must match the workspace package version, the root package version, every workspace package version, and explicit internal path dependency versions.

## Changelog

Every release tag must have a matching `## [MAJOR.MINOR.PATCH]` entry in `CHANGELOG.md` before a draft release is created.

## Deployment

Deployment means publishing verified GitHub release archives and, for stable releases, publishing the owned Sage crates needed by `cargo install sage-cli`. Stable releases remain draft until the public `cargo install sage-cli --version <version>` smoke passes.

## Required Gates

The release workflow fails closed before publishing. A tag release must pass:

- release preflight for tag, Cargo workspace versions, path dependency versions, changelog entry, support matrix, and workflow policy
- quality gates for fmt, clippy, workspace tests, documentation consistency, and clean worktree state
- security gates for cargo audit, cargo deny, outdated dependency checks, MSRV, and action policy
- artifact verification for per-archive checksums, aggregate checksum manifest, release manifest, archive contents, and `sage --version` smoke for every supported archive target
- Cargo install smoke for the local `sage-cli` package before release publish and the published `sage-cli` crate before stable GitHub releases are made public

## Local Commands

```bash
make release-gate
make release-self-test
make release-preflight TAG=v0.13.57
make release-artifact-smoke VERSION=v0.13.57 ARTIFACT_DIR=release-artifacts
make release-smoke VERSION=v0.13.57
```

## Artifact Verification

Each release archive must contain `sage`, `LICENSE`, and `NOTICE`. The release workflow uploads a matching `.sha256` file for every archive, writes `SHA256SUMS`, writes `release-manifest.json`, and creates GitHub artifact attestations for the archive and checksum sidecar.

Every supported archive is executable-smoked before release publish. Native macOS targets run on matching ARM64 or Intel runners, Linux x86 targets run on Ubuntu runners, and Linux ARM64 runs through `qemu-aarch64` with the AArch64 sysroot installed.

## Unsupported Platforms

Native Windows release archives are intentionally unsupported until a dedicated Windows packaging and smoke path is restored. Windows users should use WSL2, source builds, or `cargo install sage-cli` when the crate is available.

Unsupported platforms must be explicit in `.github/release-support.json`; they are never treated as silent success.

## Branch Protection

Repository settings should require the stable aggregate checks `CI / Required gates` and `Security Audit / Required security gates`. These aggregate jobs depend on the individual fmt, clippy, test, build, documentation, release policy, clean worktree, audit, deny, outdated dependency, MSRV, and workflow policy jobs.
