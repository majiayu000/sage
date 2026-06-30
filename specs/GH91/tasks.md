# Task Plan

## Linked Issue

GH-91

## Spec Packet

- Product: `product.md`
- Tech: `tech.md`

## 实现任务

- [ ] `SP91-T01` Owner: release. Done when: support matrix declares Linux/macOS/Windows status or explicitly marks unsupported platforms. Verify: support matrix check passes.
- [ ] `SP91-T02` Owner: release. Done when: preflight checks release tag, Cargo workspace/package versions and changelog entry. Verify: mismatch fixture tests pass.
- [ ] `SP91-T03` Owner: release. Done when: artifacts include checksum/signature manifest and archive plus `cargo install sage-cli` smoke checks. Verify: artifact smoke dry-run passes.
- [ ] `SP91-T04` Owner: security. Done when: audit/deny/MSRV/action pinning or equivalent policy are required gates. Verify: workflow validation proves gates are required.
- [ ] `SP91-T05` Owner: CI. Done when: fmt/clippy/test/doc consistency and clean-worktree generated file gates block publish on failure. Verify: CI workflow validation and dirty-file fixture pass.
- [ ] `SP91-T06` Owner: coordinator. Done when: this focused spec PR links GH-91, excludes App/IDE/app-server-client scope, and does not claim implementation completion. Verify: SpecRail packet check, forbidden typo scan and `cargo check --workspace --all-targets --all-features`.

## 并行拆分

Preflight fixture tests and support matrix docs can start independently. Artifact smoke should wait until release artifact naming is stable. Security gate policy should be reviewed before any workflow is marked required.

## 验证

Spec PR verification:

- `git diff --check`
- `python3 checks/check_workflow.py --repo <specrail> --spec-dir <repo>/specs/GH91`
- Forbidden typo scan over `specs/GH91`
- `python3 scripts/check_doc_consistency.py`
- `cargo check --workspace --all-targets --all-features`

Future implementation verification:

- `cargo test -p sage-core release_preflight` or equivalent fixture runner
- workflow validation or dry-run for CI/release/security
- artifact checksum verification
- archive and `cargo install sage-cli` smoke checks
- `cargo check --workspace --all-targets --all-features`

## Handoff Notes

Use `Refs #91` for spec-only PRs. Use a closing keyword only after release preflight, required gates, checksum/signing and install smoke acceptance criteria are implemented.
