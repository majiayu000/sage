# SWE-bench 评估恢复命令

## 当前状态 (2024-12-27)

| 指标 | 数值 |
|-----|------|
| 总运行目录 | 148 |
| 有效补丁数 | 65 (已恢复合并) |
| 补丁有效率 | 100% (65/65 非空) |
| 完成进度 | 65/300 = 21.7% |

## 执行日志分析

### 批次 3 (100-149) 分析

| 实例范围 | 状态 | 说明 |
|---------|------|------|
| 1-3 | ✅ 成功 | 生成补丁 |
| 4 | ❌ 失败 | 无补丁 |
| 5-15 | ✅ 成功 | 大部分生成补丁 |
| 16-21+ | ❌ 失败 | API 限额触发 |

**成功实例**: django__django-15819, 15851, 15902, 16041, 16046, 16139, 16229, 16255, 16379, 16400, 16408, 16527, 16595, 16816 (14个)

### 批次 4 (150-199) 分析

| 实例范围 | 状态 | 说明 |
|---------|------|------|
| 1-5 | ✅ 成功 | 生成补丁 |
| 6 | ❌ 失败 | psf__requests-863 |
| 7 | ✅ 成功 | pydata__xarray-3364 |
| 8 | ⏰ 超时 | pydata__xarray-4094 (900s) |
| 9-20+ | ❌ 失败 | API 限额触发 |

**成功实例**: psf__requests-1963, 2148, 2317, 2674, 3362, pydata__xarray-3364 (6个)

### 问题总结

1. **API 限额**: 两批次在中途触发 5 小时限额，后续所有请求返回空
2. **超时**: pydata__xarray-4094 超过 900s 限制
3. **pip 安装失败**: 部分项目 (requests, matplotlib) 用 PYTHONPATH 回退
4. **补丁未保存**: 进程被杀时约 20 个已生成的补丁未写入 JSON 文件

### 已生成但未保存的补丁

补丁存在于 `swebench_runs/*/` 目录的 git diff 中，可手动提取:

```bash
# 提取未保存的补丁
for dir in django__django-15819 django__django-15851 django__django-15902 django__django-16041 django__django-16046 django__django-16139 django__django-16229 django__django-16255 django__django-16379 django__django-16400 django__django-16408 django__django-16527 django__django-16595 django__django-16816 psf__requests-1963 psf__requests-2148 psf__requests-2317 psf__requests-2674 psf__requests-3362 pydata__xarray-3364; do
  echo "=== $dir ==="
  cd swebench_runs/$dir && git diff HEAD 2>/dev/null | head -50
  cd ../..
done
```

## 补丁质量分析

### 统计
- 所有 46 个补丁都是有效的 git diff 格式
- 补丁大小范围: 503 - 1707 字符
- 覆盖项目: astropy, django, 等

### 示例补丁预览
```diff
# astropy__astropy-12907
-        cright[-right.shape[0]:, -right.shape[1]:] = 1
+        cright[-right.shape[0]:, -right.shape[1]:] = right

# astropy__astropy-14182
-    def __init__(self):
-        super().__init__(delimiter_pad=None, bookend=False)
+    def __init__(self, header_rows=None):
+        super().__init__(delimiter_pad=None, bookend=False, header_rows=header_rows)
```

## 中断原因

API 5小时限额触发，批次3和4在执行过程中无法生成补丁。

## 恢复命令

等待 API 限额重置后执行：

```bash
cd /Users/lifcc/Desktop/code/AI/agent/sage/swebench_eval

# 批次 3 (从中断处继续): 100-149
# 已尝试 21 个，从 offset 121 继续
nohup uv run run_agent.py --offset 121 --limit 29 --output predictions_122_150.json --timeout 900 --max-retries 1 </dev/null > swebench_122_150.log 2>&1 &

# 批次 4 (从中断处继续): 150-199
# 已尝试 19 个，从 offset 169 继续
nohup uv run run_agent.py --offset 169 --limit 31 --output predictions_170_200.json --timeout 900 --max-retries 1 </dev/null > swebench_170_200.log 2>&1 &

# 批次 5: 200-249
nohup uv run run_agent.py --offset 200 --limit 50 --output predictions_201_250.json --timeout 900 --max-retries 1 </dev/null > swebench_201_250.log 2>&1 &

# 批次 6: 250-299
nohup uv run run_agent.py --offset 250 --limit 50 --output predictions_251_300.json --timeout 900 --max-retries 1 </dev/null > swebench_251_300.log 2>&1 &
```

## 监控命令

```bash
# 查看进程
ps aux | grep run_agent

# 查看日志
tail -f swebench_122_150.log

# 查看完成数
ls swebench_runs/ | wc -l

# 查看补丁数
cat predictions_122_150.json | python3 -c "import json,sys; print(len(json.load(sys.stdin)))"
```

## 合并所有预测

完成后执行：

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

## 已完成实例列表 (46个)

| # | Instance ID |
|---|-------------|
| 1 | astropy__astropy-12907 |
| 2 | astropy__astropy-14182 |
| 3 | astropy__astropy-14365 |
| 4 | astropy__astropy-14995 |
| 5 | astropy__astropy-6938 |
| 6 | astropy__astropy-7746 |
| 7 | django__django-10914 |
| 8 | django__django-10924 |
| 9 | django__django-11001 |
| 10 | django__django-11019 |
| 11 | django__django-13230 |
| 12 | django__django-13265 |
| 13 | django__django-13315 |
| 14 | django__django-13401 |
| 15 | django__django-13447 |
| 16 | django__django-13448 |
| 17 | django__django-13551 |
| 18 | django__django-13590 |
| 19 | django__django-13658 |
| 20 | django__django-13660 |
| 21 | django__django-13710 |
| 22 | django__django-13757 |
| 23 | django__django-13768 |
| 24 | django__django-13925 |
| 25 | django__django-13933 |
| 26 | django__django-13964 |
| 27 | django__django-14016 |
| 28 | django__django-14017 |
| 29 | django__django-14155 |
| 30 | django__django-14238 |
| 31 | django__django-14382 |
| 32 | django__django-14411 |
| 33 | django__django-14534 |
| 34 | django__django-14580 |
| 35 | django__django-14608 |
| 36 | django__django-14667 |
| 37 | django__django-14672 |
| 38 | django__django-14730 |
| 39 | django__django-14752 |
| 40 | django__django-14787 |
| 41 | django__django-14855 |
| 42 | django__django-14915 |
| 43 | django__django-15498 |
| 44 | django__django-16379 |
| 45 | sympy__sympy-17022 |
| 46 | sympy__sympy-18087 |
