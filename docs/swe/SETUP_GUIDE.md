# SWE-bench 测试环境设置指南

## 方案一：使用 HuggingFace 数据集 (推荐)

### 1. 安装依赖

```bash
pip install datasets gitpython
```

### 2. 下载数据集并设置测试环境

```python
#!/usr/bin/env python3
"""
SWE-bench 测试环境设置脚本
"""

import os
import json
import subprocess
from datasets import load_dataset

# 加载 SWE-bench Lite 数据集
dataset = load_dataset("princeton-nlp/SWE-bench_Lite", split="test")

# 筛选 Django 中等难度问题
django_problems = [
    item for item in dataset
    if item['repo'] == 'django/django'
]

print(f"找到 {len(django_problems)} 个 Django 问题")

# 显示前 10 个问题
for i, problem in enumerate(django_problems[:10]):
    print(f"\n{i+1}. {problem['instance_id']}")
    print(f"   Base commit: {problem['base_commit']}")
    print(f"   问题描述: {problem['problem_statement'][:100]}...")
```

### 3. 自动设置测试环境的脚本

```python
#!/usr/bin/env python3
"""
setup_swebench_test.py - 自动设置 SWE-bench 测试环境
"""

import os
import subprocess
from datasets import load_dataset

def setup_test_env(instance_id: str, output_dir: str = "swebench_test"):
    """设置单个测试环境"""

    # 加载数据集
    dataset = load_dataset("princeton-nlp/SWE-bench_Lite", split="test")

    # 查找指定问题
    problem = None
    for item in dataset:
        if item['instance_id'] == instance_id:
            problem = item
            break

    if not problem:
        print(f"未找到问题: {instance_id}")
        return False

    # 创建目录
    test_dir = os.path.join(output_dir, instance_id)
    os.makedirs(test_dir, exist_ok=True)

    # 克隆仓库
    repo_url = f"https://github.com/{problem['repo']}.git"
    print(f"克隆仓库: {repo_url}")

    subprocess.run([
        "git", "clone", "--depth", "100", repo_url, test_dir
    ], check=True)

    # 切换到基础 commit
    print(f"切换到 commit: {problem['base_commit']}")
    subprocess.run([
        "git", "-C", test_dir, "checkout", problem['base_commit']
    ], check=True)

    # 写入问题描述
    problem_file = os.path.join(test_dir, "PROBLEM_STATEMENT.md")
    with open(problem_file, 'w') as f:
        f.write(f"# {instance_id}\n\n")
        f.write(problem['problem_statement'])

    # 写入测试信息
    test_info = {
        "instance_id": problem['instance_id'],
        "repo": problem['repo'],
        "base_commit": problem['base_commit'],
        "test_patch": problem.get('test_patch', ''),
        "hints_text": problem.get('hints_text', ''),
    }

    info_file = os.path.join(test_dir, "test_info.json")
    with open(info_file, 'w') as f:
        json.dump(test_info, f, indent=2)

    print(f"✅ 测试环境已设置: {test_dir}")
    return True

# 使用示例
if __name__ == "__main__":
    import sys
    if len(sys.argv) > 1:
        setup_test_env(sys.argv[1])
    else:
        # 默认设置一些 Django 问题
        problems = [
            "django__django-11099",
            "django__django-11179",
            "django__django-13933",
            "django__django-12286",
            "django__django-13590",
        ]
        for p in problems:
            try:
                setup_test_env(p)
            except Exception as e:
                print(f"设置 {p} 失败: {e}")
```

## 方案二：使用 SWE-bench 官方工具

### 1. 安装 SWE-bench

```bash
pip install swebench
```

### 2. 下载测试实例

```bash
# 下载 SWE-bench Lite
python -m swebench.harness.prepare_instance \
    --dataset_name princeton-nlp/SWE-bench_Lite \
    --instance_id django__django-11099
```

### 3. 运行评估

