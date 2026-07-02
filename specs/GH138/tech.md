# Tech Spec

## Linked Issue

GH-138

## Product Spec

`specs/GH138/product.md`

## Codebase Context

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| Trajectory 记录 | `crates/sage-core/src/trajectory/` | 已记录执行轨迹 | 指标来源 |
| SDK 执行入口 | `crates/sage-sdk/src/client/execution/` | 可编程驱动 agent | harness 驱动 agent 的入口 |
| 工具调用记录 | session/trajectory 中的 tool_call/tool_result | 已有 | tool-call 执行指标来源 |

## 设计方案

1. 新增最小 eval crate 或 `examples/eval` harness：读任务集（数据文件：prompt + 期望断言），用 SDK 驱动 agent，收集 trajectory。
2. **指标分离（W-38）**：
   - tool-need 识别：任务标注声明 required tool category；agent 在执行前必须写入独立的 `tool_intent` trajectory 事件（计划/选择的工具类别与理由），该指标只读 `tool_intent`，不得从实际 `tool_call` 反推识别成功。
   - tool-call 执行：从 trajectory 的实际 tool_call 记录判断「是否真的发出调用」。
   - 分别汇总，输出 recognition-correct/execution-missing 的 mismatch 率。
3. **可复现**：pass@1 必须使用固定模型配置、温度/seed（若 provider 支持）和一次采样；可选 multi-sampling 只能作为附加 variance 报告，不能替代 deterministic pass@1 验收。评分用可执行断言（文件状态/输出匹配）起步，可选 grader。
4. **运行面与安全**：本地命令 + 可选 CI job（离线任务子集）；每个任务在临时 workspace 中运行，并强制 deny-by-default permission profile / sandbox，仅显式 allow 任务需要的只读/写入路径与命令。慢任务不进 PR 最短路径。

## Product-to-Test Mapping

| Product invariant | Implementation area | Verification |
| --- | --- | --- |
| P1 | 任务运行器 | deterministic pass@1 同输入两次运行结果一致的测试；multi-sampling 仅附加报告 |
| P2 | 指标分离逻辑 | 构造 `tool_intent` 正确但 `tool_call` 缺失、`tool_call` 存在但无 `tool_intent` 的轨迹单测 |
| P3 | trajectory 溯源 | 指标引用的记录存在于本次运行 |
| P4 | 数据驱动任务集 | 新增一个任务文件不改 harness 代码 |

## 数据流

任务集文件→创建 sandboxed 临时 workspace + deny-by-default permission profile→SDK 驱动 agent→`tool_intent` + `tool_call` trajectory→评分器（断言）+ 指标分离器→报告（deterministic pass@1 + 两个 tool 指标，可选 variance）。

## 备选方案

- 直接接 SWE-bench：覆盖真实但重、慢、依赖外部数据；先自建小任务集。
- 纯 LLM grader：灵活但不确定；起步用可执行断言，grader 作为可选。

## 风险

- Security: eval 会执行 agent 工具（bash 等），须在隔离目录/临时仓运行，并使用 deny-by-default permission profile / sandbox；临时目录本身不构成足够隔离。
- Compatibility: 纯新增开发工具，无运行时影响。
- Performance: eval 慢；隔离出 CI 可选 job。
- Maintenance: 任务集需随能力演进维护。

## 测试计划

- [ ] Unit tests: 指标分离器（独立 `tool_intent` vs execution）；评分断言；permission profile 默认拒绝未授权工具。
- [ ] Integration tests: 端到端跑 1-2 个离线任务出报告。
- [ ] Manual verification: 本地跑任务集看 pass@1 与两个指标。

## 回滚方案

移除 eval crate/harness 即可，无运行时耦合。
