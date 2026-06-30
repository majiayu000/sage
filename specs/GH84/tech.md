# Tech Spec

## Linked Issue

GH-84

## Product Spec

Link to `product.md`.

## Codebase Context

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| Subagents | `crates/sage-core/src/agent/subagent/**` | Built-in roles, runner, registry and executor concepts exist | Core area for child agent lifecycle |
| Task tools | `crates/sage-tools/src/tools/process/task/**` | Task execution concepts exist | User-facing background/follow-up surface likely lives here |
| Team tools | `crates/sage-tools/src/tools/team/**` | Team coordination helpers exist | May need graph/status integration |
| ThreadStore | `specs/GH82/**` | Planned thread/turn/item/lineage store | Parent-child graph should persist through the same state model |
| Runtime protocol | `specs/GH81/**` | Defines thread and item identity | Agent graph should reuse protocol IDs |

## 设计方案

Future implementation should add a child-agent graph layer near subagent runtime:

- `crates/sage-core/src/agent/subagent/graph.rs`
- `crates/sage-core/src/agent/subagent/background.rs`
- `crates/sage-core/src/agent/subagent/mailbox.rs`
- `crates/sage-core/src/agent/subagent/status.rs`
- `crates/sage-tools/src/tools/process/task/output.rs`

The graph should persist through ThreadStore lineage when GH-82 exists. Before GH-82 implementation, use an explicit in-memory or JSONL-backed adapter with a migration path, not a hidden global-only registry.

## Contract Sketch

- `spawn_child(parent_thread_id, request) -> ChildAgentHandle`
- `list_children(parent_thread_id, depth) -> Vec<ChildAgentSummary>`
- `read_child(agent_path) -> ChildAgentSnapshot`
- `wait_child(agent_path, timeout) -> ChildAgentSnapshot`
- `interrupt_child(agent_path, reason) -> ChildAgentSnapshot`
- `send_follow_up(agent_path, message) -> TurnId`
- `read_output(agent_path, cursor) -> OutputPage`

`AgentPath` should be stable enough for user/tool references, but must resolve to stored IDs internally.

## Product-to-Test Mapping

| Product invariant | Implementation area | Verification |
| --- | --- | --- |
| Parent-child edge persistence | graph + ThreadStore adapter | graph query and restart tests |
| Background execution | background runner | async lifecycle tests |
| TaskOutput reads state | task output tool | status/output/error tests |
| wait/list/interrupt | lifecycle API | timeout/cancel/failure tests |
| follow-up messaging | mailbox + runtime | follow-up starts new child turn test |

## 数据流

1. Parent runtime spawns child agent and writes graph edge.
2. Child emits protocol items into its child thread.
3. Background registry tracks live process/task state while store persists durable summary.
4. TaskOutput reads store plus live registry for current status/output.
5. Follow-up appends a message to the child mailbox and starts a new child turn.

## 备选方案

- Keep only process-local registry: rejected because restart recovery and audit would fail.
- Use tmux/OS threads as orchestrator: rejected by explicit non-goal.
- Bundle role/context fork work here: deferred to GH-85 to keep graph lifecycle reviewable.

## 风险

- Concurrency: child output and parent queries can race.
- Persistence: graph edges must survive process restart.
- Security: follow-up and interrupt must respect permission/tool scope.
- UX: IDs must be stable enough for users without leaking internals.

## 测试计划

- Unit tests for graph edge creation and descendant queries.
- Background lifecycle tests for running/completed/failed/cancelled states.
- TaskOutput tests for status, output cursor, error and final result.
- wait/list/interrupt tests with timeout and invalid state.
- Follow-up tests that create a new child turn.
- Completion check: `cargo check --workspace --all-targets --all-features`.

## 回滚方案

Keep existing subagent execution path as fallback while graph persistence is gated. If graph persistence fails, disable background/follow-up operations and return structured unsupported errors rather than losing child state.
