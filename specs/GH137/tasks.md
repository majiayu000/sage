# Task Plan

## Linked Issue

GH-137

## Spec Packet

- Product: `product.md`
- Tech: `tech.md`

## 实现任务

- [ ] `SP137-T01` Owner: tools. Done when: GoToDefinition/FindReferences/SymbolSearch 工具基于现有 LSP 客户端实现并注册，Rust 可用. Verify: `cargo test -p sage-tools code_intelligence`。
- [ ] `SP137-T02` Owner: tools. Done when: LSP 不可用返回 degraded 信号且与 empty 结果可区分. Verify: `cargo test -p sage-tools`。
- [ ] `SP137-T03` Owner: prompts. Done when: 工具描述/策略说明中小仓 lexical 优先、大仓补结构化导航. Verify: `cargo test -p sage-core --lib prompts`。

## 并行拆分

T01/T02 同在 code_intelligence，串行；T03 独立文件可并行。

## 验证

- `cargo fmt --all -- --check`
- `cargo clippy -p sage-tools --all-targets -- -D warnings`
- `cargo test -p sage-tools`
- `python3 checks/check_workflow.py --repo <specrail> --spec-dir <repo>/specs/GH137`

## Handoff Notes

先锁 Rust LSP；其余语言按可用性降级。向量库明确不在范围（U-33）。
