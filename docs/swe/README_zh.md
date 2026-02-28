# SWE-bench 测试追踪

本目录记录 Sage Agent 在 SWE-bench 基准测试上的表现。

## 目录结构

```
docs/swe/
├── README.md              # 本文件
├── RESULTS.md             # 详细测试结果
├── LEADERBOARD.md         # 成绩排行榜
└── problems/              # 各问题详细记录
    ├── django__django-11099.md
    ├── django__django-11179.md
    └── ...
```

## 快速导航

- [测试结果汇总](RESULTS.md)
- [成绩排行榜](LEADERBOARD.md)

## 关于 SWE-bench

SWE-bench 是一个用于评估大语言模型解决真实软件工程问题能力的基准测试。

- **SWE-bench Full**: 2294 个问题
- **SWE-bench Verified**: 500 个经人工验证的问题
- **SWE-bench Lite**: 300 个精选问题

### 难度分类

根据 OpenAI 的人工评估：

| 难度等级 | 预估时间 | 问题数量 |
|---------|---------|---------|
| 简单 | < 15 分钟 | 196 |
| 中等 | 15 分钟 - 1 小时 | ~250 |
| 较难 | 1 - 4 小时 | 42 |
| 困难 | > 4 小时 | 3 |

## 测试配置

| 配置项 | 值 |
|-------|-----|
| Agent 版本 | Sage Agent v0.1.0 |
| 默认 LLM | GLM-4.6 |
| 最大步数 | 15 (可调) |
| 超时时间 | 5 分钟/任务 |
