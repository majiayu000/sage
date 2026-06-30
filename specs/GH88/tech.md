# Tech Spec

## Linked Issue

GH-88

## Product Spec

Link to `product.md`.

## Codebase Context

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| Permission settings | `crates/sage-core/src/settings/types/permissions.rs` | Permission-related settings are defined in settings types | Natural home for profile DTO and merge rules |
| Unified execution | `crates/sage-core/src/agent/unified/{settings_permission,step_execution_permissions}.rs` | Unified executor computes execution permissions | Needs to call central decision engine |
| Permission handlers | `crates/sage-core/src/tools/permission/**` | Tool permission policy and prompts are handled here | Should become the single decision path |
| Sandbox | `crates/sage-core/src/sandbox/**` | Platform sandbox config/execution is separate | Must enforce platform fail-closed semantics |
| Bash process tools | `crates/sage-tools/src/tools/process/bash/**` | Bash has sync/background execution paths | Must not bypass permission profile |
| Background process | `crates/sage-tools/src/tools/process/{task,task_output}.rs` | Background execution exists | Needs profile propagation and auditability |

## 设计方案

Future implementation should centralize permission decisions:

- `crates/sage-core/src/permissions/profile.rs`
- `crates/sage-core/src/permissions/decision_engine.rs`
- `crates/sage-core/src/permissions/approval_cache.rs`
- `crates/sage-core/src/permissions/platform_support.rs`

Existing settings, tool permission handlers and sandbox execution should become adapters into the central engine. The engine should return a structured decision rather than a boolean.

## Decision Model Sketch

Decision inputs:

- action kind: filesystem, network, exec, sandbox
- path/command/network target
- caller tool id
- thread/session id
- permission profile source stack
- approval cache state
- platform sandbox support

Decision outputs:

- `allow`
- `deny`
- `ask`
- `unsupported`
- `needs_approval`
- structured reason and source provenance

## Product-to-Test Mapping

| Product invariant | Implementation area | Verification |
| --- | --- | --- |
| Unified profile | settings/profile merge | precedence fixture tests |
| Deny before allow | decision engine | conflicting rule tests |
| Bash cannot bypass | bash execution adapter | sync/background Bash deny tests |
| Platform fail closed | sandbox adapter | unsupported platform tests |
| Approval semantics | approval cache | allow/deny/timeout tests |

## 数据流

1. Settings and runtime inputs merge into a `PermissionProfile`.
2. Tool request builds a decision input with action metadata.
3. Decision engine applies deny-before-allow rules and approval state.
4. Sandbox adapter validates platform support before execution.
5. Bash and background process tools execute only after an allow decision.
6. Deny/ask/unsupported outcomes are surfaced as structured runtime errors or prompts.

## 备选方案

- Keep separate per-tool permission checks: rejected because it enables inconsistent behavior and bypasses.
- Warn and continue on unsupported sandbox: rejected because this silently weakens security.
- Let Bash sanitize commands without central profile: rejected because command safety and execution permission are separate concerns.

## 风险

- Security: any unadapted tool path can bypass the central engine.
- Compatibility: stricter fail-closed behavior may expose previously hidden unsafe behavior.
- Platform variance: macOS/Linux sandbox support differs and needs explicit tests.
- UX: ask/deny/unsupported states must be clear enough for recovery.

## 测试计划

- Profile merge and precedence tests.
- Decision engine tests for deny-before-allow conflicts.
- Bash sync/background permission tests.
- Workspace write allow and outside/protected path deny tests.
- Network deny and approval timeout tests.
- Platform unsupported sandbox fail-closed tests.
- Completion check: `cargo check --workspace --all-targets --all-features`.

## 回滚方案

Keep existing permission handler APIs as adapters while the central engine is gated. If central engine integration fails, block only the affected action with a structured error instead of falling back to unrestricted execution.
