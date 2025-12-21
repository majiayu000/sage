# SWE-bench 官方评估工具

本目录包含用于运行 SWE-bench 官方评估的脚本。

## 快速开始

### 1. 安装依赖

```bash
pip install swebench datasets
```

### 2. 运行评估（完整流程）

```bash
# 方式一：运行 Agent 并评估（一步到位）
python run_agent.py --repo django/django --limit 5

# 方式二：分步执行
# Step 1: 运行 Agent 生成补丁
python run_agent.py --instances django__django-14382 django__django-11099

# Step 2: 运行官方评估
python run_evaluation.py evaluate swebench_runs/predictions.json
```

### 3. 从已有测试目录提取补丁

如果你已经手动运行了测试，可以提取补丁：

```bash
python run_evaluation.py extract ../swebench_test --output predictions.json
```

## 使用说明

### run_agent.py - 运行 Agent 生成补丁

```bash
# 运行所有 Django 问题（限制 10 个）
python run_agent.py --repo django/django --limit 10

# 运行特定问题
python run_agent.py --instances django__django-14382 sympy__sympy-17022

# 运行 SWE-bench Verified 数据集
python run_agent.py --dataset SWE-bench/SWE-bench_Verified --limit 20

# 自定义参数
python run_agent.py \
    --repo django/django \
    --limit 5 \
    --max-steps 30 \
    --timeout 900 \
    --output my_predictions.json
```

参数说明：
- `--dataset`: 数据集名称（默认: princeton-nlp/SWE-bench_Lite）
- `--instances`: 指定实例 ID 列表
- `--repo`: 按仓库筛选（如 django/django）
- `--limit`: 最大实例数量
- `--max-steps`: Agent 最大步数（默认: 25）
- `--timeout`: 每个实例超时时间（默认: 600s）
- `--output`: 输出文件名

### run_evaluation.py - 运行官方评估

```bash
# 运行评估
python run_evaluation.py evaluate predictions.json

# 指定特定实例
python run_evaluation.py evaluate predictions.json --instances django__django-14382

# 查看结果
python run_evaluation.py results sage_eval_20241221_120000
```

参数说明：
- `--dataset`: 数据集名称
- `--workers`: 并行工作进程数（默认: 4）
- `--run-id`: 运行 ID（用于标识结果）
- `--timeout`: 每个实例评估超时（默认: 1800s）

## 输出格式

### predictions.json

```json
[
  {
    "instance_id": "django__django-14382",
    "model_patch": "diff --git a/django/core/management/templates.py ...",
    "model_name_or_path": "sage-agent"
  }
]
```

### 评估结果

评估完成后，结果保存在 `~/.swebench/logs/<run_id>/` 目录：
- `results.json`: 每个实例的通过/失败状态
- `<instance_id>/`: 每个实例的详细日志

## 评估流程

```
┌─────────────────────────────────────────────────────────────┐
│                    SWE-bench 评估流程                        │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  1. 加载实例                                                │
│     └── 从 HuggingFace 下载 SWE-bench 数据集                │
│                                                             │
│  2. 设置环境                                                │
│     ├── 克隆仓库                                           │
│     └── 切换到 base_commit                                 │
│                                                             │
│  3. 运行 Agent                                              │
│     ├── 读取 problem_statement                             │
│     ├── 执行修复                                           │
│     └── 生成补丁 (git diff)                                │
│                                                             │
│  4. 官方评估                                                │
│     ├── 应用补丁到 base_commit                             │
│     ├── 应用 test_patch (官方测试)                         │
│     └── 运行测试套件                                       │
│                                                             │
│  5. 统计结果                                                │
│     └── 计算通过率                                         │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## 数据集说明

| 数据集 | 问题数 | 说明 |
|-------|-------|------|
| SWE-bench_Lite | 300 | 精选的 300 个问题 |
| SWE-bench_Verified | 500 | 人工验证的 500 个问题 |
| SWE-bench (完整) | 2,294 | 完整数据集 |

## 常见问题

### Q: 评估时间很长怎么办？

A: 每个实例需要：
- 克隆仓库 (~30s)
- 运行 Agent (~60-300s)
- 运行测试 (~60-300s)

建议先用少量实例测试，确认无误后再跑完整评估。

### Q: 如何只评估特定问题？

```bash
python run_agent.py --instances django__django-14382 django__django-11099
python run_evaluation.py evaluate predictions.json --instances django__django-14382
```

### Q: 评估失败怎么排查？

查看日志：
```bash
ls ~/.swebench/logs/<run_id>/
cat ~/.swebench/logs/<run_id>/<instance_id>/test_output.txt
```

### Q: 本地评估和官方评估的区别？

| 特性 | 本地评估 (evaluate_local.py) | 官方评估 (Docker) |
|------|------------------------------|-------------------|
| 依赖隔离 | ❌ 使用系统 Python | ✅ 每个实例独立环境 |
| 准确性 | 可能有依赖冲突 | 完全准确 |
| 速度 | 较快 | 较慢（需构建镜像）|
| 用途 | 快速验证补丁格式 | 正式基准测试 |

**建议**：
- 开发调试时使用本地评估
- 提交正式结果时使用 Docker 官方评估

## 当前测试结果

已完成的测试（手动验证）：

| Instance ID | Patch Applied | Status | Notes |
|-------------|--------------|--------|-------|
| django__django-11099 | ✅ | ✅ Pass | 与官方补丁一致 |
| django__django-11179 | ✅ | ✅ Pass | 与官方补丁一致 |
| django__django-13933 | ✅ | ✅ Pass | 与官方补丁一致 |
| django__django-14382 | ✅ | ✅ Pass | 与官方补丁一致 |
| sympy__sympy-17022 | ✅ | ✅ Pass | 功能验证通过 |

**说明**：以上结果为手动验证，非官方 SWE-bench 评估。正式评估需使用 Docker 环境。

## 快速命令参考

```bash
# 激活环境
cd swebench_eval
source .venv/bin/activate

# 1. 生成补丁（运行 Agent）
python run_agent.py --repo django/django --limit 5

# 2. 提取现有补丁
python run_evaluation.py extract ../swebench_test

# 3. 本地验证（快速）
python evaluate_local.py predictions.json

# 4. 官方评估（需要先构建镜像）
python -m swebench.harness.prepare_images \
    --dataset_name princeton-nlp/SWE-bench_Lite \
    --instance_ids django__django-14382 \
    --env_image_tag latest --tag latest

python -m swebench.harness.run_evaluation \
    --dataset_name princeton-nlp/SWE-bench_Lite \
    --predictions_path predictions.json \
    --instance_ids django__django-14382 \
    --run_id my_test
```

## 已知问题

1. **Docker 镜像构建网络问题**：在 Docker 容器内克隆大型仓库可能失败
   - 解决方案：重试或使用代理

2. **ARM64 平台**：SWE-bench 镜像是 x86_64，在 Apple Silicon 上通过 Rosetta 运行
   - 可能导致速度较慢
