# Product Spec

## Linked Issue

GH-138

## 用户问题

Sage 只有单元测试，没有 agent 级质量度量。工具选择、多步任务成功率、以及「识别到该用工具」与「真的调用了工具」之间的差距都无法量化，能力演进只能靠人眼。

## 目标

- 一个最小 eval harness，对固定任务集跑 agent 并产出可复现 pass@1。
- 指标分离：分别报告 tool-need 识别与 tool-call 执行（W-38）。
- 可本地/CI 运行，作为能力回归护栏。

## 非目标

- 不追求 SWE-bench 全量；先小而稳。
- 不引入隐藏状态探针；从 trajectory 度量。
- 不把慢 eval 塞进每个 PR 最短路径。

## Behavior Invariants

1. harness 对固定任务集运行 agent，产出确定、可复现的 pass@1（同输入同结果）。
2. 报告分别给出 tool-need 识别指标与 tool-call 执行指标，不合并为单一 tool-use accuracy（W-38）。
3. 所有指标来源可追溯到本次运行产生的 trajectory / 工具调用记录（W-16/W-38），非记忆或历史。
4. 任务集与评分标准以数据/配置形式存在，可新增任务而不改 harness 代码。

## 验收标准

- [ ] 固定任务集可跑出可复现 pass@1。
- [ ] 报告分离 tool-need 识别与 tool-call 执行两个指标。
- [ ] 指标来源为本次 trajectory。
- [ ] 有运行与新增任务的说明文档。

## 边界情况

- 任务依赖 LLM 不确定性：用固定 seed/温度或多次取样并报告方差。
- 无网络/无 key 的 CI：提供离线可跑的任务子集或明确 skip 并声明。

## 发布说明

新增开发者工具，不影响运行时行为；文档说明如何运行与扩展任务集。

## 开放问题

- 评分用精确匹配还是 grader（LLM/断言）？由 tech spec 定，倾向可执行断言起步。
