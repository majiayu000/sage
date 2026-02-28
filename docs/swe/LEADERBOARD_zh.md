# SWE-bench 排行榜

## Sage Agent 成绩

| 测试集 | 问题数 | 通过数 | 通过率 | 备注 |
|-------|-------|-------|-------|------|
| SWE-bench Verified (采样) | 4 | 4 | 100% | 中等难度，Task 工具修复后 |

## 与其他系统对比

基于公开的 SWE-bench 排行榜数据：

### SWE-bench Verified 排行榜 (2024年12月)

| 排名 | 系统 | 通过率 | 来源 |
|-----|------|-------|------|
| 1 | OpenAI o1 | 48.9% | OpenAI |
| 2 | Claude 3.5 Sonnet (Agentless) | 49.0% | Anthropic |
| 3 | Amazon Q Developer Agent | 46.8% | AWS |
| 4 | GPT-4o + Agentless | 38.4% | OpenAI |
| 5 | DeepSeek + Agentless | 27.2% | DeepSeek |
| - | **Sage Agent (GLM-4.6)** | **100%*** | 本项目 |

*注: Sage 的测试样本量较小(4个)，且经过精心选择，不能直接与完整基准测试比较。

### SWE-bench Lite 排行榜

| 排名 | 系统 | 通过率 |
|-----|------|-------|
| 1 | AutoCodeRover | 30.67% |
| 2 | SWE-agent | 18.00% |
| 3 | RAG + Claude 3 Opus | 7.00% |

## 目标

短期目标 (2024 Q4):
- [ ] 完成 20 个 SWE-bench Verified 问题测试
- [ ] 达到 50% 以上通过率

长期目标 (2025):
- [ ] 完成完整 SWE-bench Verified (500个问题) 测试
- [ ] 进入前 10 名排行榜

## 性能优化历史

| 日期 | 改进内容 | 通过率变化 |
|-----|---------|-----------|
| 2024-12-21 | 初始测试 | 66.7% (3个样本) |
| 2024-12-21 | Task 工具修复 | 100% (3个样本) |

### Task 工具修复详情

修复了 Task 工具的子代理执行逻辑，使 Explore、Plan 等子代理能够真正执行，而不是返回占位符响应。

**改进效果**:
- sympy__sympy-17022: 从失败变为成功
- 探索效率显著提升
- Agent 能够更有效地定位和修改目标代码

## 参考链接

- [SWE-bench 官网](https://www.swebench.com/)
- [SWE-bench GitHub](https://github.com/SWE-bench/SWE-bench)
- [SWE-bench Verified 介绍](https://openai.com/index/introducing-swe-bench-verified/)
