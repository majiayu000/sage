#!/bin/bash
# 查找并报告可以优化的克隆使用

set -e

CORE_SRC="crates/sage-core/src"
OUTPUT_FILE="optimization-reports/clone-analysis-$(date +%Y%m%d-%H%M%S).md"

mkdir -p optimization-reports

echo "🔍 克隆使用分析报告" > "$OUTPUT_FILE"
echo "===================" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"
echo "生成时间: $(date)" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

# 1. 总体统计
echo "## 总体统计" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

TOTAL_CLONES=$(rg "\.clone\(\)" --type rust "$CORE_SRC" 2>/dev/null | wc -l | tr -d ' ')
echo "- 总克隆数: $TOTAL_CLONES" >> "$OUTPUT_FILE"

CLONES_IN_LOOPS=$(rg "\.clone\(\)" --type rust "$CORE_SRC" -B 3 2>/dev/null | rg -c "for |while |loop" || echo "0")
echo "- 循环中的克隆: $CLONES_IN_LOOPS (高优先级)" >> "$OUTPUT_FILE"

CLONES_IN_ASYNC=$(rg "\.clone\(\)" --type rust "$CORE_SRC" -B 5 2>/dev/null | rg -c "async fn" || echo "0")
echo "- async 函数中的克隆: $CLONES_IN_ASYNC" >> "$OUTPUT_FILE"

echo "" >> "$OUTPUT_FILE"

# 2. 按文件分类
echo "## 克隆热点文件 (Top 20)" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"
echo "| 文件 | 克隆数 |" >> "$OUTPUT_FILE"
echo "|------|--------|" >> "$OUTPUT_FILE"

rg "\.clone\(\)" --type rust "$CORE_SRC" -c 2>/dev/null | \
  sort -t: -k2 -rn | \
  head -20 | \
  while IFS=: read -r file count; do
    short_file=$(echo "$file" | sed "s|$CORE_SRC/||")
    echo "| $short_file | $count |" >> "$OUTPUT_FILE"
  done

echo "" >> "$OUTPUT_FILE"

# 3. 循环中的克隆（高优先级）
echo "## 🔴 高优先级: 循环中的克隆" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"
echo "这些克隆在循环中被调用，可能导致显著的性能开销。" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

rg "\.clone\(\)" --type rust "$CORE_SRC" -B 5 -A 2 2>/dev/null | \
  rg -B 5 -A 2 "for |while |loop" | \
  head -50 >> "$OUTPUT_FILE"

echo "" >> "$OUTPUT_FILE"

# 4. 字符串克隆
echo "## 字符串克隆分析" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

TO_STRING=$(rg "\.to_string\(\)" --type rust "$CORE_SRC" 2>/dev/null | wc -l | tr -d ' ')
TO_OWNED=$(rg "\.to_owned\(\)" --type rust "$CORE_SRC" 2>/dev/null | wc -l | tr -d ' ')
STRING_FROM=$(rg "String::from" --type rust "$CORE_SRC" 2>/dev/null | wc -l | tr -d ' ')

echo "- .to_string(): $TO_STRING" >> "$OUTPUT_FILE"
echo "- .to_owned(): $TO_OWNED" >> "$OUTPUT_FILE"
echo "- String::from(): $STRING_FROM" >> "$OUTPUT_FILE"
echo "- 总计: $(($TO_STRING + $TO_OWNED + $STRING_FROM))" >> "$OUTPUT_FILE"

echo "" >> "$OUTPUT_FILE"

# 5. 可能的优化建议
echo "## 优化建议" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

echo "### 模式 1: 函数参数使用引用" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"
echo '```rust' >> "$OUTPUT_FILE"
echo '// 之前' >> "$OUTPUT_FILE"
echo 'fn process(data: String) { }' >> "$OUTPUT_FILE"
echo 'process(data.clone()); // 不必要的克隆' >> "$OUTPUT_FILE"
echo '' >> "$OUTPUT_FILE"
echo '// 之后' >> "$OUTPUT_FILE"
echo 'fn process(data: &str) { }' >> "$OUTPUT_FILE"
echo 'process(&data); // 无克隆' >> "$OUTPUT_FILE"
echo '```' >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

echo "### 模式 2: 使用 Arc 共享所有权" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"
echo '```rust' >> "$OUTPUT_FILE"
echo '// 之前' >> "$OUTPUT_FILE"
echo 'let config = config.clone(); // 深拷贝' >> "$OUTPUT_FILE"
echo '' >> "$OUTPUT_FILE"
echo '// 之后' >> "$OUTPUT_FILE"
echo 'let config = Arc::clone(&config); // 只增加引用计数' >> "$OUTPUT_FILE"
echo '```' >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

echo "### 模式 3: 使用 Cow 处理条件所有权" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"
echo '```rust' >> "$OUTPUT_FILE"
echo 'use std::borrow::Cow;' >> "$OUTPUT_FILE"
echo '' >> "$OUTPUT_FILE"
echo "fn format_msg<'a>(prefix: &str, msg: &'a str) -> Cow<'a, str> {" >> "$OUTPUT_FILE"
echo '    if prefix.is_empty() {' >> "$OUTPUT_FILE"
echo '        Cow::Borrowed(msg) // 无克隆' >> "$OUTPUT_FILE"
echo '    } else {' >> "$OUTPUT_FILE"
echo '        Cow::Owned(format!("{}: {}", prefix, msg))' >> "$OUTPUT_FILE"
echo '    }' >> "$OUTPUT_FILE"
echo '}' >> "$OUTPUT_FILE"
echo '```' >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

# 6. 具体文件的优化建议
echo "## 具体文件优化建议" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

# 找出克隆最多的前 5 个文件并给出建议
rg "\.clone\(\)" --type rust "$CORE_SRC" -c 2>/dev/null | \
  sort -t: -k2 -rn | \
  head -5 | \
  while IFS=: read -r file count; do
    short_file=$(echo "$file" | sed "s|$CORE_SRC/||")
    echo "### $short_file ($count 个克隆)" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
    echo "建议审查此文件中的克隆使用，特别关注:" >> "$OUTPUT_FILE"
    echo "- 是否可以使用引用代替" >> "$OUTPUT_FILE"
    echo "- 是否在循环中克隆" >> "$OUTPUT_FILE"
    echo "- 是否可以使用 Arc 共享" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
  done

echo "" >> "$OUTPUT_FILE"
echo "---" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"
echo "报告生成完成。查看详细信息请参考上述分析。" >> "$OUTPUT_FILE"

# 输出到终端
cat "$OUTPUT_FILE"

echo ""
echo "✅ 报告已保存到: $OUTPUT_FILE"
