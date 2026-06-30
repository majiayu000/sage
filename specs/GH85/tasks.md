# Task Plan

## Linked Issue

GH-85

## Spec Packet

- Product: `product.md`
- Tech: `tech.md`

## 实现任务

- [ ] `SP85-T01` Owner: multi-agent. Done when: role schema and loader support built-in and path-bounded custom role config. Verify: role fixture and path-boundary tests pass.
- [ ] `SP85-T02` Owner: multi-agent. Done when: built-in roles preserve existing prompt/tools/model defaults. Verify: built-in compatibility snapshot tests pass.
- [ ] `SP85-T03` Owner: multi-agent. Done when: fork context builder supports none/all/last_n. Verify: context fork tests pass.
- [ ] `SP85-T04` Owner: security. Done when: child tool scope is parent/profile intersection and role declarations cannot escalate tools. Verify: tool escalation denial tests pass.
- [ ] `SP85-T05` Owner: multi-agent. Done when: model/reasoning/profile overrides are validated and unsupported values fail closed. Verify: override validation tests pass.
- [ ] `SP85-T06` Owner: coordinator. Done when: this focused spec PR links GH-85, excludes marketplace/App/IDE/app-server-client scope, and does not claim implementation completion. Verify: SpecRail packet check, forbidden typo scan and `cargo check --workspace --all-targets --all-features`.

## 并行拆分

GH-85 should follow GH-84 for durable child identity and should align with GH-88 permission profile. Role loader/schema work can be reviewed separately from context fork construction once the shared types are stable.

## 验证

Spec PR verification:

- `git diff --check`
- `python3 checks/check_workflow.py --repo <specrail> --spec-dir <repo>/specs/GH85`
- Forbidden typo scan over `specs/GH85`
- `python3 scripts/check_doc_consistency.py`
- `cargo check --workspace --all-targets --all-features`

Future implementation verification:

- `cargo test -p sage-core subagent_role`
- `cargo test -p sage-core subagent_fork_context`
- `cargo test -p sage-core subagent_tool_scope`
- `cargo check --workspace --all-targets --all-features`

## Handoff Notes

Use `Refs #85` for spec-only PRs. Use a closing keyword only after role loading, fork policy and tool-scope acceptance criteria are implemented.
