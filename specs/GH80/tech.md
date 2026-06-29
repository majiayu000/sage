# Tech Spec

## Linked Issue

GH-80

## Product Spec

Link to `product.md`.

## Codebase Context

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| CLI | `crates/sage-cli/src/args.rs` | Supports interactive, print, continue, resume, stream JSON, status, doctor, usage | #83 must preserve existing CLI behavior while routing through a runtime facade |
| Runtime | `crates/sage-core/src/agent/unified/**` | `UnifiedExecutor` owns execution loop, sessions, tool calls, compaction, and restoration | #81/#83 must extract stable protocol and API boundaries without breaking executor behavior |
| Sessions | `crates/sage-core/src/session/**`, `crates/sage-core/src/trajectory/**` | JSONL/session metadata/checkpoint/trajectory exist | #82 must index and backfill rather than discard existing data |
| Subagents | `crates/sage-core/src/agent/subagent/**`, `crates/sage-tools/src/tools/process/task/**` | Subagent and Task concepts exist, but background child-agent registry is incomplete | #84/#85 depend on durable thread graph and context fork policy |
| Skills/plugins | `crates/sage-core/src/skills/**`, `crates/sage-core/src/plugins/**` | Skill discovery and plugin framework exist | #86 should converge them into manifest-driven distribution |
| MCP | `crates/sage-core/src/mcp/**`, `crates/sage-core/src/config/mcp_config.rs` | MCP tools/resources/prompts and refresh exist | #87 should add auth, source metadata, controlled startup, and deferred discovery |
| Security | `crates/sage-core/src/sandbox/**`, `crates/sage-core/src/agent/unified/settings_permission.rs`, `crates/sage-tools/src/tools/process/bash/**` | Sandbox/policy/permission pieces exist but need one runtime decision path | #88 is a trust-boundary prerequisite |
| Providers | `crates/sage-core/src/config/provider/**`, `crates/sage-core/src/config/credential/**`, `crates/sage-core/src/config/embedded_providers.rs` | Multi-provider config and local credentials exist | #89 should avoid drift and unsafe credential storage |
| Operations | `crates/sage-core/src/telemetry/**`, `crates/sage-core/src/settings/**`, `.github/workflows/**` | Metrics/settings/CI/release workflows exist | #90/#91 mature diagnostics, managed policy, and release gates |

## 设计方案

This PR is a planning/spec PR. It does not change Rust runtime behavior.

The design is to create one umbrella issue (#80), eleven child issues (#81-#91),
one analysis document, and one SpecRail packet:

- `docs/analysis/sage-runtime-capability-roadmap-2026-06-30.md`
- `specs/GH80/product.md`
- `specs/GH80/tech.md`
- `specs/GH80/tasks.md`

Future implementation PRs should use the issue order below:

1. #81 protocol, #82 store, #83 API facade.
2. #84 child-agent graph and messaging, then #85 context fork and roles.
3. #86 extension manifest, then #87 MCP runtime hardening.
4. #88 permission and platform sandbox before broad automation.
5. #89 credentials/model catalog, #90 diagnostics/managed config, #91 release/CI.

## Product-to-Test Mapping

| Product invariant | Implementation area | Verification |
| --- | --- | --- |
| P1 issue map covers #80-#91 | Analysis doc and GitHub issues | `gh issue list --repo majiayu000/sage --state open --json number,title` |
| P2 excluded surfaces stay out of scope | Product spec and analysis doc | `rg -n "desktop app|VS Code|Cursor|Windsurf|app-server client" docs/analysis specs/GH80` |
| P3 child issues contain acceptance criteria | GitHub issue bodies | `gh issue view <number> --json body` spot check or scripted audit |
| P4 P0 order precedes dependent work | Analysis doc and task plan | Review recommended order and `SP80-T01` through `SP80-T11` |
| P5 packet links GH-80 | `specs/GH80/*` | SpecRail packet validation helper |
| P6 no implementation/merge authorization claimed | PR body and specs | Review PR body gates |
| P7 no forbidden typo | Whole repo diff and remote titles | literal typo scan and `gh issue/pr list` checks |

## 数据流

1. Read-only comparison produces a feature matrix and gap list.
2. GitHub issues store durable roadmap items.
3. `docs/analysis` maps issues to capabilities and ordering.
4. `specs/GH80` stores the SpecRail packet for the umbrella roadmap.
5. Future implementation PRs link to the relevant child issue and add their own
   focused specs when the maintainer marks them ready.

## 备选方案

- Only answer in chat: rejected because it would not create durable issue/PR state.
- One giant implementation PR: rejected because runtime/state/security work has serial dependencies and high review risk.
- Create specs for all child issues immediately: deferred because child issues still need maintainer readiness and may be reprioritized.
- Include App/IDE work: rejected by explicit maintainer scope.

## 风险

- Security: #88 is only planned here; no sandbox behavior changes in this PR.
- Compatibility: docs-only PR should not alter runtime behavior.
- Performance: no runtime effect.
- Maintenance: roadmap can drift; future work must update issue state and specs when priorities change.

## 测试计划

- Unit tests: not applicable for docs-only roadmap PR.
- Integration tests: validate SpecRail packet structure for `specs/GH80`.
- Manual verification: check open issue list, check PR title/body, check no forbidden typo string, run `cargo check --workspace --all-targets --all-features` per repository completion rule.

## 回滚方案

Revert the docs/spec PR. If needed, close #80-#91 with a superseded note or edit
the issue bodies to point to a replacement roadmap.
