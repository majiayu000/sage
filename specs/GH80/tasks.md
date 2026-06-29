# Task Plan

## Linked Issue

GH-80

## Spec Packet

- Product: `product.md`
- Tech: `tech.md`

## 实现任务

- [ ] `SP80-T01` Owner: runtime. Done when: #81 defines Sage protocol events for thread, turn, item, permission, and errors. Verify: schema fixture and event mapping tests pass.
- [ ] `SP80-T02` Owner: state. Done when: #82 provides persistent `ThreadStore`, migrations, JSONL backfill, list/read/search/archive, and restart recovery. Verify: migration and query tests pass.
- [ ] `SP80-T03` Owner: runtime-api. Done when: #83 routes CLI and SDK through a shared runtime facade without breaking print/resume/stream JSON behavior. Verify: CLI/SDK contract tests pass.
- [ ] `SP80-T04` Owner: multi-agent. Done when: #84 persists child-agent graph edges and supports background output, wait, list, interrupt, and follow-up messaging. Verify: child-agent lifecycle tests pass.
- [ ] `SP80-T05` Owner: multi-agent. Done when: #85 supports configured roles and context fork modes. Verify: role loading, fork policy, and tool-scope tests pass.
- [ ] `SP80-T06` Owner: extensions. Done when: #86 supports manifest-driven extension install/list/read/uninstall/enable/disable. Verify: manifest fixture and lifecycle tests pass.
- [ ] `SP80-T07` Owner: mcp. Done when: #87 supports MCP source metadata, auth status, user authorization prompts, controlled startup, and deferred discovery. Verify: MCP auth/source/deferred-tool tests pass.
- [ ] `SP80-T08` Owner: security. Done when: #88 routes filesystem, network, exec, sandbox, and approval through one permission profile. Verify: allow/deny/ask/sandbox tests pass.
- [ ] `SP80-T09` Owner: auth-models. Done when: #89 supports secure credential backend and refreshable model catalog with offline fallback. Verify: credential lifecycle and model refresh tests pass.
- [ ] `SP80-T10` Owner: ops. Done when: #90 supports redacted diagnostics, feedback bundles, managed config, and audit log source attribution. Verify: redaction and managed-policy tests pass.
- [ ] `SP80-T11` Owner: release. Done when: #91 adds cross-platform CI, supply-chain gates, artifact checksums, and install smoke gates. Verify: release workflow dry-run or CI fixture passes.
- [ ] `SP80-T12` Owner: coordinator. Done when: this roadmap PR links #80, documents #81-#91, and does not include excluded App/IDE/app-server-client scope as implementation work. Verify: issue list, SpecRail packet validation, forbidden typo scan, and `cargo check --workspace --all-targets --all-features`.

## 并行拆分

The first implementation tranche should be serial for #81, #82, and #83 because
the protocol, persistent state, and runtime API facade define shared contracts.
After those land, #84/#85, #86/#87, and #89/#90/#91 can be prepared in parallel
when their writable files are disjoint. #88 should be reviewed as a security
boundary and should not be mixed with unrelated extension or telemetry changes.

## 验证

Verification for this roadmap PR is handled by `SP80-T12`. Future implementation
PRs must add focused tests named in their child issue and then run the Rust
workspace completion check.

## Handoff Notes

The child issues are triaged roadmap items. Do not treat them as
`ready_to_implement` until a maintainer explicitly marks readiness and the
focused spec for that issue is accepted.
