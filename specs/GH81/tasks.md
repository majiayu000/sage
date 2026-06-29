# Task Plan

## Linked Issue

GH-81

## Spec Packet

- Product: `product.md`
- Tech: `tech.md`
- Fixture: `fixtures/runtime_protocol_v0.schema.json`
- Fixture: `fixtures/runtime_protocol_v0_stream.jsonl`
- Fixture: `fixtures/runtime_protocol_v0_legacy_stream_mapping.jsonl`
- Fixture: `fixtures/runtime_protocol_v0_permission_roundtrip.jsonl`
- Fixture: `fixtures/runtime_protocol_v0_structured_error.jsonl`

## 实现任务

- [ ] `SP81-T01` Owner: runtime-protocol. Done when: `sage.runtime.v0` envelope and typed DTO modules exist for request, notification, response and error. Verify: serde round-trip tests pass.
- [ ] `SP81-T02` Owner: runtime-protocol. Done when: thread lifecycle events cover start, resume, fork, started and ended with stable correlation IDs. Verify: thread fixture tests pass.
- [ ] `SP81-T03` Owner: runtime-protocol. Done when: turn lifecycle events cover start, steer, interrupt, started, completed and interrupted. Verify: turn ordering tests pass.
- [ ] `SP81-T04` Owner: runtime-protocol. Done when: item events cover user/assistant/system messages, tool calls, permission, error and result items. Verify: item schema tests pass.
- [ ] `SP81-T05` Owner: security. Done when: permission and error payloads include stable codes, decisions, risk/source metadata and redaction/truncation markers. Verify: denied permission and redacted error tests pass.
- [ ] `SP81-T06` Owner: cli. Done when: current `OutputEvent` values map to protocol notifications without changing default `--stream-json` output. Verify: old stream JSON fixture plus protocol mapping fixture pass.
- [ ] `SP81-T07` Owner: sdk. Done when: SDK unified execution has a documented future hook for protocol streams without replacing `ExecutionResult` in this slice. Verify: SDK contract test documents old and new boundaries.
- [ ] `SP81-T08` Owner: coordinator. Done when: this focused spec PR links #81, includes schema/stream/mapping/permission/error fixtures, keeps App/IDE/app-server-client out of scope, and does not claim implementation completion. Verify: packet validation, JSON validation, forbidden typo scan and `cargo check --workspace --all-targets --all-features`.

## 并行拆分

#81 should remain serial with #82 and #83. Implement `SP81-T01` through `SP81-T06`
before starting persistent ThreadStore work because #82 needs the final envelope and
ID model. #83 should wait until #81 defines request/notification/response/error DTOs.

Within #81, the DTO/schema work and mapping-test planning can be prepared in parallel
as read-only review lanes, but only one writable implementation lane should own
`crates/sage-core/src/runtime_protocol/**` to avoid API drift.

## 验证

Spec PR verification:

- `jq empty specs/GH81/fixtures/runtime_protocol_v0.schema.json`
- `python3 -m json.tool` for each JSONL line in `specs/GH81/fixtures/*.jsonl`
- Literal forbidden typo scan across new spec files and open issue/PR titles.
- `cargo check --workspace --all-targets --all-features`

Future implementation PR verification:

- `cargo test -p sage-core runtime_protocol`
- `cargo test -p sage-core output_event_protocol_mapping`
- `cargo test -p sage-cli stream_json_protocol_mapping`
- `cargo test -p sage-sdk protocol_contract`
- `cargo check --workspace --all-targets --all-features`

## Handoff Notes

This PR should use `Refs #81`, not `Fixes #81`, because it defines the focused
SpecRail packet and fixtures but does not implement runtime protocol code. #81 remains
open until the typed DTOs, mapping tests and compatibility checks are implemented.

Do not include desktop app, IDE entrypoints, or app-server client scope in #81 follow-up PRs.
