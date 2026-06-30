# Task Plan

## Linked Issue

GH-86

## Spec Packet

- Product: `product.md`
- Tech: `tech.md`

## 实现任务

- [ ] `SP86-T01` Owner: extensions. Done when: extension manifest v0 schema covers metadata, assets, dependencies and permissions. Verify: manifest fixture tests pass.
- [ ] `SP86-T02` Owner: extensions. Done when: package lifecycle API supports discover/list/read/install/uninstall/enable/disable. Verify: lifecycle unit tests pass.
- [ ] `SP86-T03` Owner: registry. Done when: enabled packages register skills, MCP servers, hooks and commands with package/source metadata. Verify: registry bridge tests pass.
- [ ] `SP86-T04` Owner: security. Done when: path escape, missing dependency and undeclared permission cases fail closed before registry mutation. Verify: negative fixture tests pass.
- [ ] `SP86-T05` Owner: extensions. Done when: disabling/uninstalling package unregisters all declared assets and leaves no stale registry entries. Verify: enable-disable-uninstall integration tests pass.
- [ ] `SP86-T06` Owner: coordinator. Done when: this focused spec PR links GH-86, excludes graphical store/App/IDE/app-server-client scope, and does not claim implementation completion. Verify: SpecRail packet check, forbidden typo scan and `cargo check --workspace --all-targets --all-features`.

## 并行拆分

Manifest parser and lifecycle store work can proceed before registry bridge integration. Registry bridge work should wait until asset declaration types are stable. GH-87 should consume package-sourced MCP metadata after GH-86 establishes the manifest contract.

## 验证

Spec PR verification:

- `git diff --check`
- `python3 checks/check_workflow.py --repo <specrail> --spec-dir <repo>/specs/GH86`
- Forbidden typo scan over `specs/GH86`
- `python3 scripts/check_doc_consistency.py`
- `cargo check --workspace --all-targets --all-features`

Future implementation verification:

- `cargo test -p sage-core package_manifest`
- `cargo test -p sage-core package_lifecycle`
- `cargo test -p sage-core package_registry_bridge`
- `cargo check --workspace --all-targets --all-features`

## Handoff Notes

Use `Refs #86` for spec-only PRs. Use a closing keyword only after package lifecycle, registry bridge and safety acceptance criteria are implemented.
