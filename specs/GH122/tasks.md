# Task Plan

## Linked Issue

GH-122

## Spec Packet

- Product: `product.md`
- Tech: `tech.md`

## 实现任务

- [x] `SP122-T01` Owner: permissions. Done when: persist_decision 对损坏 settings 返回 Err、保留原文件并输出 error 日志. Verify: `cargo test -p sage-core --lib permission::cache`。
- [x] `SP122-T02` Owner: commands. Done when: /init 写 settings.json 失败时返回失败文案. Verify: `cargo test -p sage-core --lib`。

## 并行拆分

两个任务位于不相交文件，可并行；本 PR 内串行完成。

## 验证

- `cargo fmt --all -- --check`
- `cargo clippy -p sage-core --all-targets -- -D warnings`
- `cargo test -p sage-core --lib`
- `python3 checks/check_workflow.py --repo <specrail> --spec-dir <repo>/specs/GH122`

## Handoff Notes

PermissionCache 栈的接线/删除决策由 GH-125 承载；若删除该栈，本回归测试随栈一并移除。
