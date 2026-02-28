#!/bin/bash
# 查找代码库中的优化机会

set -e

CORE_SRC="crates/sage-core/src"
TOOLS_SRC="crates/sage-tools/src"
CLI_SRC="crates/sage-cli/src"

echo "🔍 Sage Agent 优化机会分析"
echo "================================"
echo ""

# 1. 克隆分析
echo "📊 1. 克隆使用分析"
echo "-------------------"
CLONE_COUNT=$(rg "\.clone\(\)" --type rust "$CORE_SRC" 2>/dev/null | wc -l | tr -d ' ')
echo "sage-core 中的 .clone() 调用: $CLONE_COUNT"

CLONE_IN_LOOPS=$(rg "\.clone\(\)" --type rust "$CORE_SRC" -B 3 2>/dev/null | rg "for |while |loop" | wc -l | tr -d ' ')
echo "循环中的克隆 (高优先级): $CLONE_IN_LOOPS"

echo ""
echo "热点文件 (克隆最多的前 10 个):"
rg "\.clone\(\)" --type rust "$CORE_SRC" -c 2>/dev/null | sort -t: -k2 -rn | head -10

echo ""
echo ""

# 2. Unwrap/Expect 分析
echo "⚠️  2. Unwrap/Expect 使用分析"
echo "----------------------------"
UNWRAP_COUNT=$(rg "\.unwrap\(\)|\.expect\(" --type rust "$CORE_SRC" --glob '!**/tests/**' 2>/dev/null | wc -l | tr -d ' ')
echo "非测试代码中的 unwrap/expect: $UNWRAP_COUNT"

echo ""
echo "高风险文件 (unwrap 最多的前 10 个):"
rg "\.unwrap\(\)|\.expect\(" --type rust "$CORE_SRC" --glob '!**/tests/**' -c 2>/dev/null | sort -t: -k2 -rn | head -10

echo ""
echo ""

# 3. 锁使用分析
echo "🔒 3. 锁使用分析"
echo "----------------"
LOCK_COUNT=$(rg "\.lock\(\)|\.read\(\)|\.write\(\)" --type rust "$CORE_SRC" 2>/dev/null | wc -l | tr -d ' ')
echo "锁操作总数: $LOCK_COUNT"

echo ""
echo "可能存在嵌套锁的函数:"
rg "\.lock\(\)|\.read\(\)|\.write\(\)" --type rust "$CORE_SRC" -A 20 2>/dev/null | \
  rg "fn \w+" -A 20 | \
  rg "\.lock\(\)|\.read\(\)|\.write\(\)" | \
  head -20

echo ""
echo ""

# 4. 大文件分析
echo "📄 4. 大文件分析 (>450 行)"
echo "--------------------------"
find crates -name "*.rs" -type f ! -path "*/tests/*" ! -path "*/target/*" -exec wc -l {} + 2>/dev/null | \
  sort -rn | \
  awk '$1 > 450 {print $1 " lines - " $2}' | \
  head -15

echo ""
echo ""

# 5. 测试覆盖率分析
echo "🧪 5. 测试覆盖率分析"
echo "--------------------"
TOTAL_FILES=$(find crates -name "*.rs" -type f ! -path "*/tests/*" ! -path "*/target/*" 2>/dev/null | wc -l | tr -d ' ')
TEST_FILES=$(find crates -name "*test*.rs" -o -name "tests.rs" 2>/dev/null | wc -l | tr -d ' ')
FILES_WITH_TESTS=$(rg "#\[cfg\(test\)\]" --type rust crates -l 2>/dev/null | wc -l | tr -d ' ')

echo "总源文件数: $TOTAL_FILES"
echo "测试文件数: $TEST_FILES"
echo "包含测试的文件数: $FILES_WITH_TESTS"
echo "测试覆盖率: $(echo "scale=1; $FILES_WITH_TESTS * 100 / $TOTAL_FILES" | bc)%"

echo ""
echo "缺少测试的大文件 (>200 行且无测试):"
for file in $(find crates -name "*.rs" -type f ! -path "*/tests/*" ! -path "*/target/*" 2>/dev/null); do
  lines=$(wc -l < "$file" 2>/dev/null | tr -d ' ')
  if [ "$lines" -gt 200 ]; then
    if ! rg -q "#\[cfg\(test\)\]" "$file" 2>/dev/null; then
      echo "  $lines lines - $file"
    fi
  fi
done | sort -rn | head -10

echo ""
echo ""

# 6. 字符串分配分析
echo "💾 6. 字符串分配分析"
echo "--------------------"
TO_STRING=$(rg "\.to_string\(\)" --type rust "$CORE_SRC" 2>/dev/null | wc -l | tr -d ' ')
TO_OWNED=$(rg "\.to_owned\(\)" --type rust "$CORE_SRC" 2>/dev/null | wc -l | tr -d ' ')
STRING_FROM=$(rg "String::from" --type rust "$CORE_SRC" 2>/dev/null | wc -l | tr -d ' ')
FORMAT=$(rg "format!\(" --type rust "$CORE_SRC" 2>/dev/null | wc -l | tr -d ' ')

