# Task Plan

## Linked Issue

GH-87

## Spec Packet

- Product: `product.md`
- Tech: `tech.md`

## 实现任务

- [ ] `SP87-T01` Owner: MCP. Done when: direct config and extension-sourced MCP declarations merge into one source set with source metadata. Verify: source merge fixture tests pass.
- [ ] `SP87-T02` Owner: MCP. Done when: auth/OAuth status and authorization prompt states are exposed programmatically. Verify: auth pending and recovery tests pass.
- [ ] `SP87-T03` Owner: runtime. Done when: connect/disconnect/retry actions update structured server status, and unsupported transport modes fail closed. Verify: fake transport lifecycle tests pass.
- [ ] `SP87-T04` Owner: MCP tools. Done when: deferred MCP tool list/search exposes server identity and freshness without eager startup. Verify: deferred discovery tests pass.
- [ ] `SP87-T05` Owner: reliability. Done when: connection, auth and schema failures return structured errors without hiding enabled-source failures. Verify: negative error fixture tests pass.
- [ ] `SP87-T06` Owner: coordinator. Done when: this focused spec PR links GH-87, excludes App/IDE/app-server-client scope, and does not claim implementation completion. Verify: SpecRail packet check, forbidden typo scan and `cargo check --workspace --all-targets --all-features`.

## 并行拆分

Source merge and status model can be implemented before transport actions. Deferred tool search can start once source identity and server status types are stable. Package-sourced MCP should wait for GH-86's package manifest contract.

## 验证

Spec PR verification:

- `git diff --check`
- `python3 checks/check_workflow.py --repo <specrail> --spec-dir <repo>/specs/GH87`
- Forbidden typo scan over `specs/GH87`
- `python3 scripts/check_doc_consistency.py`
- `cargo check --workspace --all-targets --all-features`

Future implementation verification:

- `cargo test -p sage-core mcp_source`
- `cargo test -p sage-core mcp_auth_status`
- `cargo test -p sage-core mcp_runtime_status`
- `cargo test -p sage-tools mcp_tools`
- `cargo check --workspace --all-targets --all-features`

## Handoff Notes

Use `Refs #87` for spec-only PRs. Use a closing keyword only after source merge, auth status, controlled startup and deferred discovery acceptance criteria are implemented.
