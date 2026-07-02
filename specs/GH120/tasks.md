# Task Plan

## Linked Issue

GH-120

## Spec Packet

- Product: `product.md`
- Tech: `tech.md`

## 实现任务

- [x] `SP120-T01` Owner: security. Done when: shell_safety 模块提供元字符检测、命令分段与部分通配判定. Verify: `cargo test -p sage-core --lib permissions::shell_safety`。
- [x] `SP120-T02` Owner: security. Done when: decision engine 的 allow/deny 匹配对 Bash key 使用 bash-aware 包装. Verify: `cargo test -p sage-core --lib settings_permission_shell`。
- [x] `SP120-T03` Owner: security. Done when: allow 逃逸/deny 绕过/全信任/精确匹配均有回归测试. Verify: `cargo test -p sage-core --lib`。

## 并行拆分

单一作者的关联改动（新模块 + 匹配层 + 测试），串行完成。

## 验证

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p sage-core --lib`
- `cargo test --package sage-core --test architecture_guards`
- `python3 checks/check_workflow.py --repo <specrail> --spec-dir <repo>/specs/GH120`

## Handoff Notes

带引号元字符按 fail-closed 处理（allow 降级 Ask）；若未来需要精确 shell 解析，可在 shell_safety 内替换实现而不改调用点。
