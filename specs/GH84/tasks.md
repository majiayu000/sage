# Task Plan

## Linked Issue

GH-84

## Spec Packet

- Product: `product.md`
- Tech: `tech.md`

## 实现任务

- [ ] `SP84-T01` Owner: multi-agent. Done when: parent-child agent/thread graph types and storage adapter are defined. Verify: graph unit tests pass.
- [ ] `SP84-T02` Owner: multi-agent. Done when: child spawn writes graph edge and returns stable agent path/task id. Verify: spawn lifecycle tests pass.
- [ ] `SP84-T03` Owner: multi-agent. Done when: background execution tracks running/completed/failed/cancelled status and survives restart reconciliation. Verify: background lifecycle tests pass.
- [ ] `SP84-T04` Owner: multi-agent. Done when: TaskOutput reads status, output summary, errors and final result with cursor support. Verify: TaskOutput tests pass.
- [ ] `SP84-T05` Owner: multi-agent. Done when: wait/list/interrupt/follow-up use the graph and return structured errors for timeout/invalid state. Verify: lifecycle and follow-up tests pass.
- [ ] `SP84-T06` Owner: coordinator. Done when: this focused spec PR links GH-84, excludes OS/tmux/App/IDE scope, and does not claim implementation completion. Verify: SpecRail packet check, forbidden typo scan and `cargo check --workspace --all-targets --all-features`.

## 并行拆分

GH-84 depends on GH-82 lineage for durable persistence and should not be implemented as an isolated global registry. GH-85 role/context work should remain separate until graph lifecycle is stable.

## 验证

Spec PR verification:

- `git diff --check`
- `python3 checks/check_workflow.py --repo <specrail> --spec-dir <repo>/specs/GH84`
- Forbidden typo scan over `specs/GH84`
- `python3 scripts/check_doc_consistency.py`
- `cargo check --workspace --all-targets --all-features`

Future implementation verification:

- `cargo test -p sage-core subagent_graph`
- `cargo test -p sage-core subagent_background`
- `cargo test -p sage-tools task_output`
- `cargo check --workspace --all-targets --all-features`

## Handoff Notes

Use `Refs #84` for spec-only PRs. Use a closing keyword only on the implementation PR that satisfies graph, background, output and messaging acceptance.
