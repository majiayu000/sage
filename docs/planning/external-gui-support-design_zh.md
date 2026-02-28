# External GUI Integration Design (Backend Support from Sage Core)

## 1. Background

The GUI will be implemented in a separate repository.
This repository (`sage`) should serve as the execution backend and domain runtime.

This design document defines:

- What `sage` can already provide to an external GUI
- What minimal changes are recommended in `sage` for stable integration
- A contract model (commands/events/input/session) for cross-repo collaboration

---

## 2. Goals and Non-Goals

### Goals

- Reuse existing `sage-core` and `sage-sdk` capabilities without rewriting agent logic
- Enable an external GUI to drive task execution, display real-time progress, and handle user interaction
- Keep backend and frontend decoupled so each can evolve independently

### Non-Goals

- Rebuilding terminal UI in this repository
- Defining frontend visual design details
- Introducing breaking API changes unless necessary

---

## 3. Existing Backend Capabilities in This Repo

### 3.1 Task Execution Engine

- `SageAgentSdk::execute_unified(...)`
- `SageAgentSdk::execute_non_interactive(...)`

These provide end-to-end agent execution with tools, MCP, and session support.

### 3.2 UI-State Event Model (Framework-Independent)

- `AgentEvent` (event vocabulary)
- `AppState` (UI state model)
- `EventAdapter` + `subscribe()` (state updates via `watch::Receiver`)
- `UiContext` + `EventSink` (dependency injection for UI frameworks)

This is the key foundation for external GUI rendering.

### 3.3 Interactive Input Protocol

- `InputChannel` / `InputChannelHandle`
- `InputRequestKind` / `InputResponseKind`

Supports:

- Structured questions (AskUserQuestion)
- Permission decisions
- Free text prompts
- Cancellation

### 3.4 Session Persistence and Resume

- `set_jsonl_storage(...)`
- `enable_session_recording()`
- `restore_session(session_id)`
- `get_most_recent_session()`

Storage format already supports session metadata and message history.

### 3.5 Model and Tool Runtime

- `switch_model(model)`
- Rich default tools from `sage-tools::get_default_tools()`
- MCP loading and runtime registration paths already implemented

---

## 4. Recommended Integration Architecture

### 4.1 Preferred Pattern

Use this repository as a backend runtime with one of two integration modes:

1. `Library Mode` (Rust GUI stack)
2. `Local Service Mode` (non-Rust GUI stack, e.g. Electron/Tauri/Web)

### 4.2 Library Mode (Same Process)

Best when GUI runtime is Rust-native.

Flow:

- GUI starts execution via `SageAgentSdk` / `UnifiedExecutor`
- GUI registers custom `EventSink`
- GUI receives events/state updates and renders
- GUI handles `InputRequest` and replies via `InputChannelHandle`

Pros:

- Lowest latency
- No transport layer needed
- Strong type safety

Cons:

- GUI repo must be Rust-heavy
- Tighter build/runtime coupling

### 4.3 Local Service Mode (Separate Process)

Best when GUI stack is JS/TS or mixed technology.

Flow:

- Start backend worker process (local only)
- GUI sends commands via IPC/HTTP/WebSocket
- Backend emits events/streaming updates
- GUI responds to input/permission requests via API

Pros:

- Frontend stack freedom
- Clean separation of concerns
- Easier independent release cycle

Cons:

- Need protocol definition and transport maintenance
- Slightly higher latency/complexity

---

## 5. Cross-Repo Contract (Command/Event Model)

The external GUI should depend on a stable backend contract.

### 5.1 Commands (GUI -> Backend)

- `start_task`
- `cancel_task`
- `respond_input`
- `switch_model`
- `list_sessions`
- `resume_session`
- `continue_recent_session`
- `get_runtime_status`

### 5.2 Events (Backend -> GUI)

Use `AgentEvent` as canonical source and expose transport-safe DTOs:

- Session lifecycle: started/ended
- Step lifecycle: step_started
- Thinking lifecycle: thinking_started/stopped
- Streaming lifecycle: content_stream_started/chunk/ended
- Tool lifecycle: tool_execution_started/completed
- User interaction: user_input_requested
- Error: error_occurred
- Context updates: git_branch_changed, working_directory_changed

### 5.3 State Snapshot

Expose `AppState` snapshots for reconnect/recovery use cases.

---

## 6. Required Improvements in This Repo (Before External GUI GA)

These are high-impact cleanup items to reduce terminal coupling.

### 6.1 Remove Deprecated Global Event Path

Current code still contains deprecated global event emission paths:

- `output/strategy/rnk.rs` uses `emit_event(...)`
- unified execution error path also calls legacy `emit_event(...)`

Action:

- Migrate all event emission to `UiContext` + `EventSink` only
- Keep one event pipeline for CLI and external GUI

### 6.2 Remove Direct Terminal I/O in Core Interaction Paths

Current code still prints directly in some core paths:

- Ask-user question flow (`println!`)
- Permission prompt flow (stdin/stdout dialog)

Action:

- Route these interactions entirely through `InputChannel`
- Let UI layer (CLI or external GUI) decide presentation

### 6.3 Add Transport DTO Layer (for Service Mode)

Action:

- Add serializable DTOs for events, state, input requests/responses
- Avoid exposing internal Rust-only types directly across process boundary

---

## 7. Milestone Plan

### M1 - Contract and Runtime Baseline

- Freeze command/event contract
- Implement/verify single event path (`UiContext`)
- Remove direct terminal I/O from core interaction flows

### M2 - Adapter Layer

- Provide backend adapter for:
  - Library mode (`EventSink` implementation template)
  - Service mode (local API + streaming channel)

### M3 - Session and Recovery

- Expose session list/resume APIs
- Expose state snapshot/replay support

### M4 - Stabilization

- Add integration tests for:
  - streaming correctness
  - input/permission loop
  - cancel/retry behavior
  - session resume continuity

---

## 8. Testing Strategy

### 8.1 Contract Tests

- Command acceptance/rejection behavior
- Event ordering guarantees
- Input request/response correlation by request ID

### 8.2 Integration Tests

- Start task -> stream content -> tool calls -> complete
- AskUserQuestion roundtrip
- Permission allow/deny roundtrip
- Cancel during LLM streaming and tool execution

### 8.3 Regression Tests

- Session recording and resume equivalence
- Model switching correctness
- MCP tool registration consistency

---

## 9. Risks and Mitigations

### Risk: Event path divergence (legacy vs injected)

Mitigation:

- One mandatory event source (`UiContext`)
- Add lint/CI check to prevent new global `emit_event` usage

### Risk: Hidden terminal dependencies in core logic

Mitigation:

- Move all user interaction to `InputChannel` abstraction
- Add tests that run without TTY

### Risk: Contract drift between backend and GUI repos

Mitigation:

- Versioned backend contract
- Compatibility matrix in release notes

---

## 10. Immediate Action Checklist

- [ ] Remove legacy global event emission from unified execution paths
- [ ] Replace direct `println!/stdin` interaction in core with `InputChannel`
- [ ] Introduce serializable event/input DTOs for service mode
- [ ] Add a minimal external integration example (library or local service)
- [ ] Add contract tests for command/event/input APIs

---

## 11. Summary

This repository is already strong enough to be the backend runtime for an external GUI.
The core execution, event model, input protocol, sessions, and tool ecosystem are present.

The main work left is decoupling the last terminal-specific paths and defining a stable contract layer for cross-repo integration.
