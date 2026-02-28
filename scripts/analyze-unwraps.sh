#!/bin/bash
# 查找并报告 unwrap/expect 使用情况

set -e

CORE_SRC="crates/sage-core/src"
OUTPUT_FILE="optimization-reports/unwrap-analysis-$(date +%Y%m%d-%H%M%S).md"

mkdir -p optimization-reports

echo "⚠️  Unwrap/Expect 使用分析报告" > "$OUTPUT_FILE"
echo "===============================" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"
echo "生成时间: $(date)" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

# 1. 总体统计
echo "## 总体统计" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

TOTAL_UNWRAP=$(rg "\.unwrap\(\)" --type rust "$CORE_SRC" --glob '!**/tests/**' 2>/dev/null | wc -l | tr -d ' ')
TOTAL_EXPECT=$(rg "\.expect\(" --type rust "$CORE_SRC" --glob '!**/tests/**' 2>/dev/null | wc -l | tr -d ' ')
TOTAL=$(($TOTAL_UNWRAP + $TOTAL_EXPECT))

echo "- .unwrap() 调用: $TOTAL_UNWRAP" >> "$OUTPUT_FILE"
echo "- .expect() 调用: $TOTAL_EXPECT" >> "$OUTPUT_FILE"
echo "- **总计: $TOTAL**" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

if [ "$TOTAL" -gt 100 ]; then
  echo "🔴 **警告**: unwrap/expect 使用过多，建议立即优化" >> "$OUTPUT_FILE"
elif [ "$TOTAL" -gt 50 ]; then
  echo "🟡 **注意**: unwrap/expect 使用较多，建议逐步优化" >> "$OUTPUT_FILE"
else
  echo "🟢 **良好**: unwrap/expect 使用在可接受范围内" >> "$OUTPUT_FILE"
fi

echo "" >> "$OUTPUT_FILE"

# 2. 按文件分类
echo "## 高风险文件 (Top 20)" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"
echo "| 文件 | unwrap/expect 数量 | 风险等级 |" >> "$OUTPUT_FILE"
echo "|------|-------------------|----------|" >> "$OUTPUT_FILE"

rg "\.unwrap\(\)|\.expect\(" --type rust "$CORE_SRC" --glob '!**/tests/**' -c 2>/dev/null | \
  sort -t: -k2 -rn | \
  head -20 | \
  while IFS=: read -r file count; do
    short_file=$(echo "$file" | sed "s|$CORE_SRC/||")
    if [ "$count" -gt 10 ]; then
      risk="🔴 高"
    elif [ "$count" -gt 5 ]; then
      risk="🟡 中"
    else
      risk="🟢 低"
    fi
    echo "| $short_file | $count | $risk |" >> "$OUTPUT_FILE"
  done

echo "" >> "$OUTPUT_FILE"

# 3. 按模块分类
echo "## 按模块分类" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

for module in agent llm tools config session mcp memory; do
  if [ -d "$CORE_SRC/$module" ]; then
    count=$(rg "\.unwrap\(\)|\.expect\(" --type rust "$CORE_SRC/$module" --glob '!**/tests/**' 2>/dev/null | wc -l | tr -d ' ')
    echo "- **$module/**: $count" >> "$OUTPUT_FILE"
  fi
done

echo "" >> "$OUTPUT_FILE"

# 4. 关键路径中的 unwrap
echo "## 🔴 关键路径中的 unwrap (高优先级)" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"
echo "以下是在关键执行路径中的 unwrap 调用，应优先修复：" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

# 配置加载
echo "### 配置加载" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"
if [ -d "$CORE_SRC/config" ]; then
  rg "\.unwrap\(\)|\.expect\(" --type rust "$CORE_SRC/config" --glob '!**/tests/**' -n 2>/dev/null | head -10 >> "$OUTPUT_FILE" || echo "无" >> "$OUTPUT_FILE"
fi
echo "" >> "$OUTPUT_FILE"

# 会话管理
echo "### 会话管理" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"
if [ -d "$CORE_SRC/session" ]; then
  rg "\.unwrap\(\)|\.expect\(" --type rust "$CORE_SRC/session" --glob '!**/tests/**' -n 2>/dev/null | head -10 >> "$OUTPUT_FILE" || echo "无" >> "$OUTPUT_FILE"
fi
echo "" >> "$OUTPUT_FILE"

# 工具执行
echo "### 工具执行" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"
if [ -d "$CORE_SRC/tools" ]; then
  rg "\.unwrap\(\)|\.expect\(" --type rust "$CORE_SRC/tools" --glob '!**/tests/**' -n 2>/dev/null | head -10 >> "$OUTPUT_FILE" || echo "无" >> "$OUTPUT_FILE"
fi
echo "" >> "$OUTPUT_FILE"

# 5. 修复建议
echo "## 修复建议" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

