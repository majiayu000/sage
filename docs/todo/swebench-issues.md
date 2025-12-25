# SWE-bench 执行问题分析与修复计划

## 执行概况

- **测试日期**: 2025-12-25
- **数据集**: princeton-nlp/SWE-bench_Verified
- **测试实例数**: 10
- **补丁生成数**: 10 (100%)
- **总耗时**: 1:36:05

## 问题分析

### 问题 1: Bash 命令链被禁止 (高优先级)

**错误信息**:
```
Permission denied: Command chaining operator not allowed: '&&'
Permission denied: Command chaining operator not allowed: ';'
```

**影响**: 53+ 次命令执行失败

**原因**: Bash 工具的安全限制禁止使用 `&&` 和 `;` 连接多个命令

**典型场景**:
```bash
# Agent 尝试执行:
cd /path/to/repo && python -c "import astropy; ..."

# 被拒绝，需要改为:
python -c "import sys; sys.path.insert(0, '/path/to/repo'); import astropy; ..."
```

**修复方案**:
1. **方案 A**: 在 SWE-bench 模式下放宽 bash 安全限制
2. **方案 B**: 修改 system prompt 指导 agent 使用替代方法
3. **方案 C**: 提供专门的 `run_in_directory` 工具

---

### 问题 2: Python 环境/依赖问题 (中优先级)

**错误信息**:
```
astropy/version.py:11: UserWarning: could not determine astropy package version
ImportError: No module named 'xxx'
```

**原因**:
- 仓库直接克隆后未安装依赖
- Python 路径未正确配置
- 缺少开发模式安装 (`pip install -e .`)

**修复方案**:
1. 在 `run_agent.py` 中添加仓库初始化步骤
2. 创建独立的 Python 虚拟环境
3. 自动执行 `pip install -e .` 或等效设置

---

### 问题 3: 测试验证不完整 (中优先级)

**现状**: Agent 生成了补丁但无法验证修复是否正确

**原因**:
- 无法运行 `pytest` 测试（因命令链限制）
- 依赖未安装导致测试无法执行

**修复方案**:
1. 修复命令链问题后自然解决
2. 添加补丁应用后的自动测试验证

---

### 问题 4: Agent 反复尝试失败命令 (低优先级)

**观察**: Agent 在遇到 `&&` 限制后会尝试多种变体，浪费 token

**修复方案**:
1. 在 system prompt 中明确说明命令限制
2. 提供错误恢复指导

---

## 执行详情统计

| Instance ID | 总工具调用 | 失败次数 | 主要错误类型 |
|-------------|-----------|---------|-------------|
| astropy-12907 | 50+ | 20+ | && 限制, 导入错误 |
| astropy-13033 | 24 | ~5 | && 限制 |
| astropy-13236 | 17 | ~3 | && 限制 |
| astropy-13398 | 45 | ~10 | && 限制, 导入错误 |
| astropy-13453 | 45 | ~8 | && 限制 |
| astropy-13579 | 26 | ~5 | && 限制 |
| astropy-13977 | 17 | ~3 | && 限制 |
| astropy-14096 | 38 | ~8 | && 限制 |
| astropy-14182 | 36 | ~6 | && 限制 |
| astropy-14309 | 11 | ~2 | && 限制 |

---

## 修复计划

### Phase 1: 紧急修复 (1-2天)

- [ ] **Task 1.1**: 为 SWE-bench 模式添加 bash 命令链支持
  - 文件: `crates/sage-tools/src/tools/process/bash.rs`
  - 添加 `--allow-chaining` 或 `swebench_mode` 标志

- [ ] **Task 1.2**: 更新 SWE-bench system prompt
  - 文件: `swebench_eval/run_agent.py`
  - 说明命令执行限制和替代方法

### Phase 2: 环境改进 (3-5天)

- [ ] **Task 2.1**: 添加仓库初始化脚本
  - 克隆后自动设置 Python 路径
  - 可选: 安装开发依赖

- [ ] **Task 2.2**: 提供专用测试运行工具
  - 新工具: `RunTests` 或增强 bash 工具

### Phase 3: 验证与优化 (1周)

- [ ] **Task 3.1**: 运行官方 SWE-bench 评估
  - 使用 `run_evaluation.py` 验证补丁正确性

- [ ] **Task 3.2**: 对比分析
  - 计算实际解决率
  - 与其他 agent 对比

---

## 临时解决方案

在修复完成前，可使用以下方式运行:

```bash
# 1. 禁用 sandbox 模式 (不推荐用于生产)
sage unified "..." --disable-sandbox

# 2. 使用 Docker 隔离环境
cd swebench_eval && ./docker_eval.sh
```

---

## 参考资料

- [SWE-bench 官方文档](https://www.swebench.com/)
- [Sage Bash 工具源码](../crates/sage-tools/src/tools/process/bash.rs)
- Session 日志: `~/.sage/projects/Users-Zhuanz-Desktop-code-Open-AI-code-agent-sage-swebench_eval-swebench_runs-*/`
