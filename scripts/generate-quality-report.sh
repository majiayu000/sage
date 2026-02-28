#!/bin/bash
# 生成完整的代码质量报告

set -e

REPORT_DIR="optimization-reports"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
REPORT_FILE="$REPORT_DIR/quality-report-$TIMESTAMP.md"

mkdir -p "$REPORT_DIR"

echo "📊 生成代码质量报告..."
echo ""

# 开始报告
cat > "$REPORT_FILE" << 'EOF'
# Sage Agent 代码质量报告

**生成时间**: $(date)

---

## 执行摘要

本报告提供 Sage Agent 代码库的全面质量分析，包括：
- 代码度量
- 潜在问题
- 优化建议
- 行动计划

---

EOF

# 1. 基本统计
echo "## 1. 代码库统计" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

TOTAL_FILES=$(find crates -name "*.rs" -type f ! -path "*/target/*" 2>/dev/null | wc -l | tr -d ' ')
TOTAL_LINES=$(find crates -name "*.rs" -type f ! -path "*/target/*" -exec wc -l {} + 2>/dev/null | tail -1 | awk '{print $1}')
TEST_FILES=$(find crates -name "*test*.rs" -o -name "tests.rs" 2>/dev/null | wc -l | tr -d ' ')

echo "| 指标 | 数值 |" >> "$REPORT_FILE"
echo "|------|------|" >> "$REPORT_FILE"
echo "| 总文件数 | $TOTAL_FILES |" >> "$REPORT_FILE"
echo "| 总代码行数 | $TOTAL_LINES |" >> "$REPORT_FILE"
echo "| 测试文件数 | $TEST_FILES |" >> "$REPORT_FILE"
echo "| 平均文件大小 | $((TOTAL_LINES / TOTAL_FILES)) 行 |" >> "$REPORT_FILE"

echo "" >> "$REPORT_FILE"

# 2. 代码质量指标
echo "## 2. 代码质量指标" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

# Clippy
echo "### 2.1 Clippy 检查" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"
echo '```' >> "$REPORT_FILE"
if cargo clippy --all-targets --all-features -- -D warnings 2>&1 | head -20 >> "$REPORT_FILE"; then
    echo "✅ 无警告" >> "$REPORT_FILE"
else
    echo "⚠️ 存在警告，请查看详情" >> "$REPORT_FILE"
fi
echo '```' >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

# VibeGuard
echo "### 2.2 VibeGuard 检查" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

DUPLICATE_TYPES=$(make guard 2>&1 | grep "RS-05" | wc -l | tr -d ' ')
NESTED_LOCKS=$(make guard 2>&1 | grep "RS-01" | wc -l | tr -d ' ')
UNWRAPS=$(make guard 2>&1 | grep "RS-03" | wc -l | tr -d ' ')

echo "| 检查项 | 数量 | 状态 |" >> "$REPORT_FILE"
echo "|--------|------|------|" >> "$REPORT_FILE"
echo "| 重复类型 (RS-05) | $DUPLICATE_TYPES | $([ $DUPLICATE_TYPES -eq 0 ] && echo '✅' || echo '⚠️') |" >> "$REPORT_FILE"
echo "| 嵌套锁 (RS-01) | $NESTED_LOCKS | $([ $NESTED_LOCKS -eq 0 ] && echo '✅' || echo '⚠️') |" >> "$REPORT_FILE"
echo "| Unwrap 使用 (RS-03) | $UNWRAPS | $([ $UNWRAPS -eq 0 ] && echo '✅' || echo '⚠️') |" >> "$REPORT_FILE"

echo "" >> "$REPORT_FILE"

# 3. 性能指标
echo "## 3. 性能相关指标" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

CLONE_COUNT=$(rg "\.clone\(\)" --type rust crates/sage-core/src 2>/dev/null | wc -l | tr -d ' ')
LOCK_COUNT=$(rg "\.lock\(\)|\.read\(\)|\.write\(\)" --type rust crates/sage-core/src 2>/dev/null | wc -l | tr -d ' ')
UNWRAP_COUNT=$(rg "\.unwrap\(\)|\.expect\(" --type rust crates/sage-core/src --glob '!**/tests/**' 2>/dev/null | wc -l | tr -d ' ')

