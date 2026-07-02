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
   - tool-need 识别：从 trajectory + 任务标注判断「该用工具的任务里 agent 是否表达了工具意图」。
   - tool-call 执行：从 trajectory 的实际 tool_call 记录判断「是否真的发出调用」。
   - 分别汇总，输出 recognition-correct/execution-missing 的 mismatch 率。
3. **可复现**：固定温度/seed（或多采样报方差）；评分用可执行断言（文件状态/输出匹配）起步，可选 grader。
4. **运行面**：本地命令 + 可选 CI job（离线任务子集）；慢任务不进 PR 最短路径。

## Product-to-Test Mapping

| Product invariant | Implementation area | Verification |
| --- | --- | --- |
| P1 | 任务运行器 | 同输入两次运行结果一致的测试 |
| P2 | 指标分离逻辑 | 构造 recognition-correct/execution-missing 轨迹的单测 |
| P3 | trajectory 溯源 | 指标引用的记录存在于本次运行 |
| P4 | 数据驱动任务集 | 新增一个任务文件不改 harness 代码 |

## 数据流

任务集文件→SDK 驱动 agent→trajectory→评分器（断言）+ 指标分离器→报告（pass@1 + 两个 tool 指标）。

## 备选方案

- 直接接 SWE-bench：覆盖真实但重、慢、依赖外部数据；先自建小任务集。
- 纯 LLM grader：灵活但不确定；起步用可执行断言，grader 作为可选。

## 风险

- Security: eval 会执行 agent 工具（bash 等），须在隔离目录/临时仓运行。
- Compatibility: 纯新增开发工具，无运行时影响。
- Performance: eval 慢；隔离出 CI 可选 job。
- Maintenance: 任务集需随能力演进维护。

## 测试计划

- [ ] Unit tests: 指标分离器（recognition vs execution）；评分断言。
- [ ] Integration tests: 端到端跑 1-2 个离线任务出报告。
- [ ] Manual verification: 本地跑任务集看 pass@1 与两个指标。

## 回滚方案

移除 eval crate/harness 即可，无运行时耦合。
