# Task Plan

## Linked Issue

GH-136

## Spec Packet

- Product: `product.md`
- Tech: `tech.md`

## 实现任务

- [ ] `SP136-T01` Owner: agent-core. Done when: core-owned runtime API 初始化 memory manager + 挂载同存储的 learning engine，按 project/storage path 幂等隔离，失败 error 级上报. Verify: `cargo test -p sage-core --lib memory learning`。
- [ ] `SP136-T02` Owner: agent-core. Done when: context_builder 使用 `RecallQuery` 增加 recall 步骤并注入 prompt 专用区段，受上界与 redaction 约束. Verify: `cargo test -p sage-core --lib context`。
- [ ] `SP136-T03` Owner: learning. Done when: 任务收尾先最小化/脱敏，再写入带 outcome 的轨迹，失败先提炼为教训. Verify: `cargo test -p sage-core --lib learning`。
- [ ] `SP136-T04` Owner: config. Done when: memory.enabled 开关生效，关闭回归无注入基线. Verify: `cargo test -p sage-core --lib`。

## 并行拆分

T01（启动）与 T03（收尾写入）文件不重叠可并行；T02 依赖 T01 的全局可用；T04 贯穿，最后接。

## 验证

- `cargo fmt --all -- --check`
- `cargo clippy -p sage-core --all-targets -- -D warnings`
- `cargo test -p sage-core --lib`
- `python3 checks/check_workflow.py --repo <specrail> --spec-dir <repo>/specs/GH136`

## Handoff Notes

recall 相关性信号由 `RecallQuery` 承载；默认开关取值仍是开放问题，建议 opt-in 起步。与 auto_compact 的 token 预算协作是关键风险点。
