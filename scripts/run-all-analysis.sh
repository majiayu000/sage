#!/bin/bash
# 一键运行所有优化分析

set -e

echo "🚀 Sage Agent 完整优化分析"
echo "============================"
echo ""

# 创建报告目录
mkdir -p optimization-reports

echo "📊 步骤 1/4: 运行综合分析..."
./scripts/find-optimization-opportunities.sh > optimization-reports/comprehensive-analysis.txt
echo "✅ 完成"
echo ""

echo "📊 步骤 2/4: 生成质量报告..."
./scripts/generate-quality-report.sh > /dev/null
echo "✅ 完成"
echo ""

echo "📊 步骤 3/4: 分析克隆使用..."
./scripts/analyze-clones.sh > /dev/null
echo "✅ 完成"
echo ""

echo "📊 步骤 4/4: 分析 unwrap 使用..."
./scripts/analyze-unwraps.sh > /dev/null
echo "✅ 完成"
echo ""

echo "================================"
echo "✅ 所有分析完成！"
echo ""
echo "📁 生成的报告:"
ls -lh optimization-reports/ | tail -n +2
echo ""
echo "📖 查看报告:"
echo "  - 综合分析: cat optimization-reports/comprehensive-analysis.txt"
echo "  - 质量报告: cat optimization-reports/quality-report-*.md"
echo "  - 克隆分析: cat optimization-reports/clone-analysis-*.md"
echo "  - Unwrap 分析: cat optimization-reports/unwrap-analysis-*.md"
echo ""
echo "📚 相关文档:"
echo "  - OPTIMIZATION_RECOMMENDATIONS.md"
echo "  - OPTIMIZATION_SUMMARY.md"
echo "  - OPTIMIZATION_PROGRESS.md"
echo "  - docs/optimization/quick-wins.md"
