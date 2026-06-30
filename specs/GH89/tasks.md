# Task Plan

## Linked Issue

GH-89

## Spec Packet

- Product: `product.md`
- Tech: `tech.md`

## 实现任务

- [ ] `SP89-T01` Owner: credentials. Done when: credential backend trait and source precedence table are implemented. Verify: precedence fixture tests pass.
- [ ] `SP89-T02` Owner: credentials. Done when: env override remains explicit, and legacy plaintext config can be imported or used only through documented fallback. Verify: env/legacy tests pass.
- [ ] `SP89-T03` Owner: credentials. Done when: save/logout/revoke return structured status, redacted provider identity and recovery hints. Verify: fake backend operation tests pass.
- [ ] `SP89-T04` Owner: providers. Done when: provider/model catalog supports TTL, ETag, static fallback and freshness metadata. Verify: mock HTTP catalog tests pass.
- [ ] `SP89-T05` Owner: LLM. Done when: all capability lookups route through one manager with deterministic unknown-model fallback. Verify: capability manager tests pass.
- [ ] `SP89-T06` Owner: coordinator. Done when: this focused spec PR links GH-89, excludes App/IDE/app-server-client scope, and does not claim implementation completion. Verify: SpecRail packet check, forbidden typo scan and `cargo check --workspace --all-targets --all-features`.

## 并行拆分

Credential backend and catalog cache can proceed independently. Capability manager integration should wait until catalog merge semantics are stable. GH-90 diagnostics should consume redacted credential/catalog status after GH-89 defines those outputs.

## 验证

Spec PR verification:

- `git diff --check`
- `python3 checks/check_workflow.py --repo <specrail> --spec-dir <repo>/specs/GH89`
- Forbidden typo scan over `specs/GH89`
- `python3 scripts/check_doc_consistency.py`
- `cargo check --workspace --all-targets --all-features`

Future implementation verification:

- `cargo test -p sage-core credential_backend`
- `cargo test -p sage-core credential_precedence`
- `cargo test -p sage-core model_catalog`
- `cargo test -p sage-core capability_manager`
- `cargo check --workspace --all-targets --all-features`

## Handoff Notes

Use `Refs #89` for spec-only PRs. Use a closing keyword only after secure credential backend, catalog cache and capability manager acceptance criteria are implemented.