echo "| 指标 | 数量 | 评估 |" >> "$REPORT_FILE"
echo "|------|------|------|" >> "$REPORT_FILE"
echo "| .clone() 调用 | $CLONE_COUNT | $([ $CLONE_COUNT -lt 500 ] && echo '🟢 良好' || echo '🟡 需优化') |" >> "$REPORT_FILE"
echo "| 锁操作 | $LOCK_COUNT | $([ $LOCK_COUNT -lt 400 ] && echo '🟢 良好' || echo '🟡 需审查') |" >> "$REPORT_FILE"
echo "| unwrap/expect | $UNWRAP_COUNT | $([ $UNWRAP_COUNT -lt 100 ] && echo '🟢 良好' || echo '🔴 需修复') |" >> "$REPORT_FILE"

echo "" >> "$REPORT_FILE"

# 4. 测试覆盖率
echo "## 4. 测试覆盖率" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

FILES_WITH_TESTS=$(rg "#\[cfg\(test\)\]" --type rust crates -l 2>/dev/null | wc -l | tr -d ' ')
COVERAGE=$((FILES_WITH_TESTS * 100 / TOTAL_FILES))

echo "| 指标 | 数值 |" >> "$REPORT_FILE"
echo "|------|------|" >> "$REPORT_FILE"
echo "| 包含测试的文件 | $FILES_WITH_TESTS / $TOTAL_FILES |" >> "$REPORT_FILE"
echo "| 覆盖率 | $COVERAGE% |" >> "$REPORT_FILE"
echo "| 评估 | $([ $COVERAGE -gt 50 ] && echo '🟢 良好' || echo '🟡 需提高') |" >> "$REPORT_FILE"

echo "" >> "$REPORT_FILE"

# 5. 大文件分析
echo "## 5. 大文件分析 (>450 行)" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

echo "| 文件 | 行数 |" >> "$REPORT_FILE"
echo "|------|------|" >> "$REPORT_FILE"

find crates -name "*.rs" -type f ! -path "*/tests/*" ! -path "*/target/*" -exec wc -l {} + 2>/dev/null | \
  sort -rn | \
  awk '$1 > 450 {print "| " $2 " | " $1 " |"}' | \
  head -10 >> "$REPORT_FILE"

echo "" >> "$REPORT_FILE"

# 6. 技术债务
echo "## 6. 技术债务" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

TODO_COUNT=$(rg "TODO" --type rust crates 2>/dev/null | wc -l | tr -d ' ')
FIXME_COUNT=$(rg "FIXME" --type rust crates 2>/dev/null | wc -l | tr -d ' ')
HACK_COUNT=$(rg "HACK" --type rust crates 2>/dev/null | wc -l | tr -d ' ')

echo "| 类型 | 数量 |" >> "$REPORT_FILE"
echo "|------|------|" >> "$REPORT_FILE"
echo "| TODO | $TODO_COUNT |" >> "$REPORT_FILE"
echo "| FIXME | $FIXME_COUNT |" >> "$REPORT_FILE"
echo "| HACK | $HACK_COUNT |" >> "$REPORT_FILE"
echo "| **总计** | **$((TODO_COUNT + FIXME_COUNT + HACK_COUNT))** |" >> "$REPORT_FILE"

echo "" >> "$REPORT_FILE"

# 7. 优先级建议
echo "## 7. 优化优先级建议" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

echo "### 🔴 高优先级 (立即处理)" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

if [ $DUPLICATE_TYPES -gt 0 ]; then
    echo "- [ ] 修复 $DUPLICATE_TYPES 个重复类型定义" >> "$REPORT_FILE"
fi

if [ $UNWRAP_COUNT -gt 100 ]; then
    echo "- [ ] 减少 unwrap/expect 使用 (当前 $UNWRAP_COUNT 个)" >> "$REPORT_FILE"
fi

if [ $NESTED_LOCKS -gt 20 ]; then
    echo "- [ ] 优化嵌套锁模式 (发现 $NESTED_LOCKS 处)" >> "$REPORT_FILE"
fi

echo "" >> "$REPORT_FILE"

echo "### 🟡 中优先级 (1-2 个月)" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

