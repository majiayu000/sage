# Tech Spec

## Linked Issue

GH-83

## Product Spec

Link to `product.md`.

## Codebase Context

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| CLI args | `crates/sage-cli/src/args.rs`, `crates/sage-cli/src/commands/unified/**` | Handles print, continue, resume, stream JSON and unified execution setup | Runtime facade must preserve CLI semantics |
| SDK execution | `crates/sage-sdk/src/client/execution/unified.rs`, `crates/sage-sdk/src/client/options/**` | SDK constructs `UnifiedExecutor` and input channel directly | SDK should route through the facade contract |
| Core executor | `crates/sage-core/src/agent/unified/**` | Owns execution loop, session manager, tools, inputs and output strategy | Initial facade should wrap this rather than duplicate loop logic |
| Protocol | `specs/GH81/**` | Defines runtime request/notification/response/error | Facade contract must use this protocol vocabulary |
| State | `specs/GH82/**`, `crates/sage-core/src/session/**` | ThreadStore is planned; session state exists today | Facade should define seam for persistent state without blocking initial API |

## 设计方案

Future implementation should introduce a core runtime facade module:

- `crates/sage-core/src/runtime/mod.rs`
- `crates/sage-core/src/runtime/facade.rs`
- `crates/sage-core/src/runtime/request.rs`
- `crates/sage-core/src/runtime/response.rs`
- `crates/sage-core/src/runtime/stream.rs`
- `crates/sage-core/src/runtime/status.rs`
- `crates/sage-core/src/runtime/error.rs`

The facade should initially wrap `UnifiedExecutor` construction and execution. It should not fork a second execution loop. CLI and SDK should call the same facade builder with different presentation/output adapters.

## API Sketch

Core API shape:

- `Runtime::new(config, tools, options) -> Runtime`
- `runtime.start(StartRequest) -> RuntimeHandle`
- `runtime.resume(ResumeRequest) -> RuntimeHandle`
- `runtime.fork(ForkRequest) -> RuntimeHandle`
- `runtime.interrupt(ThreadId | TurnId) -> RuntimeResponse`
- `runtime.status(ThreadId) -> RuntimeStatus`
- `RuntimeHandle::events() -> Stream<RuntimeNotification>`
- `RuntimeHandle::result() -> RuntimeResult`

The facade should accept typed requests that can map to GH-81 protocol messages. It may expose legacy adapters:

- CLI adapter for text/json/stream-json output.
- SDK adapter preserving `execute_unified` and `execute_non_interactive`.
- Test adapter that captures protocol events for contract tests.

## Compatibility Rules

- Keep `OutputEvent` stream JSON stable unless a new explicit protocol stream flag is introduced.
- Keep SDK `ExecutionResult` stable while adding a future protocol-event hook.
- Keep resume/continue CLI argument parsing stable; only move execution wiring behind facade.
- Return structured unsupported errors for operations whose lower-level implementation is not ready.

## Product-to-Test Mapping

| Product invariant | Implementation area | Verification |
| --- | --- | --- |
| CLI behavior preserved | CLI adapter and unified command | snapshot/contract tests for print/continue/resume/stream-json |
| SDK behavior preserved | SDK execution adapter | SDK contract tests for interactive and non-interactive calls |
| Single execution boundary | runtime facade wrapping `UnifiedExecutor` | code review plus tests asserting shared setup path |
| Protocol compatibility | runtime stream adapter | event fixture tests against GH-81 |
| State seam | runtime state abstraction | tests with ephemeral state and mocked ThreadStore |

## 数据流

1. CLI or SDK parses user input into a runtime request.
2. Runtime facade resolves config, tools, state mode and output adapter.
3. Facade starts/resumes/forks by wrapping `UnifiedExecutor` with the correct options.
4. Execution emits legacy output and optional protocol notifications.
5. Runtime result is adapted back to CLI exit/output or SDK `ExecutionResult`.

## 备选方案

- Keep CLI and SDK direct executor construction: rejected because setup drift is already the problem.
- Rewrite `UnifiedExecutor` first: rejected because facade can wrap the existing loop with less risk.
- Introduce server/client handler first: rejected because user explicitly excluded app-server client scope.

## 风险

- Compatibility: CLI flags and SDK return types are user-facing and need snapshot coverage.
- Double abstraction: facade must reduce duplicate wiring, not add a parallel runtime.
- State readiness: GH-82 may not be implemented yet; facade needs an ephemeral/null state mode.
- Streaming: old `--stream-json` must remain parseable while protocol events are introduced.

## 测试计划

- CLI contract tests for print, continue, resume and stream JSON modes.
- SDK contract tests for non-interactive and interactive input handle behavior.
- Unit tests for runtime request validation and structured unsupported errors.
- Protocol stream mapping tests using GH-81 fixtures.
- Completion check: `cargo check --workspace --all-targets --all-features`.

## 回滚方案

Keep direct CLI/SDK executor paths behind compatibility adapters until the facade path is proven. If facade rollout fails, disable the new path while preserving existing CLI/SDK behavior.
