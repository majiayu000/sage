# Sage Runtime Capability Roadmap

日期：2026-06-30

## Scope

这份 roadmap 对比 Sage 与当前 OpenAI Codex 开源 runtime 实现，并把差距转换成 Sage 自身可跟踪的 issue。它不是复制 UI 入口的请求。

本轮范围：

- runtime protocol、thread lifecycle、turn 和 item events
- 持久化 thread/session state
- CLI 和 SDK runtime API 边界
- 子代理图、后台输出、代理间消息和 context fork
- manifest 驱动的 extensions、skills、MCP runtime 和 deferred tool discovery
- permission profiles、approval events、平台 sandbox、credentials、model catalog、telemetry、managed configuration、release、CI 和 supply chain

维护者明确排除：

- desktop app
- VS Code、Cursor、Windsurf 或其他 IDE entrypoints
- app-server client

## Evidence

- Sage `origin/main`: `cf95e7d2c246a2e872c21bbcc47309842fc236cb`
- 参考仓库 `origin/main`: `ccdfb4f342a2e659be7ab878309cc5d81683d737`
- SpecRail workflow pack: `240e8df6eeaa03c4f3d51593700a86d4de10cef3`
- 创建前 GitHub 队列：open issues `0`，open PRs `0`
- 使用的 threads lanes:
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

1. 先落地 #81 和 #82。它们定义 protocol 和 state substrate。
2. #81/#82 后再落地 #83，避免 CLI 和 SDK 继续直接围绕 executor internals 生长。
3. thread store 能表达 lineage 和 child state 后，再落地 #84 和 #85。
4. runtime 能暴露 typed tools 和 stateful MCP failures 后，再落地 #86 和 #87。
5. #88 应在扩大自动化前完成，因为 sandbox 和 approval 是信任边界。
6. #89、#90、#91 属于 runtime boundary 稳定后的成熟度工作。

## SpecRail Status

- 创建 #80-#91 前，`triage_issue` route 本地 gate 为 allowed。
- `write_spec` 需要 linked issue 和 maintainer readiness signal。本 PR 为 #80 提供 linked spec packet，但 implementation 和 merge gates 仍由 human owner 控制。
- 已创建的子 issue 是 triaged roadmap items，不是 `ready_to_implement` implementation tickets。

## Non-Goals

这份 roadmap 不能扩展成 desktop、IDE 或 app-server-client 工作。如果后续需要这些入口，应作为独立产品决策创建单独 issue 和 spec。
