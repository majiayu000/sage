# Tech Spec

## Linked Issue

GH-81

## Product Spec

Link to `product.md`.

## Codebase Context

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| Stream JSON | `crates/sage-cli/src/commands/unified/stream.rs`, `crates/sage-core/src/output/**` | Emits `OutputEvent` JSONL for system, assistant, tool, error and result | #81 must map this output into protocol notifications without breaking old consumers |
| Output events | `crates/sage-core/src/output/types/events.rs` | Event enum is output-oriented and lacks thread/turn/request correlation | Protocol needs envelope IDs and lifecycle semantics |
| Execution loop | `crates/sage-core/src/agent/unified/**` | `UnifiedExecutor` owns steps, sessions, tools, input and output strategy | Protocol mapping must observe execution without rewriting the loop in the first slice |
| UI events | `crates/sage-core/src/agent/unified/event_manager/**`, `crates/sage-core/src/ui/bridge/**` | Internal UI events include session, tool and error events | Useful source for notifications, but UI-specific fields must not leak into runtime protocol |
| Input | `crates/sage-core/src/input/request.rs`, `crates/sage-core/src/input/response.rs`, `crates/sage-core/src/input/dto.rs` | Requests model questions, permissions, free text and cancellation, with transport-safe DTOs | Runtime protocol needs request/response correlation for user input and permissions |
| Permissions | `crates/sage-core/src/tools/permission/**`, `crates/sage-core/src/agent/unified/settings_permission.rs` | Permission decisions exist in several runtime paths | #81 names protocol payloads only; #88 unifies policy enforcement later |
| SDK | `crates/sage-sdk/src/client/execution/unified.rs`, `crates/sage-sdk/src/client/result/**` | SDK returns execution result and optional input handle | Future #83 facade should expose protocol stream; #81 defines the contract first |
| Sessions | `crates/sage-core/src/session/**`, `crates/sage-core/src/session/types/unified/**`, `crates/sage-core/src/trajectory/**` | Session recorder, JSONL storage and unified message records exist | #82 will persist protocol items; #81 should keep IDs compatible with backfill |

## 设计方案

This PR is a focused spec PR. It does not change Rust runtime behavior.

Future implementation should add a new core module, likely:

- `crates/sage-core/src/runtime_protocol/mod.rs`
- `crates/sage-core/src/runtime_protocol/envelope.rs`
- `crates/sage-core/src/runtime_protocol/thread.rs`
- `crates/sage-core/src/runtime_protocol/turn.rs`
- `crates/sage-core/src/runtime_protocol/item.rs`
- `crates/sage-core/src/runtime_protocol/permission.rs`
- `crates/sage-core/src/runtime_protocol/error.rs`
- `crates/sage-core/src/runtime_protocol/stream_mapping.rs`

The module should expose typed serde DTOs rather than public `serde_json::Value`
payloads for core protocol variants. Use `serde_json::Value` only for explicitly
bounded metadata/details fields that are documented and redacted.

Compatibility decision: protocol `thread_id` is the durable runtime identity.
Existing `session_id` may be reused as `thread_id` only when the implementation can
prove a one-to-one mapping. Otherwise, #81 implementations should generate a
protocol `thread_id` and preserve the old value as `metadata.legacy_session_id` for
#82 migration/backfill. New code must not require clients to treat `session_id` and
`thread_id` as aliases.

## Protocol Envelope

Every protocol message should use the same envelope shape:

| Field | Required | Notes |
| --- | --- | --- |
| `protocol_version` | yes | `sage.runtime.v0` for this slice |
| `kind` | yes | `request`, `notification`, `response`, or `error` |
| `type` | yes | Dot-separated action/event name such as `turn.started` |
| `id` | yes | Unique message ID |
| `thread_id` | context-dependent | Required after `thread.started`; optional on initial `thread.start` |
| `turn_id` | context-dependent | Required for turn/item/permission events |
| `item_id` | context-dependent | Required for item-level events |
| `request_id` | context-dependent | Required on responses and permission/user-input results |
| `timestamp` | yes | RFC3339 UTC timestamp |
| `sequence` | recommended | Monotonic per-thread sequence when available |
| `source` | yes | `cli`, `sdk`, `runtime`, `tool`, `permission`, or `system` |
| `payload` | yes | Typed payload for the message `type` |
| `metadata` | optional | Redacted, non-contractual metadata such as `legacy_session_id` |

## Message Families

### Requests

- `thread.start`: start an ephemeral or persistent thread.
- `thread.resume`: continue a known `thread_id`.
- `thread.fork`: create a new thread from a parent thread/turn/item.
- `turn.start`: submit user input to a thread.
- `turn.steer`: add steering text or constraints to a running turn.
- `turn.interrupt`: request cancellation/interruption.
- `permission.respond`: answer a permission request.
- `input.respond`: answer a structured user-input request.

### Notifications

- `thread.started`, `thread.ended`
- `turn.started`, `turn.completed`, `turn.interrupted`
- `item.created`, `item.updated`, `item.completed`
- `permission.requested`, `permission.resolved`
- `error.reported`

