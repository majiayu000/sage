# Product Spec

## Linked Issue

GH-137

## 用户问题

Sage 的代码检索靠 grep/glob + LSP，缺少结构化导航。在大代码库上 agent 容易定位错符号或迷失，任务成功率下降。

## 目标

- 在已有 LSP 基础上向 agent 暴露结构化导航：go-to-definition、find-references、symbol search、type hierarchy（至少 Rust）。
- 检索策略遵循 U-33：中小仓默认 lexical（grep/glob），大仓补结构化导航。
- LSP 不可用时明确报告降级，而非静默退回 grep-only。

## 非目标

- 不引入向量库 / embedding / RAG（除非附 staleness/权限/成本论证）。
- 不改变现有 grep/glob 工具语义。
- 不承诺全语言结构化导航。

## Behavior Invariants

1. agent 可调用 go-to-definition、find-references、symbol search、type hierarchy，返回文件+行位置；Rust 必须可用。
2. LSP 对目标语言不可用时，工具返回明确的「degraded: LSP unavailable」信号，不静默返回空或退回 grep（U-29/U-33）。
3. 结构化导航结果为空（符号不存在）与 LSP 不可用（能力缺失）是两种可区分的返回。
4. 现有 grep/glob 行为不变。

## 验收标准

- [ ] go-to-definition / find-references / symbol search / type hierarchy 对 Rust 可用并返回位置。
- [ ] 检索策略文档化：中小仓 lexical 优先，大仓补结构化导航。
- [ ] LSP 不可用时明确降级报告。

## 边界情况

- 多语言混合仓：按语言的 LSP 可用性分别降级。
- 符号在依赖/生成代码中：返回位置但标注来源，或按配置排除。

## 发布说明

新增 agent 可见的导航工具；文档需说明依赖 LSP 与支持语言范围。

## 开放问题

- 复用现有 `code_intelligence/lsp` 到何种程度 vs 新增工具封装？由 tech spec 定。
