# SWE-bench Evaluation TODO

## Overview

SWE-bench Lite 共 300 个实例，分批运行评估。

## Progress

| 批次 | 范围 | 命令 | 状态 | 备注 |
|-----|------|------|------|------|
| 1 | 0-49 | `--offset 0 --limit 50` | ✅ 已完成 | 46 predictions |
| 2 | 50-99 | `--offset 50 --limit 50` | ✅ 已完成 | 合并到批次1 |
| 3 | 100-149 | `--offset 100 --limit 50` | ⚠️ 中断 | 21/50 尝试，API限额 |
| 4 | 150-199 | `--offset 150 --limit 50` | ⚠️ 中断 | 19/50 尝试，API限额 |
| 5 | 200-249 | `--offset 200 --limit 50` | ✅ 已完成 | |
| 6 | 250-299 | `--offset 250 --limit 50` | ⚠️ 中断 | 20/50 (GLM限额) |

**当前统计**: 180+ 个预测，批次6中断于 20/50 (补丁率100%)

**2024-12-28 更新**:
- 实现 Claude Code 风格安全特性 (validation, violations 模块)
- 批次6使用新版本 sage (含安全增强) 运行中
- 当前处理: sympy 系列实例

**2024-12-27 备注**: 因 API 5小时限额中断批次3和4，已从 git diff 恢复 19 个补丁

## Setup

```bash
cd swebench_eval

# 创建虚拟环境 (使用 Python 3.10)
uv venv --python 3.10

# 安装依赖
uv pip install datasets
```

## Commands

### 运行 Agent 生成补丁

```bash
cd swebench_eval

# 批次 3: 100-149
nohup uv run run_agent.py --offset 100 --limit 50 --output predictions_101_150.json --timeout 900 --max-retries 1 </dev/null > swebench_101_150.log 2>&1 &

# 批次 4: 150-199
nohup uv run run_agent.py --offset 150 --limit 50 --output predictions_151_200.json --timeout 900 --max-retries 1 </dev/null > swebench_151_200.log 2>&1 &

# 批次 5: 200-249
nohup uv run run_agent.py --offset 200 --limit 50 --output predictions_201_250.json --timeout 900 --max-retries 1 </dev/null > swebench_201_250.log 2>&1 &

# 批次 6: 250-299
nohup uv run run_agent.py --offset 250 --limit 50 --output predictions_251_300.json --timeout 900 --max-retries 1 </dev/null > swebench_251_300.log 2>&1 &
```

### 监控进度

```bash
# 查看日志
tail -f swebench_101_150.log

# 查看已完成实例数
ls swebench_runs/ | wc -l

# 查看运行状态
ps aux | grep run_agent

# 查看特定批次的补丁数量
cat predictions_101_150.json | uv run python -c "import json,sys; print(len(json.load(sys.stdin)))"
```

### 合并所有预测结果

```bash
uv run python << 'EOF'
import json
from pathlib import Path

all_predictions = []
for f in sorted(Path(".").glob("predictions_*.json")):
    if f.name != "predictions_all.json":
        with open(f) as fp:
            all_predictions.extend(json.load(fp))

# 去重
seen = set()
unique = []
for p in all_predictions:
    if p["instance_id"] not in seen:
        seen.add(p["instance_id"])
        unique.append(p)

with open("predictions_all.json", "w") as f:
    json.dump(unique, f, indent=2)

print(f"Total unique predictions: {len(unique)}")
EOF
```

## Official Evaluation

### 1. 启动 Docker Desktop

### 2. 运行官方评估

```bash
# 方式一：使用 run_evaluation.py
uv run run_evaluation.py evaluate predictions_all.json

# 方式二：直接使用 swebench CLI
uv run python -m swebench.harness.run_evaluation \
    --dataset_name princeton-nlp/SWE-bench_Lite \
    --predictions_path predictions_all.json \
    --max_workers 4 \
    --run_id sage_eval_$(date +%Y%m%d)
```

### 3. 查看结果

```bash
# 结果保存在
ls ~/.swebench/logs/sage_eval_*/

# 解析结果
uv run python -c "
import json
with open('~/.swebench/logs/sage_eval_*/results.json') as f:
    results = json.load(f)
resolved = sum(1 for r in results.values() if r['resolved'])
total = len(results)
print(f'Resolved: {resolved}/{total} = {resolved/total*100:.1f}%')
"
```

## Scoring Formula

```
% Resolved = (通过的实例数 / 总实例数) × 100%
```

**通过条件：**
1. `FAIL_TO_PASS` 测试全部通过（修复了问题）
2. `PASS_TO_PASS` 测试全部通过（没有破坏其他功能）

## Reference Scores (SWE-bench Lite)

| 系统 | % Resolved |
|-----|-----------|
| OpenAI o1 + Agentless | ~50% |
| Claude 3.5 Sonnet + tools | ~40% |
| GPT-4 + SWE-agent | ~20% |

## Notes

- 每个实例 timeout: 900 秒 (15 分钟)
- 预计每批 50 个实例需要 2-4 小时
- 可以并行运行多批加速（注意 API 限流）
