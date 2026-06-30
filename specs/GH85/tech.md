# Tech Spec

## Linked Issue

GH-85

## Product Spec

Link to `product.md`.

## Codebase Context

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| Subagent types | `crates/sage-core/src/agent/subagent/types/**` | Defines agent type, config, tool access, working directory and running state | Natural home for role schema and fork policy types |
| Built-ins | `crates/sage-core/src/agent/subagent/builtin.rs` | Built-in roles exist | Must preserve defaults |
| Runner | `crates/sage-core/src/agent/subagent/runner.rs` | Spawns/runs subagents | Needs role resolution and context fork input |
| Session/context | `crates/sage-core/src/agent/unified/session_branching.rs`, `session_manager.rs` | Existing session/branching helpers | Source for `all` and `last_n` context policies |
| Permissions | `crates/sage-core/src/tools/permission/**`, `specs/GH88/**` | Permission profile is planned | Tool scope must be an intersection, not escalation |

## 设计方案

Future implementation should add role loading and fork policy near subagent types:

- `crates/sage-core/src/agent/subagent/types/role.rs`
- `crates/sage-core/src/agent/subagent/role_loader.rs`
- `crates/sage-core/src/agent/subagent/fork_context.rs`
- `crates/sage-core/src/agent/subagent/tool_scope.rs`

Role files should be local, path-bounded configuration artifacts. They should not execute code. Unknown fields should fail schema validation until an explicit forward-compat policy exists.

## Role Schema Sketch

Fields:

- `name`
- `description`
- `prompt`
- `tools`
- `model`
- `reasoning`
- `profile`
- `working_directory_policy`
- `fork_context`
- `metadata`

`fork_context` variants:

- `none`
- `all`
- `last_n` with `turns: positive integer`

## Product-to-Test Mapping

| Product invariant | Implementation area | Verification |
| --- | --- | --- |
| Role schema validation | role loader | valid/invalid fixture tests |
| Path boundary | role loader | path escape tests |
| Built-in compatibility | built-in role adapter | default role snapshot tests |
| Context fork modes | fork context builder | none/all/last_n tests |
| Tool scope intersection | tool scope resolver | escalation denial tests |

## 数据流

1. Spawn request names a built-in or custom role and fork policy.
2. Role loader resolves built-in or config file within allowed locations.
3. Tool scope resolver intersects role tools with parent/profile permissions.
4. Context builder prepares none/all/last_n context.
5. Runner starts child agent using resolved prompt/model/tools/context.

## 备选方案

- Keep only enum roles: rejected because custom roles and context control are required.
- Load executable role plugins: rejected because role config should be declarative and safe.
- Let child role override parent permissions: rejected due security risk.

## 风险

- Security: role file path traversal or tool escalation.
- Compatibility: built-in role behavior must not change unexpectedly.
- Context leakage: fork policies must not accidentally expose full parent history when none/last_n was requested.
- Config drift: unknown fields should fail loudly.

## 测试计划

- Role schema fixture tests.
- Built-in role compatibility snapshot tests.
- Path escape and invalid role tests.
- Fork context tests for none/all/last_n.
- Tool escalation denial tests.
- Completion check: `cargo check --workspace --all-targets --all-features`.

## 回滚方案

Keep built-in role enum path as fallback while custom role loading is gated. If custom role validation fails in production, fail the custom spawn request without changing built-in behavior.
