# Task Plan

## Linked Issue

GH-138

## Spec Packet

- Product: `product.md`
- Tech: `tech.md`

## 实现任务

- [ ] `SP138-T01` Owner: eval. Done when: 数据驱动任务集 + SDK 驱动 runner，产出可复现 pass@1. Verify: `cargo test -p <eval-crate>`（或 examples 冒烟）。
- [ ] `SP138-T02` Owner: eval. Done when: 指标分离器分别报告 tool-need 识别与 tool-call 执行，来源为本次 trajectory. Verify: 指标分离器单测。
- [ ] `SP138-T03` Owner: eval. Done when: 隔离运行（临时目录/仓）+ 运行与扩展文档. Verify: 端到端离线任务冒烟。

## 并行拆分

T01（runner）与 T02（指标）可在同 crate 内并行开发；T03 依赖 T01。

## 验证

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test`（eval crate / examples）
- `python3 checks/check_workflow.py --repo <specrail> --spec-dir <repo>/specs/GH138`

## Handoff Notes

评分起步用可执行断言；grader 可选。eval 执行 bash 类工具，务必隔离目录运行（安全）。
