# Task Plan

## Linked Issue

GH-137

## Spec Packet

- Product: `product.md`
- Tech: `tech.md`

## 实现任务

- [ ] `SP137-T01` Owner: tools. Done when: stdio JSON-RPC LSP lifecycle（initialize/didOpen/request/timeout/exit）实现并可由 Rust mock server 验证. Verify: `cargo test -p sage-tools code_intelligence::lsp`。
- [ ] `SP137-T02` Owner: tools. Done when: GoToDefinition/FindReferences/SymbolSearch/TypeHierarchy 工具基于真实 LSP request 实现并注册，Rust 可用. Verify: `cargo test -p sage-tools code_intelligence`。
- [ ] `SP137-T03` Owner: tools. Done when: LSP 不可用/capability unsupported 返回 degraded 信号且与 empty 结果可区分. Verify: `cargo test -p sage-tools`。
- [ ] `SP137-T04` Owner: prompts. Done when: 工具描述/策略说明中小仓 lexical 优先、大仓补结构化导航. Verify: `cargo test -p sage-core --lib prompts`。

## 并行拆分

T01/T02/T03 同在 code_intelligence，串行；T04 独立文件可并行。

## 验证

- `cargo fmt --all -- --check`
- `cargo clippy -p sage-tools --all-targets -- -D warnings`
- `cargo test -p sage-tools`
- `python3 checks/check_workflow.py --repo <specrail> --spec-dir <repo>/specs/GH137`

## Handoff Notes

先锁 Rust LSP，且 TypeHierarchy 对 Rust 是必做；其余语言按可用性降级。向量库明确不在范围（U-33）。
