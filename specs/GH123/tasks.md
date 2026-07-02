# Task Plan

## Linked Issue

GH-123

## Spec Packet

- Product: `product.md`
- Tech: `tech.md`

## 实现任务

- [x] `SP123-T01` Owner: CLI. Done when: 三个 fetch 失败分支输出结构化 warn 日志并复用统一回退闭包. Verify: `cargo clippy -p sage-cli --all-targets -- -D warnings`。

## 并行拆分

单文件单任务，无并行需求。

## 验证

- `cargo fmt --all -- --check`
- `cargo clippy -p sage-cli --all-targets -- -D warnings`
- `cargo test -p sage-cli`
- `python3 checks/check_workflow.py --repo <specrail> --spec-dir <repo>/specs/GH123`

## Handoff Notes

ModelCatalogManager 接线与 UX 提示由 GH-124 跟踪。
