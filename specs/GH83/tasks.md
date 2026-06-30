# Task Plan

## Linked Issue

GH-83

## Spec Packet

- Product: `product.md`
- Tech: `tech.md`

## 实现任务

- [ ] `SP83-T01` Owner: runtime-api. Done when: runtime facade request/response/error/status types are defined and map to GH-81 protocol vocabulary. Verify: type and validation tests pass.
- [ ] `SP83-T02` Owner: runtime-api. Done when: facade wraps `UnifiedExecutor` start/resume execution without duplicating the execution loop. Verify: shared setup path tests pass.
- [ ] `SP83-T03` Owner: cli. Done when: CLI print, continue, resume and stream-json routes call the facade while preserving existing output semantics. Verify: CLI contract/snapshot tests pass.
- [ ] `SP83-T04` Owner: sdk. Done when: SDK interactive and non-interactive execution use the facade while preserving `ExecutionResult` and input handle behavior. Verify: SDK contract tests pass.
- [ ] `SP83-T05` Owner: runtime-api. Done when: protocol stream hook and ephemeral/null ThreadStore seam are available for later GH-82 integration. Verify: protocol fixture and mock state tests pass.
- [ ] `SP83-T06` Owner: coordinator. Done when: this focused spec PR links GH-83, excludes App/IDE/app-server-client scope, and does not claim implementation completion. Verify: SpecRail packet check, forbidden typo scan and `cargo check --workspace --all-targets --all-features`.

## 并行拆分

GH-83 should wait for GH-81 protocol spec. Runtime facade type work can start before GH-82 implementation if it supports an explicit ephemeral/null state mode. CLI and SDK migrations should be separate PRs after the facade contract lands.

## 验证

Spec PR verification:

- `git diff --check`
- `python3 checks/check_workflow.py --repo <specrail> --spec-dir <repo>/specs/GH83`
- Forbidden typo scan over `specs/GH83`
- `python3 scripts/check_doc_consistency.py`
- `cargo check --workspace --all-targets --all-features`

Future implementation verification:

- `cargo test -p sage-core runtime`
- `cargo test -p sage-cli unified_contract`
- `cargo test -p sage-sdk protocol_contract`
- `cargo check --workspace --all-targets --all-features`

## Handoff Notes

Use `Refs #83` for spec-only PRs. Use a closing keyword only on the final implementation PR that satisfies CLI and SDK compatibility acceptance.
