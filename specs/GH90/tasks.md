# Task Plan

## Linked Issue

GH-90

## Spec Packet

- Product: `product.md`
- Tech: `tech.md`

## 实现任务

- [ ] `SP90-T01` Owner: diagnostics. Done when: telemetry/diagnostic events use a bounded ring buffer with dropped-count metadata. Verify: capacity and overflow tests pass.
- [ ] `SP90-T02` Owner: security. Done when: feedback bundle redaction covers credentials, tokens, cookies, provider keys and sensitive paths before artifact write/upload. Verify: redaction fixture tests pass.
- [ ] `SP90-T03` Owner: CLI. Done when: diagnostic bundle creation requires explicit user consent and supports decline without artifact write. Verify: consent/decline command tests pass.
- [ ] `SP90-T04` Owner: settings. Done when: managed read-only config has strict schema, source provenance and restrictive-only precedence. Verify: strict schema and precedence tests pass.
- [ ] `SP90-T05` Owner: audit. Done when: permission, sandbox and provider decisions include redacted source/reason audit summaries. Verify: audit source tests pass.
- [ ] `SP90-T06` Owner: coordinator. Done when: this focused spec PR links GH-90, excludes App/IDE/app-server-client scope, and does not claim implementation completion. Verify: SpecRail packet check, forbidden typo scan and `cargo check --workspace --all-targets --all-features`.

## 并行拆分

Bounded event ring and redaction fixtures can start independently. Managed config should align with GH-88 permission precedence. Provider/auth diagnostic summaries should consume GH-89 credential/catalog status once available.

## 验证

Spec PR verification:

- `git diff --check`
- `python3 checks/check_workflow.py --repo <specrail> --spec-dir <repo>/specs/GH90`
- Forbidden typo scan over `specs/GH90`
- `python3 scripts/check_doc_consistency.py`
- `cargo check --workspace --all-targets --all-features`

Future implementation verification:

- `cargo test -p sage-core diagnostics_event_ring`
- `cargo test -p sage-core diagnostics_redaction`
- `cargo test -p sage-core managed_config`
- `cargo test -p sage-core audit_policy_source`
- `cargo test -p sage-cli diagnostics`
- `cargo check --workspace --all-targets --all-features`

## Handoff Notes

Use `Refs #90` for spec-only PRs. Use a closing keyword only after diagnostic bundle, redaction, managed config and audit acceptance criteria are implemented.
