# Task Plan

## Linked Issue

GH-88

## Spec Packet

- Product: `product.md`
- Tech: `tech.md`

## 实现任务

- [ ] `SP88-T01` Owner: permissions. Done when: unified `PermissionProfile` DTO and merge/precedence rules cover filesystem, network, exec, sandbox and approval. Verify: precedence fixture tests pass.
- [ ] `SP88-T02` Owner: permissions. Done when: central decision engine returns structured allow/deny/ask/unsupported decisions with deny-before-allow semantics. Verify: conflict and provenance tests pass.
- [ ] `SP88-T03` Owner: tools. Done when: Bash sync/background execution uses the central decision engine and sandbox adapter. Verify: Bash deny and approval tests pass.
- [ ] `SP88-T04` Owner: sandbox. Done when: platform sandbox support is explicit and unsupported sandbox requests fail closed. Verify: platform support tests pass.
- [ ] `SP88-T05` Owner: security. Done when: protected paths, outside-workspace writes, network deny and approval timeout are enforced. Verify: security negative tests pass.
- [ ] `SP88-T06` Owner: coordinator. Done when: this focused spec PR links GH-88, excludes App/IDE/app-server-client scope, and does not claim implementation completion. Verify: SpecRail packet check, forbidden typo scan and `cargo check --workspace --all-targets --all-features`.

## 并行拆分

Profile DTO and decision engine should land before Bash/sandbox adapters. Bash and platform sandbox work can proceed in parallel only after the shared decision API is stable. GH-86 and GH-87 should consume this permission model for untrusted extension and MCP execution.

## 验证

Spec PR verification:

- `git diff --check`
- `python3 checks/check_workflow.py --repo <specrail> --spec-dir <repo>/specs/GH88`
- Forbidden typo scan over `specs/GH88`
- `python3 scripts/check_doc_consistency.py`
- `cargo check --workspace --all-targets --all-features`

Future implementation verification:

- `cargo test -p sage-core permission_profile`
- `cargo test -p sage-core permission_decision`
- `cargo test -p sage-core sandbox`
- `cargo test -p sage-tools bash_permission`
- `cargo check --workspace --all-targets --all-features`

## Handoff Notes

Use `Refs #88` for spec-only PRs. Use a closing keyword only after permission profile, central decision engine, Bash integration and sandbox acceptance criteria are implemented.
