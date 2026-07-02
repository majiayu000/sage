# Task Plan

## Linked Issue

GH-121

## Spec Packet

- Product: `product.md`
- Tech: `tech.md`

## 实现任务

- [x] `SP121-T01` Owner: agent-core. Done when: execute_with_permission_check 返回实际执行的 call（去标记），所有 return 点覆盖. Verify: `cargo test -p sage-core --lib step_execution`。
- [x] `SP121-T02` Owner: agent-core. Done when: execute_single_tool 用 executed call 驱动 post_execution 与 record；编辑路径补 undo 追踪. Verify: `cargo test -p sage-core --lib`。

## 并行拆分

同一调用链上的两个文件，串行完成。

## 验证

- `cargo fmt --all -- --check`
- `cargo clippy -p sage-core --all-targets -- -D warnings`
- `cargo test -p sage-core --lib`
- `python3 checks/check_workflow.py --repo <specrail> --spec-dir <repo>/specs/GH121`

## Handoff Notes

undo 追踪的 P4 不变量当前依赖代码审查；若后续为 session_manager 提供测试探针，可补断言。