```bash
python -m swebench.harness.run_evaluation \
    --dataset_name princeton-nlp/SWE-bench_Lite \
    --predictions_path predictions.json \
    --max_workers 4 \
    --run_id my_test
```

## 方案三：批量下载脚本

```bash
#!/bin/bash
# download_swebench.sh - 批量下载 SWE-bench 测试

OUTPUT_DIR="swebench_test"
mkdir -p $OUTPUT_DIR

# Django 问题列表 (SWE-bench Lite 中的)
PROBLEMS=(
    "django__django-11099:ea071870f9"
    "django__django-11179:19fc6376ce"
    "django__django-11283:c5e373d48c"
    "django__django-11422:21e1d39c95"
    "django__django-11564:b330b918e9"
    "django__django-11620:514efa3129"
    "django__django-11742:fee75d2aed"
    "django__django-11815:9b224579f9"
    "django__django-11848:3346b78a8a"
    "django__django-11905:2f72480fbd"
)

for item in "${PROBLEMS[@]}"; do
    IFS=':' read -r problem_id base_commit <<< "$item"
    echo "设置 $problem_id..."

    dir="$OUTPUT_DIR/$problem_id"

    if [ -d "$dir" ]; then
        echo "  已存在，跳过"
        continue
    fi

    # 克隆
    git clone --depth 50 https://github.com/django/django.git "$dir"

    # 切换 commit
    cd "$dir"
    git fetch --depth 100 origin $base_commit
    git checkout $base_commit
    cd ../..

    echo "  ✅ 完成"
done

echo "所有测试环境已设置完成"
```

## 方案四：使用 Python 脚本列出所有可用问题

```python
#!/usr/bin/env python3
"""list_problems.py - 列出所有 SWE-bench 问题"""

from datasets import load_dataset

# 加载数据集
print("加载 SWE-bench Lite...")
dataset = load_dataset("princeton-nlp/SWE-bench_Lite", split="test")

# 按仓库分组
repos = {}
for item in dataset:
    repo = item['repo']
    if repo not in repos:
        repos[repo] = []
    repos[repo].append(item['instance_id'])

# 显示统计
print(f"\n总共 {len(dataset)} 个问题\n")
print("按仓库分布:")
for repo, problems in sorted(repos.items(), key=lambda x: -len(x[1])):
    print(f"  {repo}: {len(problems)} 个")

# 导出问题列表
print("\n\nDjango 问题列表:")
for p in repos.get('django/django', []):
    print(f"  - {p}")
```

## 快速开始

```bash
# 1. 安装依赖
pip install datasets gitpython

# 2. 运行设置脚本
python setup_swebench_test.py django__django-12286

# 3. 复制配置文件
cp sage_config.json swebench_test/django__django-12286/

# 4. 运行测试
cd swebench_test/django__django-12286
sage unified "$(cat PROBLEM_STATEMENT.md)" --max-steps 20
```

## 推荐的测试问题

### Django (简单-中等)
| 问题 ID | 描述 | 难度 |
|---------|------|------|
| django__django-11099 | UsernameValidator 换行符 | 简单 |
| django__django-11179 | delete() pk 清除 | 简单 |
| django__django-13933 | ModelChoiceField 错误消息 | 中等 |
| django__django-12286 | 密码帮助文本翻译 | 中等 |
| django__django-13590 | FloatField 验证 | 中等 |

### SymPy (中等)
| 问题 ID | 描述 | 难度 |
|---------|------|------|
| sympy__sympy-17022 | Identity 矩阵 lambdify | 中等 |
| sympy__sympy-18087 | trigsimp 问题 | 中等 |

### Requests (简单)
| 问题 ID | 描述 | 难度 |
|---------|------|------|
| requests__requests-3362 | 准备请求方法 | 简单 |

## 注意事项

1. **网络问题**: 如果 GitHub 克隆失败，可以使用镜像或代理
2. **依赖安装**: 某些仓库需要特定 Python 版本和依赖
3. **测试验证**: 使用官方 test_patch 验证补丁正确性