### Responses

- `thread.start.result`, `thread.resume.result`, `thread.fork.result`
- `turn.start.result`, `turn.steer.result`, `turn.interrupt.result`
- `permission.respond.result`, `input.respond.result`

### Errors

Errors should use both `kind: "error"` and stable error `type` values:

- `error.validation`
- `error.permission_denied`
- `error.tool_failed`
- `error.model_failed`
- `error.interrupted`
- `error.max_steps`
- `error.internal`

## Stream Mapping

Initial implementation should preserve `--stream-json` output and add mapping tests:

| Current `OutputEvent` | Protocol notification |
| --- | --- |
| `system` | `item.created` with `item_type: "system_message"` |
| `assistant` | `item.created` or `item.updated` with `item_type: "assistant_message"` |
| `tool_call_start` | `item.created` with `item_type: "tool_call"` and `status: "started"` |
| `tool_call_result` | `item.completed` with `item_type: "tool_call"` |
| `user_prompt` | `item.created` with `item_type: "user_message"` |
| `error` | `error.reported` plus terminal `turn.completed` if execution stops |
| `result` | `turn.completed` and `item.created` with `item_type: "result"` |

The first implementation can expose the mapper as a pure function:

```rust
RuntimeNotification::from_output_event(event, RuntimeCorrelation { thread_id, turn_id, sequence })
```

Do not route all CLI/SDK execution through this protocol in #81. That belongs to #83 after
types and fixtures are accepted.

## Product-to-Test Mapping

| Product invariant | Implementation area | Verification |
| --- | --- | --- |
| Stable envelope and typed payloads | `runtime_protocol` DTOs | serde round-trip tests and schema snapshot |
| Thread/turn/item lifecycle | `thread.rs`, `turn.rs`, `item.rs` | fixture tests for start/resume/fork, turn completion, item lifecycle |
| Permission correlation | `permission.rs`, input mapping | tests for permission requested/resolved/denied with `request_id` |
| Structured errors | `error.rs` | tests for validation, denied, tool failed, interrupted, max steps |
| `--stream-json` compatibility | `stream_mapping.rs`, CLI stream tests | old JSONL fixture still parses; mapped protocol fixture matches expected |
| UI/input/outcome mapping | event/input/outcome mapping helpers | tests cover core `AgentEvent`, `InputRequestDto`/`InputResponseDto`, and terminal `ExecutionOutcome` variants |
| No excluded App/IDE scope | specs and PR text | literal scope scan |
| No forbidden typo | specs, issue/PR title/body | literal typo scan |

## 数据流

1. CLI or SDK creates a runtime request such as `thread.start` or `turn.start`.
2. Runtime assigns correlation IDs and emits `thread.started` / `turn.started`.
3. Existing executor events and output events map to protocol item notifications.
4. Permission or user-input requests emit request-correlated protocol notifications.
5. Execution terminates with `turn.completed`, `turn.interrupted`, or a structured error.
6. #82 later persists the same envelope as indexed thread/turn/item records.
7. #83 later exposes these protocol messages through a shared runtime API facade.

## 备选方案

- Reuse current `OutputEvent` as the runtime protocol: rejected because it lacks request/response
  direction, thread/turn correlation, permission payloads and stable error codes.
- Define a store schema first: rejected because #82 depends on the protocol contract.
- Change `--stream-json` immediately: rejected because existing CLI/SDK consumers may depend on it.
- Include App/IDE-specific fields: rejected by explicit scope.

## 风险

- Compatibility: adding a new protocol must not silently change `--stream-json`.
- Security: permission and tool payloads may contain secrets; schema must require redaction or explicit
  truncation metadata for logged details.
- Data migration: #82 backfill depends on stable IDs and lifecycle event names.
- API drift: #83 should not expose a different request model than #81.

## 测试计划

- Unit tests: serde round-trip for every request/notification/response/error DTO.
- Snapshot/schema tests: compare generated schema or committed fixture against `specs/GH81/fixtures/runtime_protocol_v0.schema.json`.
- Fixture tests: parse every JSONL fixture in `specs/GH81/fixtures/` and fail loudly on malformed JSONL.
- Mapping tests: convert representative `OutputEvent` values into protocol notifications.
- Compatibility mapping tests: compare legacy stream events against `runtime_protocol_v0_legacy_stream_mapping.jsonl`.
- Permission tests: round-trip `InputRequestDto` and `InputResponseDto` through protocol request/notification/response fixtures.
- Error tests: normalize validation, permission denied, tool failed, model failed, interrupted, max steps and internal errors.
- CLI compatibility tests: verify current `--stream-json` fixture still parses unchanged.
- Integration tests: run a non-interactive task and assert event ordering: thread, turn, items, result/error.
- Completion check: `cargo check --workspace --all-targets --all-features`.

## 回滚方案

Revert the #81 spec PR. If an implementation lands and must be reverted, keep old `--stream-json`
behavior as the compatibility fallback and gate protocol exposure behind an opt-in until #83 is ready.
