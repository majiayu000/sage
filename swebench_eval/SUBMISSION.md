# SWE-bench 提交指南

本文档记录 Sage Agent 在 SWE-bench Lite 上的评估结果和提交所需的所有内容。

## 评估概况

| 指标 | 数值 |
|------|------|
| 数据集 | SWE-bench Lite |
| 总实例数 | 300 |
| 有效 predictions | 300 |
| 有 trajectory 的实例 | 164 |
| 完整数据 (problem + patch + trajectory) | 149 |
| 使用模型 | glm-4.7 |

## 提交文件清单

### 必需文件

| 文件 | 大小 | 说明 |
|------|------|------|
| `predictions_swebench_lite_final.json` | ~465KB | 300 个实例的 predictions，格式：`[{instance_id, model_patch, model_name_or_path}]` |
| `swebench_submission.json` | ~468KB | 与上面相同，用于 SWE-bench 官方提交 |

### Trajectory 文件

| 文件 | 大小 | 说明 |
|------|------|------|
| `swebench_trajectories.tar.xz` | ~4MB | 164 个有效 trajectory 的 JSONL 日志（压缩） |
| `trajectory_summary.json` | ~604KB | trajectory 汇总，包含 problem_statement, patch, model, steps 等 |
| `valid_instances.txt` | ~4KB | 164 个有效实例的 ID 列表 |

## 文件格式说明

### predictions 格式

```json
[
  {
    "instance_id": "django__django-14382",
    "model_patch": "diff --git a/...",
    "model_name_or_path": "sage-agent"
  }
]
```

### trajectory_summary.json 格式

```json
[
  {
    "instance_id": "django__django-14382",
    "repo": "django/django",
    "problem_statement": "问题描述...",
    "base_commit": "abc123...",
    "model": "glm-4.7",
    "steps": 20,
    "trajectory_file": "/path/to/trajectory.jsonl",
    "patch": "diff --git a/...",
    "has_complete_data": true
  }
]
```

### trajectory JSONL 格式

每个 `.jsonl` 文件包含多行 JSON，记录完整的 agent 执行过程：

```jsonl
{"type": "session_start", "session_id": "...", "task": "...", "model": "glm-4.7", ...}
{"type": "llm_request", "uuid": "...", "messages": [...]}
{"type": "llm_response", "uuid": "...", "content": "...", "tool_calls": [...]}
{"type": "tool_result", "uuid": "...", "result": "..."}
...
```

## 按仓库统计

| 仓库 | 有效实例 | 总实例 | 完成率 |
|------|----------|--------|--------|
| django/django | 76 | 76 | 100% |
| astropy/astropy | 21 | 24 | 87.5% |
| scikit-learn/scikit-learn | 16 | 16 | 100% |
| matplotlib/matplotlib | 15 | 15 | 100% |
| sympy/sympy | 8 | 65 | 12.3% |
| pylint-dev/pylint | 6 | 6 | 100% |
| pytest-dev/pytest | 5 | 5 | 100% |
| mwaskom/seaborn | 4 | 4 | 100% |
| pydata/xarray | 4 | 4 | 100% |
| sphinx-doc/sphinx | 4 | 4 | 100% |
| pallets/flask | 3 | 3 | 100% |
| psf/requests | 2 | 2 | 100% |

## 提交到 SWE-bench Leaderboard

### 1. Fork 官方仓库

```bash
git clone https://github.com/SWE-bench/experiments.git
cd experiments
```

### 2. 创建提交目录

```bash
mkdir -p evaluation/lite/$(date +%Y%m%d)_sage-agent
```

### 3. 复制文件

```bash
# predictions
cp predictions_swebench_lite_final.json evaluation/lite/.../all_preds.jsonl

# 创建 trajs 目录并解压 trajectory
mkdir -p evaluation/lite/.../trajs
tar -xvf swebench_trajectories.tar.xz -C evaluation/lite/.../trajs
```

### 4. 创建 metadata.yaml

```yaml
model_name: sage-agent
model_type: agent
base_model: glm-4.7
date: 2025-01-06
authors:
  - name: Your Name
    email: your@email.com
description: |
  Sage Agent is a Rust-based LLM agent system for software engineering tasks.
  Built with async architecture on Tokio, featuring multi-LLM support,
  rich tool ecosystem, and trajectory recording.
```

### 5. 运行验证

```bash
python -m analysis.get_results evaluation/lite/$(date +%Y%m%d)_sage-agent
```

### 6. 创建 Pull Request

提交到 SWE-bench/experiments 仓库。

## 本地评估

### 使用 Docker 官方评估

```bash
# 构建镜像
python -m swebench.harness.prepare_images \
    --dataset_name princeton-nlp/SWE-bench_Lite \
    --instance_ids django__django-14382 \
    --env_image_tag latest --tag latest

# 运行评估
python -m swebench.harness.run_evaluation \
    --dataset_name princeton-nlp/SWE-bench_Lite \
    --predictions_path predictions_swebench_lite_final.json \
    --run_id sage_eval
```

### 使用本地评估（快速验证）

```bash
python evaluate_local.py predictions_swebench_lite_final.json
```

## 合并多台电脑的结果

如果在多台电脑上运行了评估，可以按以下步骤合并：

### 1. 收集各台电脑的文件

- `trajectory_summary.json`
- `swebench_trajectories.tar.xz`
- `predictions_*.json`

### 2. 合并 predictions

```python
import json

all_predictions = {}
for pred_file in ["pred_pc1.json", "pred_pc2.json"]:
    with open(pred_file) as f:
        for p in json.load(f):
            # 按 instance_id 去重，保留有 patch 的
            if p["instance_id"] not in all_predictions or p.get("model_patch"):
                all_predictions[p["instance_id"]] = p

with open("merged_predictions.json", "w") as f:
    json.dump(list(all_predictions.values()), f, indent=2)
```

### 3. 合并 trajectory_summary

```python
import json

all_trajectories = {}
for traj_file in ["traj_pc1.json", "traj_pc2.json"]:
    with open(traj_file) as f:
        for t in json.load(f):
            # 按 instance_id 去重，保留步数多的
            if t["instance_id"] not in all_trajectories or t["steps"] > all_trajectories[t["instance_id"]]["steps"]:
                all_trajectories[t["instance_id"]] = t

with open("merged_trajectory_summary.json", "w") as f:
    json.dump(list(all_trajectories.values()), f, indent=2)
```

## 参考链接

- [SWE-bench 官网](https://www.swebench.com/)
- [SWE-bench GitHub](https://github.com/SWE-bench/SWE-bench)
- [SWE-bench experiments](https://github.com/SWE-bench/experiments)
- [提交 checklist](https://github.com/SWE-bench/experiments/blob/main/checklist.md)
