# SWE-bench 评估分析报告

## 运行概览

- **日期**: 2025-12-29
- **总实例数**: 50
- **耗时**: 2小时14分

## 结果统计

| 状态 | 数量 | 百分比 |
|------|------|--------|
| 成功 (生成 patch) | 31 | 62% |
| Git Clone 失败 | 10 | 20% |
| API 限流 (429) | 8 | 16% |
| 无 Patch | 1 | 2% |

## 失败原因分析

### 1. Git Clone 失败 (10 个)

网络问题或并发克隆同一仓库导致冲突：

- astropy__astropy-7746
- django__django-10924
- django__django-11001
- django__django-11019
- django__django-11039
- django__django-11049
- django__django-11283
- django__django-11422
- django__django-11583
- django__django-11815

### 2. API 限流 (8 个)

GLM API 达到 5 小时使用上限：

- django__django-13315
- django__django-13321
- django__django-13401
- django__django-13447
- django__django-13448
- django__django-13551
- django__django-13590
- django__django-13658

### 3. 无 Patch (1 个)

Agent 运行但未能使用 Edit 工具：

- django__django-11620 (只使用了 TodoWrite)

## 工具使用统计

### 全局统计

| 工具 | 调用次数 |
|------|----------|
| bash | 316 |
| Grep | 222 |
| Read | 188 |
| TodoWrite | 111 |
| Edit | 66 |
| Glob | 42 |
| Write | 35 |
| task_done | 27 |
| sequentialthinking | 7 |
| codebase-retrieval | 2 |

### 成功案例工具使用模式

成功案例平均使用：
- bash: 10 次
- Grep: 7 次
- Read: 5.8 次
- TodoWrite: 3.5 次
- Edit: 2.1 次
- Glob: 1.3 次

## 成功案例列表

| 实例 ID | 主要工具 |
|---------|----------|
| astropy__astropy-6938 | TodoWrite, Glob, Read, Edit |
| django__django-11630 | Grep(12), Read(5), Edit(3) |
| django__django-11742 | Grep(17), Read(8), bash(6) |
| django__django-11797 | bash(18), Read(6), Grep(5) |
| django__django-11848 | bash(28), Read(3), Edit(1) |
| django__django-11905 | Grep(3), Read(2), Edit(1) |
| django__django-11910 | Grep(18), bash(16), Read(8) |
| django__django-11964 | bash(15), Read(9), Grep(9) |
| django__django-11999 | bash(14), Read(7), Edit(7) |
| django__django-12113 | Grep(6), Read(4), Edit(1) |
| django__django-12125 | bash(28), Read(13), Grep(11) |
| django__django-12184 | bash(17), Read(9), Grep(6) |
| django__django-12284 | bash(24), Grep(12), Read(4) |
| django__django-12286 | bash(8), Read(4), Edit(3) |
| django__django-12308 | Glob(3), Read(3), Edit(1) |
| django__django-12453 | TodoWrite(4), Edit(2), Read(1) |
| django__django-12470 | Grep(14), Read(11), bash(5) |
| django__django-12497 | bash(6), Read(4), Edit(3) |
| django__django-12700 | bash(40), Read(4), Edit(3) |
| django__django-12708 | Read(12), bash(8), Grep(8) |
| django__django-12747 | Grep(2), Read(3), Edit(1) |
| django__django-12856 | Grep(11), bash(8), Read(7) |
| django__django-12908 | Grep(12), Read(9), bash(4) |
| django__django-12915 | Read(4), Glob(3), Edit(1) |
| django__django-12983 | bash(22), Grep(3), Read(3) |
| django__django-13028 | Grep(4), Read(4), Edit(1) |
| django__django-13033 | Grep(12), Read(11), bash(9) |
| django__django-13158 | Grep(17), bash(9), Read(5) |
| django__django-13220 | bash(9), Read(6), Edit(5) |
| django__django-13230 | Read(5), Edit(2), Grep(1) |
| django__django-13265 | Grep(14), bash(6), Read(5) |

## 问题和建议

### 发现的问题

1. **Git Clone 并发冲突**
   - 多个实例同时克隆同一仓库导致失败
   - 建议: 添加仓库锁机制或预先克隆仓库

2. **API 限流**
   - GLM API 有 5 小时使用上限
   - 建议: 使用多个 API key 轮换，或使用其他 provider

3. **Agent 未完成任务**
   - 部分案例 agent 只做了规划未执行
   - 建议: 增强 prompt 强调必须使用 Edit 工具

### 工具使用观察

1. **bash 使用过多**
   - 某些案例 bash 调用超过 20 次
   - 可能是在尝试测试或验证

2. **Grep 和 Read 是核心**
   - 成功案例大量使用 Grep 和 Read 进行代码搜索和阅读

3. **Edit 使用适中**
   - 平均每个成功案例约 2 次 Edit
   - 说明修复通常只需要少量代码修改

## 需要重跑的实例

### Git Clone 失败 (10 个)
```
astropy__astropy-7746
django__django-10924
django__django-11001
django__django-11019
django__django-11039
django__django-11049
django__django-11283
django__django-11422
django__django-11583
django__django-11815
```

### API 限流 (8 个)
```
django__django-13315
django__django-13321
django__django-13401
django__django-13447
django__django-13448
django__django-13551
django__django-13590
django__django-13658
```

### 无 Patch (1 个)
```
django__django-11620
```
