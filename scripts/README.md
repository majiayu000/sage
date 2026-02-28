# 优化工具使用指南

本目录包含用于分析和优化 Sage Agent 代码库的自动化脚本。

## 📋 可用脚本

### 1. 综合分析脚本

#### `find-optimization-opportunities.sh`
查找代码库中的所有优化机会。

```bash
./scripts/find-optimization-opportunities.sh
```

**输出内容:**
- 克隆使用统计
- Unwrap/expect 分析
- 锁使用模式
- 大文件列表
- 测试覆盖率
- 技术债务统计
- 优化优先级评分

**运行时间:** ~30 秒

---

#### `generate-quality-report.sh`
生成完整的代码质量报告。

```bash
./scripts/generate-quality-report.sh
```

**输出:** `optimization-reports/quality-report-TIMESTAMP.md`

**包含内容:**
- 代码库统计
- 质量指标
- 性能分析
- 测试覆盖率
- 优化建议
- 总体评分

**运行时间:** ~1 分钟

---

### 2. 专项分析脚本

#### `analyze-clones.sh`
深入分析克隆使用情况。

```bash
./scripts/analyze-clones.sh
```

**输出:** `optimization-reports/clone-analysis-TIMESTAMP.md`

**分析内容:**
- 克隆热点文件
- 循环中的克隆（高优先级）
- 字符串分配统计
- 具体优化建议

**运行时间:** ~20 秒

---

#### `analyze-unwraps.sh`
分析 unwrap/expect 使用情况。

```bash
./scripts/analyze-unwraps.sh
```

**输出:** `optimization-reports/unwrap-analysis-TIMESTAMP.md`

**分析内容:**
- 高风险文件列表
- 按模块分类
- 关键路径中的 unwrap
- 修复建议和优先级

**运行时间:** ~20 秒

---

### 3. 修复脚本

#### `fix-duplicate-types.sh`
半自动修复重复类型定义。

```bash
./scripts/fix-duplicate-types.sh
```

**功能:**
- 创建备份分支
- 交互式修复重复类型
- 自动添加允许列表
- 运行测试验证
- 提交更改

**运行时间:** ~10-30 分钟（取决于手动操作）

---

## 🚀 快速开始

### 第一次使用

1. **确保脚本可执行:**
```bash
chmod +x scripts/*.sh
```

2. **运行综合分析:**
```bash
./scripts/find-optimization-opportunities.sh
```

3. **生成详细报告:**
```bash
./scripts/generate-quality-report.sh
```

4. **查看报告:**
```bash
ls -lh optimization-reports/
cat optimization-reports/quality-report-*.md
```

---

## 📊 工作流程建议

### 每周例行检查

```bash
#!/bin/bash
# weekly-check.sh

echo "🔍 运行每周代码质量检查..."

# 1. 生成质量报告
./scripts/generate-quality-report.sh

# 2. 运行 VibeGuard
make guard

# 3. 运行 Clippy
cargo clippy --all-targets -- -D warnings

# 4. 运行测试
cargo test

echo "✅ 每周检查完成"
```

### 优化冲刺（2 周）

**第 1 周:**
```bash
# Day 1: 分析
./scripts/find-optimization-opportunities.sh
./scripts/analyze-unwraps.sh
./scripts/analyze-clones.sh

# Day 2-3: 修复重复类型
./scripts/fix-duplicate-types.sh

# Day 4-5: 修复高优先级 unwrap
# 手动修复 config/ 和 session/ 中的 unwrap
```

**第 2 周:**
```bash
# Day 1-2: 优化克隆
# 手动优化热点文件中的克隆

# Day 3-4: 拆分大文件
# 手动拆分 >450 行的文件

# Day 5: 验证和总结
./scripts/generate-quality-report.sh
cargo test
make guard
```

---

## 🎯 优化目标

### 短期目标（1 个月）

- [ ] 重复类型: 0 个
- [ ] Unwrap/expect: <100 个
- [ ] 大文件 (>450 行): <5 个
- [ ] 测试覆盖率: >40%

### 中期目标（3 个月）