echo "### 模式 1: 返回 Result" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"
echo '```rust' >> "$OUTPUT_FILE"
echo '// 之前' >> "$OUTPUT_FILE"
echo 'fn load_config() -> Config {' >> "$OUTPUT_FILE"
echo '    let path = get_path().unwrap();' >> "$OUTPUT_FILE"
echo '    let content = fs::read_to_string(path).unwrap();' >> "$OUTPUT_FILE"
echo '    serde_json::from_str(&content).unwrap()' >> "$OUTPUT_FILE"
echo '}' >> "$OUTPUT_FILE"
echo '' >> "$OUTPUT_FILE"
echo '// 之后' >> "$OUTPUT_FILE"
echo 'fn load_config() -> Result<Config, ConfigError> {' >> "$OUTPUT_FILE"
echo '    let path = get_path().ok_or(ConfigError::PathNotFound)?;' >> "$OUTPUT_FILE"
echo '    let content = fs::read_to_string(path)' >> "$OUTPUT_FILE"
echo '        .map_err(ConfigError::IoError)?;' >> "$OUTPUT_FILE"
echo '    serde_json::from_str(&content)' >> "$OUTPUT_FILE"
echo '        .map_err(ConfigError::ParseError)' >> "$OUTPUT_FILE"
echo '}' >> "$OUTPUT_FILE"
echo '```' >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

echo "### 模式 2: 使用 unwrap_or / unwrap_or_else" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"
echo '```rust' >> "$OUTPUT_FILE"
echo '// 之前' >> "$OUTPUT_FILE"
echo 'let value = map.get("key").unwrap();' >> "$OUTPUT_FILE"
echo '' >> "$OUTPUT_FILE"
echo '// 之后' >> "$OUTPUT_FILE"
echo 'let value = map.get("key").unwrap_or(&default_value);' >> "$OUTPUT_FILE"
echo '// 或' >> "$OUTPUT_FILE"
echo 'let value = map.get("key").unwrap_or_else(|| compute_default());' >> "$OUTPUT_FILE"
echo '```' >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

echo "### 模式 3: 使用 ok_or / ok_or_else" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"
echo '```rust' >> "$OUTPUT_FILE"
echo '// 之前' >> "$OUTPUT_FILE"
echo 'let value = option.unwrap();' >> "$OUTPUT_FILE"
echo '' >> "$OUTPUT_FILE"
echo '// 之后' >> "$OUTPUT_FILE"
echo 'let value = option.ok_or(Error::ValueNotFound)?;' >> "$OUTPUT_FILE"
echo '// 或' >> "$OUTPUT_FILE"
echo 'let value = option.ok_or_else(|| Error::custom("not found"))?;' >> "$OUTPUT_FILE"
echo '```' >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

echo "### 模式 4: 使用 context (anyhow)" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"
echo '```rust' >> "$OUTPUT_FILE"
echo 'use anyhow::{Context, Result};' >> "$OUTPUT_FILE"
echo '' >> "$OUTPUT_FILE"
echo '// 之前' >> "$OUTPUT_FILE"
echo 'let config = load_config().unwrap();' >> "$OUTPUT_FILE"
echo '' >> "$OUTPUT_FILE"
echo '// 之后' >> "$OUTPUT_FILE"
echo 'let config = load_config()' >> "$OUTPUT_FILE"
echo '    .context("Failed to load configuration")?;' >> "$OUTPUT_FILE"
echo '```' >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

# 6. 优先级建议
echo "## 修复优先级" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

echo "### 🔴 P0 - 立即修复" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"
echo "1. 配置加载路径" >> "$OUTPUT_FILE"
echo "2. 会话管理路径" >> "$OUTPUT_FILE"
echo "3. 工具执行路径" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

echo "### 🟡 P1 - 1-2 周内修复" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"
echo "1. LLM 通信路径" >> "$OUTPUT_FILE"
echo "2. MCP 集成路径" >> "$OUTPUT_FILE"
echo "3. 内存管理路径" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

echo "### 🟢 P2 - 逐步修复" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"
echo "1. 其他辅助功能" >> "$OUTPUT_FILE"
echo "2. 内部工具函数" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

# 7. 自动修复脚本
echo "## 自动修复脚本" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"
echo "可以使用以下命令查找特定模式的 unwrap:" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"
echo '```bash' >> "$OUTPUT_FILE"
echo '# 查找 Option::unwrap' >> "$OUTPUT_FILE"
echo 'rg "\.unwrap\(\)" --type rust crates/sage-core/src/config' >> "$OUTPUT_FILE"
echo '' >> "$OUTPUT_FILE"
echo '# 查找 Result::unwrap' >> "$OUTPUT_FILE"
echo 'rg "\.unwrap\(\)" --type rust crates/sage-core/src/session' >> "$OUTPUT_FILE"
echo '' >> "$OUTPUT_FILE"
echo '# 查找带消息的 expect' >> "$OUTPUT_FILE"
echo 'rg "\.expect\(" --type rust crates/sage-core/src/' >> "$OUTPUT_FILE"
echo '```' >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

echo "---" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"
echo "**建议**: 从高优先级文件开始，逐步替换 unwrap/expect 为正确的错误处理。" >> "$OUTPUT_FILE"

# 输出到终端
cat "$OUTPUT_FILE"

echo ""
echo "✅ 报告已保存到: $OUTPUT_FILE"
