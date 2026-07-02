# Tech Spec

## Linked Issue

GH-137

## Product Spec

`specs/GH137/product.md`

## Codebase Context

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| LSP 集成 | `crates/sage-tools/src/tools/code_intelligence/lsp` | 已有 LSP 客户端 | 结构化导航底座 |
| 检索工具 | grep/glob（`tools/` 通用） | lexical 检索 | 默认路径，需保留 |
| 工具注册 | `crates/sage-tools/src/tools/mod.rs` | ~90 工具注册 | 新导航工具注册点 |

## 设计方案

1. 基于现有 LSP 客户端，封装 agent 级工具：`GoToDefinition`、`FindReferences`、`SymbolSearch`（可选 `TypeHierarchy`），输入符号/位置，输出 `file:line` 列表。
2. 能力探测：目标语言无 LSP server 时返回结构化的 degraded 结果（枚举区分「no LSP」与「no result」），不退回 grep（U-33/U-29）。
3. 策略文档：在 agent system prompt / 工具描述中说明「中小仓 grep/glob 优先，大仓/跨符号用结构化导航」，避免模型滥用重型工具。
4. 不引入向量库；若未来提议，需在文档中附 U-33 要求的三点论证。

## Product-to-Test Mapping

| Product invariant | Implementation area | Verification |
| --- | --- | --- |
| P1 | 三个导航工具 | Rust fixture 仓的 def/ref/symbol 单测 |
| P2/P3 | 能力探测枚举 | 无 LSP → degraded、有 LSP 无命中 → empty 两类测试 |
| P4 | grep/glob 不变 | 既有检索测试回归 |

## 数据流

agent 调用导航工具→LSP 客户端查询→结果映射为 file:line→返回；LSP 不可用→degraded 信号。

## 备选方案

- tree-sitter 静态符号索引（无需 LSP server）：更省依赖但精度低于 LSP，可作为 LSP 不可用时的中间降级层（未来）。
- 向量/语义检索：U-33 明确要求先用结构化导航，故本 issue 不采用。

## 风险

- Security: 导航只读，风险低；注意路径遍历与 workspace 边界复用现有 file 工具约束。
- Compatibility: 纯新增工具，无破坏。
- Performance: LSP 启动/索引有成本；按需启动并缓存。
- Maintenance: 多语言 LSP server 管理复杂度，先锁 Rust。

## 测试计划

- [ ] Unit tests: def/ref/symbol on Rust fixture；degraded vs empty 区分。
- [ ] Integration tests: 工具注册与 schema 暴露。
- [ ] Manual verification: 在本仓对一个 pub fn 跑 find-references。

## 回滚方案

不注册新工具即回到 grep/glob+LSP 现状；revert 工具封装 commit。