if [ $CLONE_COUNT -gt 500 ]; then
    echo "- [ ] 减少不必要的克隆 (当前 $CLONE_COUNT 个)" >> "$REPORT_FILE"
fi

if [ $COVERAGE -lt 50 ]; then
    echo "- [ ] 提高测试覆盖率 (当前 $COVERAGE%)" >> "$REPORT_FILE"
fi

LARGE_FILES=$(find crates -name "*.rs" -type f ! -path "*/tests/*" ! -path "*/target/*" -exec wc -l {} + 2>/dev/null | awk '$1 > 450' | wc -l | tr -d ' ')
if [ $LARGE_FILES -gt 5 ]; then
    echo "- [ ] 拆分大文件 ($LARGE_FILES 个 >450 行)" >> "$REPORT_FILE"
fi

echo "" >> "$REPORT_FILE"

echo "### 🟢 低优先级 (3-6 个月)" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

if [ $((TODO_COUNT + FIXME_COUNT + HACK_COUNT)) -gt 50 ]; then
    echo "- [ ] 处理技术债务标记 ($((TODO_COUNT + FIXME_COUNT + HACK_COUNT)) 个)" >> "$REPORT_FILE"
fi

echo "- [ ] 添加性能基准测试" >> "$REPORT_FILE"
echo "- [ ] 模块重组优化" >> "$REPORT_FILE"

echo "" >> "$REPORT_FILE"

# 8. 总体评分
echo "## 8. 总体质量评分" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

SCORE=100

# 扣分项
[ $DUPLICATE_TYPES -gt 0 ] && SCORE=$((SCORE - 5))
[ $UNWRAP_COUNT -gt 100 ] && SCORE=$((SCORE - 15))
[ $NESTED_LOCKS -gt 20 ] && SCORE=$((SCORE - 10))
[ $CLONE_COUNT -gt 500 ] && SCORE=$((SCORE - 10))
[ $COVERAGE -lt 50 ] && SCORE=$((SCORE - 15))
[ $LARGE_FILES -gt 5 ] && SCORE=$((SCORE - 5))

echo "**总分: $SCORE / 100**" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

if [ $SCORE -ge 80 ]; then
    echo "🟢 **评级: 优秀** - 代码质量良好，继续保持" >> "$REPORT_FILE"
elif [ $SCORE -ge 60 ]; then
    echo "🟡 **评级: 良好** - 有一些需要改进的地方" >> "$REPORT_FILE"
else
    echo "🔴 **评级: 需要改进** - 建议立即开始优化工作" >> "$REPORT_FILE"
fi

echo "" >> "$REPORT_FILE"

# 9. 下一步行动
echo "## 9. 下一步行动" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

echo "1. **审查本报告** - 与团队讨论优化优先级" >> "$REPORT_FILE"
echo "2. **创建 Issues** - 为每个优化项创建跟踪 Issue" >> "$REPORT_FILE"
echo "3. **分配资源** - 确定负责人和时间表" >> "$REPORT_FILE"
echo "4. **开始实施** - 从高优先级项目开始" >> "$REPORT_FILE"
echo "5. **定期复查** - 每月生成新报告跟踪进度" >> "$REPORT_FILE"

echo "" >> "$REPORT_FILE"

# 10. 相关资源
echo "## 10. 相关资源" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

echo "- [完整优化建议](../OPTIMIZATION_RECOMMENDATIONS.md)" >> "$REPORT_FILE"
echo "- [快速修复指南](../docs/optimization/quick-wins.md)" >> "$REPORT_FILE"
echo "- [克隆分析报告](./clone-analysis-*.md)" >> "$REPORT_FILE"
echo "- [Unwrap 分析报告](./unwrap-analysis-*.md)" >> "$REPORT_FILE"

echo "" >> "$REPORT_FILE"
echo "---" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"
echo "*此报告由自动化脚本生成。运行 \`./scripts/generate-quality-report.sh\` 生成最新报告。*" >> "$REPORT_FILE"

# 输出到终端
cat "$REPORT_FILE"

echo ""
echo "✅ 报告已保存到: $REPORT_FILE"
echo ""
echo "📊 质量评分: $SCORE / 100"