echo ".to_string() 调用: $TO_STRING"
echo ".to_owned() 调用: $TO_OWNED"
echo "String::from 调用: $STRING_FROM"
echo "format!() 调用: $FORMAT"
echo "总字符串分配: $(($TO_STRING + $TO_OWNED + $STRING_FROM + $FORMAT))"

echo ""
echo ""

# 7. Unsafe 代码分析
echo "⚡ 7. Unsafe 代码分析"
echo "--------------------"
UNSAFE_COUNT=$(rg "unsafe" --type rust "$CORE_SRC" 2>/dev/null | wc -l | tr -d ' ')
echo "unsafe 关键字使用: $UNSAFE_COUNT"

if [ "$UNSAFE_COUNT" -gt 0 ]; then
  echo ""
  echo "Unsafe 代码位置:"
  rg "unsafe" --type rust "$CORE_SRC" -n 2>/dev/null | head -10
fi

echo ""
echo ""

# 8. TODO/FIXME 分析
echo "📝 8. TODO/FIXME 分析"
echo "---------------------"
TODO_COUNT=$(rg "TODO|FIXME|HACK|XXX" --type rust crates 2>/dev/null | wc -l | tr -d ' ')
echo "技术债务标记: $TODO_COUNT"

echo ""
echo "按类型分类:"
echo "  TODO: $(rg "TODO" --type rust crates 2>/dev/null | wc -l | tr -d ' ')"
echo "  FIXME: $(rg "FIXME" --type rust crates 2>/dev/null | wc -l | tr -d ' ')"
echo "  HACK: $(rg "HACK" --type rust crates 2>/dev/null | wc -l | tr -d ' ')"
echo "  XXX: $(rg "XXX" --type rust crates 2>/dev/null | wc -l | tr -d ' ')"

echo ""
echo ""

# 9. 依赖分析
echo "📦 9. 依赖分析"
echo "--------------"
echo "Workspace 依赖数:"
grep -c "^[a-z]" Cargo.toml 2>/dev/null || echo "0"

echo ""
echo "最大的依赖 (编译时间):"
cargo tree --depth 1 2>/dev/null | head -10 || echo "运行 'cargo tree' 查看详情"

echo ""
echo ""

# 10. 总结和建议
echo "📋 10. 优化建议总结"
echo "-------------------"

PRIORITY_SCORE=0

if [ "$UNWRAP_COUNT" -gt 100 ]; then
  echo "🔴 高优先级: 减少 unwrap/expect 使用 ($UNWRAP_COUNT 个)"
  PRIORITY_SCORE=$((PRIORITY_SCORE + 3))
fi

if [ "$CLONE_COUNT" -gt 500 ]; then
  echo "🟡 中优先级: 优化克隆使用 ($CLONE_COUNT 个)"
  PRIORITY_SCORE=$((PRIORITY_SCORE + 2))
fi

if [ "$LOCK_COUNT" -gt 400 ]; then
  echo "🟡 中优先级: 审查锁使用模式 ($LOCK_COUNT 个)"
  PRIORITY_SCORE=$((PRIORITY_SCORE + 2))
fi

LARGE_FILES=$(find crates -name "*.rs" -type f ! -path "*/tests/*" ! -path "*/target/*" -exec wc -l {} + 2>/dev/null | awk '$1 > 450' | wc -l | tr -d ' ')
if [ "$LARGE_FILES" -gt 5 ]; then
  echo "🟡 中优先级: 拆分大文件 ($LARGE_FILES 个 >450 行)"
  PRIORITY_SCORE=$((PRIORITY_SCORE + 2))
fi

TEST_COVERAGE=$(echo "scale=0; $FILES_WITH_TESTS * 100 / $TOTAL_FILES" | bc)
if [ "$TEST_COVERAGE" -lt 50 ]; then
  echo "🟡 中优先级: 提高测试覆盖率 (当前 ${TEST_COVERAGE}%)"
  PRIORITY_SCORE=$((PRIORITY_SCORE + 2))
fi

if [ "$TODO_COUNT" -gt 50 ]; then
  echo "🟢 低优先级: 处理技术债务标记 ($TODO_COUNT 个)"
  PRIORITY_SCORE=$((PRIORITY_SCORE + 1))
fi

echo ""
echo "总体优化优先级评分: $PRIORITY_SCORE/15"

if [ "$PRIORITY_SCORE" -gt 10 ]; then
  echo "建议: 立即开始优化工作"
elif [ "$PRIORITY_SCORE" -gt 5 ]; then
  echo "建议: 在接下来的 1-2 个月内进行优化"
else
  echo "建议: 代码质量良好，可以进行渐进式优化"
fi

echo ""
echo "================================"
echo "✅ 分析完成"
echo ""
echo "详细报告: OPTIMIZATION_RECOMMENDATIONS.md"
echo "快速修复: docs/optimization/quick-wins.md"
