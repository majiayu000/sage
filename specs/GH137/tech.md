# Tech Spec

## Linked Issue

GH-137

## Product Spec

`specs/GH137/product.md`

## Codebase Context

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| LSP 集成 | `crates/sage-tools/src/tools/code_intelligence/lsp` | 现有 `LspClient` 只是配置/可用性占位，operations 返回说明文本或 regex fallback，尚无 stdio JSON-RPC server lifecycle | 需要补真实协议底座 |
| 检索工具 | grep/glob（`tools/` 通用） | lexical 检索 | 默认路径，需保留 |
| 工具注册 | `crates/sage-tools/src/tools/mod.rs` | ~90 工具注册 | 新导航工具注册点 |

## 设计方案

1. **真实 LSP lifecycle**：实现 stdio JSON-RPC client：按需启动 server、发送 `initialize`/`initialized`、`textDocument/didOpen`，并为请求维护 id/timeout/进程退出错误；Rust 先接 `rust-analyzer`。占位说明文本不满足验收。
2. **agent 级工具**：封装并注册 `GoToDefinition`、`FindReferences`、`SymbolSearch`、`TypeHierarchy` 四个工具，输入符号/位置，输出结构化 `file:line` 列表；`TypeHierarchy` 对 Rust 为必做，不是可选项。
3. **能力探测**：目标语言无 LSP server、server 启动失败、capability 不支持时返回结构化 degraded 结果（枚举区分「no LSP / capability unsupported / no result」），不退回 grep（U-33/U-29）。
4. **策略文档**：在 agent system prompt / 工具描述中说明「中小仓 grep/glob 优先，大仓/跨符号用结构化导航」，避免模型滥用重型工具。
5. 不引入向量库；若未来提议，需在文档中附 U-33 要求的三点论证。

## Product-to-Test Mapping

| Product invariant | Implementation area | Verification |
| --- | --- | --- |
| P1 | 四个导航工具 + JSON-RPC lifecycle | Rust fixture 仓的 def/ref/symbol/type-hierarchy 单测；mock LSP server 断言 initialize/didOpen/request 顺序 |
| P2/P3 | 能力探测枚举 | 无 LSP → degraded、capability unsupported → degraded、有 LSP 无命中 → empty 三类测试 |
| P4 | grep/glob 不变 | 既有检索测试回归 |

## 数据流

agent 调用导航工具→按语言启动/复用 LSP server→initialize/didOpen→JSON-RPC request→结果映射为 file:line→返回；LSP 不可用/不支持→degraded 信号。

## 备选方案

- tree-sitter 静态符号索引（无需 LSP server）：更省依赖但精度低于 LSP，可作为 LSP 不可用时的中间降级层（未来）。
- 向量/语义检索：U-33 明确要求先用结构化导航，故本 issue 不采用。

## 风险

- Security: 导航只读，风险低；注意路径遍历与 workspace 边界复用现有 file 工具约束。
- Compatibility: 纯新增工具，无破坏。
- Performance: LSP 启动/索引有成本；按需启动并按 workspace/language 缓存，超时必须返回 degraded。
- Maintenance: 多语言 LSP server 管理复杂度，先锁 Rust。

## 测试计划

- [ ] Unit tests: def/ref/symbol/type-hierarchy on Rust fixture；mock JSON-RPC lifecycle；degraded vs empty 区分。
- [ ] Integration tests: 工具注册与 schema 暴露。
- [ ] Manual verification: 在本仓对一个 pub fn 跑 find-references。

## 回滚方案

不注册新工具即回到 grep/glob+LSP 现状；revert 工具封装 commit。
