# Sage Runtime Capability Roadmap

Date: 2026-06-30

## Scope

This roadmap compares Sage against the current OpenAI Codex open-source runtime
implementation, then converts the gaps into issue-sized Sage work. It is not a
request to copy UI surfaces.

In scope:

- runtime protocol, thread lifecycle, turn and item events
- persistent thread/session state
- CLI and SDK runtime API boundaries
- child-agent graph, background output, inter-agent messaging, and context fork
- manifest-driven extensions, skills, MCP runtime, and deferred tool discovery
- permission profiles, approval events, platform sandboxing, credentials,
  model catalog, telemetry, managed configuration, release, CI, and supply chain

Out of scope by maintainer decision:

- desktop app
- VS Code, Cursor, Windsurf, or other IDE entrypoints
- app-server client

## Evidence

- Sage `origin/main`: `cf95e7d2c246a2e872c21bbcc47309842fc236cb`
- Reference repo `origin/main`: `ccdfb4f342a2e659be7ab878309cc5d81683d737`
- SpecRail workflow pack: `240e8df6eeaa03c4f3d51593700a86d4de10cef3`
- GitHub queue before creation: open issues `0`, open PRs `0`
- Threads lanes used:
  - runtime/state/API lane
  - multi-agent/extensions/MCP lane
  - security/auth/operations lane

## Issue Map

| Issue | Priority | Area | Summary |
| --- | --- | --- | --- |
| #80 | Roadmap | Coordination | Umbrella roadmap and SpecRail packet |
| #81 | P0 | Runtime protocol | Define Sage thread/turn/item request and event schema |
| #82 | P0 | State | Build persistent `ThreadStore` and session index |
| #83 | P0 | Runtime API | Route CLI and SDK through one runtime facade |
| #84 | P1 | Multi-agent | Add child-agent graph, background output, and messaging |
| #85 | P1 | Multi-agent | Add context fork and configurable agent roles |
| #86 | P1 | Extensions | Add manifest-driven extension and skill distribution |
| #87 | P1 | MCP | Add auth, plugin sources, and deferred MCP discovery |
| #88 | P1 | Security | Unify permission profiles and platform sandbox execution |
| #89 | P2 | Auth/models | Add secure credential backend and refreshable model catalog |
| #90 | P2 | Ops | Add feedback diagnostics, managed config, and audit logs |
| #91 | P2 | Release | Strengthen cross-platform CI, supply chain, and release gates |

## Feature Matrix

| Capability | Sage today | Reference capability | Roadmap action |
| --- | --- | --- | --- |
| CLI interactive and print mode | Present | Present | Keep compatible through #83 |
| Rust SDK execution | Present | Present via runtime APIs | Converge under #83 |
| Streaming JSON output | Partial | Typed protocol events and exported schema | #81 |
| Thread lifecycle | Partial session resume/checkpoint model | Thread start/resume/fork/close/list/read | #81, #82, #83 |
| Turn/item event model | Partial executor events | Typed turn/item/request/notification model | #81 |
| Persistent session store | Partial JSONL/session metadata/trajectory | SQLite-backed metadata, rollout, searchable thread store | #82 |
| Thread search/archive | Missing | List/search/archive thread metadata | #82 |
| Runtime service boundary | Partial `UnifiedExecutor` coupling | Shared runtime facade and request handlers | #83 |
| Local execution service protocol | Missing | Process, filesystem, HTTP, attach/resume protocol | Future child of #83 after P0 boundary lands |
| Child-agent graph | Partial subagent runner | Persistent parent-child graph and descendants query | #84 |
| Background subagent output | Partial; background path is not a full child-agent registry | Wait/list/interrupt/output for child agents | #84 |
| Inter-agent messaging | Partial team/message concepts | Agent path, mailbox, queued message, follow-up turn | #84 |
| Context fork | Missing | None/all/recent-N inheritance policy | #85 |
| Configurable agent roles | Partial fixed role enum | Role files with prompt/model/tool/profile overrides | #85 |
| Skill discovery | Present | Present | Integrate with extension manifest under #86 |
| Extension package lifecycle | Partial plugin trait/registry | Manifest install/list/read/uninstall/enable/disable | #86 |
| Tool search/deferred loading | Partial skill/tool discovery | Deferred dynamic tool exposure and search metadata | #86, #87 |
| MCP tools/resources/prompts | Present | Present | Keep and harden under #87 |
| MCP auth/OAuth/status | Partial/missing | Auth elicitation, auth status, retry handling | #87 |
| MCP plugin source merge | Missing | Direct config plus plugin-sourced MCP servers | #87 |
| Permission settings | Partial settings permissions and hooks | Unified permission profile and approval protocol | #88 |
| Platform sandbox enforcement | Partial sandbox module and policies | Platform sandbox wrappers and fail-closed behavior | #88 |
| Network policy | Partial and permissive | Profile-controlled network decisions | #88 |
| Credential storage | Partial local JSON/env | Secret backend/keyring plus lifecycle operations | #89 |
| Model catalog | Partial static provider/model tables | Refreshable catalog with cache and fallback | #89 |
| Telemetry metrics | Partial in-process metrics | Feedback bundle, ring buffer, redaction, OTEL/analytics option | #90 |
| Managed configuration | Partial user/project/local settings | Read-only managed policy layer and source-aware errors | #90 |
| Release CI | Present but not complete | Required cross-platform gates, artifacts, checksums, smoke | #91 |
| Supply-chain gates | Partial audit/deny/outdated | Required dependency/license/action/clean-tree gates | #91 |
| Desktop app | Not applicable | Present in reference ecosystem | Explicitly excluded |
| IDE entrypoints | Not applicable | Present in reference ecosystem | Explicitly excluded |
| app-server client | Not applicable | Present in reference ecosystem | Explicitly excluded |

## Recommended Order

1. Land #81 and #82 first. They define the protocol and state substrate.
2. Land #83 after #81/#82 so CLI and SDK stop growing around executor internals.
3. Land #84 and #85 after the thread store can represent lineage and child
   state.
4. Land #86 and #87 after the runtime can expose typed tools and stateful MCP
   failures.
5. Land #88 before broadening automation, because sandbox and approval behavior
   is a trust boundary.
6. Land #89, #90, and #91 as maturity work once the runtime boundary is stable.

## SpecRail Status

- `triage_issue` route was locally allowed before creating #80-#91.
- `write_spec` requires a linked issue and a maintainer readiness signal. This
  PR provides the linked spec packet for #80, but implementation and merge gates
  still remain human-owned.
- The created child issues are intentionally triaged roadmap items, not
  `ready_to_implement` implementation tickets.

## Non-Goals

This roadmap must not grow into desktop, IDE, or app-server-client work. If
those surfaces become wanted later, they should be separate product decisions
with separate issues and specs.