- [ ] Unwrap/expect: <50 个
- [ ] 克隆调用: <500 个
- [ ] 测试覆盖率: >60%
- [ ] 嵌套锁: <15 处

### 长期目标（6 个月）

- [ ] 代码质量评分: >85/100
- [ ] 测试覆盖率: >70%
- [ ] 所有 VibeGuard 检查通过
- [ ] 性能基准测试建立

---

## 📈 跟踪进度

### 创建进度跟踪表

```bash
# 在项目根目录创建
cat > OPTIMIZATION_PROGRESS.md << 'EOF'
# 优化进度跟踪

## 当前状态

| 指标 | 基线 | 当前 | 目标 | 进度 |
|------|------|------|------|------|
| 重复类型 | 9 | 9 | 0 | 0% |
| Unwrap/expect | 1105 | 1105 | 100 | 0% |
| 克隆调用 | 740 | 740 | 500 | 0% |
| 测试覆盖率 | 33% | 33% | 60% | 0% |
| 质量评分 | ? | ? | 85 | ? |

## 更新日志

### 2026-02-23
- 建立基线指标
- 创建优化脚本
- 生成初始报告

EOF
```

### 每周更新进度

```bash
# 运行分析并更新进度表
./scripts/find-optimization-opportunities.sh > /tmp/metrics.txt

# 手动更新 OPTIMIZATION_PROGRESS.md
# 或创建自动化脚本
```

---

## 🛠️ 自定义脚本

### 创建自定义分析脚本

```bash
#!/bin/bash
# scripts/custom-analysis.sh

# 示例: 查找特定模式
echo "查找 TODO 标记..."
rg "TODO" --type rust crates/ -n

echo "查找长函数 (>100 行)..."
# 添加你的分析逻辑
```

### 集成到 CI/CD

```yaml
# .github/workflows/code-quality.yml
name: Code Quality

on: [push, pull_request]

jobs:
  quality-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      
      - name: Run optimization analysis
        run: ./scripts/find-optimization-opportunities.sh
      
      - name: Run VibeGuard
        run: make guard-strict
      
      - name: Generate quality report
        run: ./scripts/generate-quality-report.sh
      
      - name: Upload report
        uses: actions/upload-artifact@v2
        with:
          name: quality-report
          path: optimization-reports/
```

---

## 📚 相关文档

- [完整优化建议](../OPTIMIZATION_RECOMMENDATIONS.md)
- [快速修复指南](../docs/optimization/quick-wins.md)
- [项目文档](../CLAUDE.md)

---

## 💡 提示和技巧

### 1. 批量处理

```bash
# 批量运行所有分析脚本
for script in scripts/analyze-*.sh; do
    echo "运行 $script..."
    bash "$script"
done
```

### 2. 比较报告

```bash
# 比较两次报告的差异
diff optimization-reports/quality-report-20260223-*.md \
     optimization-reports/quality-report-20260301-*.md
```

### 3. 导出为 HTML

```bash
# 使用 pandoc 转换为 HTML
pandoc optimization-reports/quality-report-*.md \
       -o quality-report.html \
       --standalone \
       --css style.css
```

### 4. 定时运行

```bash
# 添加到 crontab (每周一早上 9 点)
0 9 * * 1 cd /path/to/sage && ./scripts/generate-quality-report.sh
```

---

## 🤝 贡献

如果你创建了有用的分析脚本，欢迎贡献！

1. 将脚本放在 `scripts/` 目录
2. 添加到本文档
3. 确保脚本有适当的注释
4. 提交 Pull Request

---

## ❓ 常见问题

**Q: 脚本运行失败怎么办？**

A: 确保：
- 在项目根目录运行
- 已安装 `ripgrep` (`rg` 命令)
- 脚本有执行权限 (`chmod +x`)

**Q: 报告太长怎么办？**

A: 使用 `head` 或 `tail` 查看部分内容：
```bash
./scripts/find-optimization-opportunities.sh | head -50
```

**Q: 如何只分析特定模块？**

A: 修改脚本中的 `CORE_SRC` 变量：
```bash
CORE_SRC="crates/sage-core/src/agent"
```

---

*最后更新: 2026-02-23*
