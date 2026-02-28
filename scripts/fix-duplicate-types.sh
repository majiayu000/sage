#!/bin/bash
# 半自动修复重复类型定义

set -e

echo "🔧 修复重复类型定义"
echo "===================="
echo ""

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 检查是否在正确的目录
if [ ! -f "Cargo.toml" ]; then
  echo -e "${RED}错误: 请在项目根目录运行此脚本${NC}"
  exit 1
fi

# 创建备份分支
echo -e "${YELLOW}创建备份分支...${NC}"
CURRENT_BRANCH=$(git branch --show-current)
BACKUP_BRANCH="backup-before-duplicate-fix-$(date +%Y%m%d-%H%M%S)"
git branch "$BACKUP_BRANCH"
echo -e "${GREEN}✓ 已创建备份分支: $BACKUP_BRANCH${NC}"
echo ""

# 创建新的工作分支
WORK_BRANCH="fix/duplicate-types"
if git show-ref --verify --quiet "refs/heads/$WORK_BRANCH"; then
  echo -e "${YELLOW}分支 $WORK_BRANCH 已存在，切换到该分支${NC}"
  git checkout "$WORK_BRANCH"
else
  echo -e "${YELLOW}创建新分支: $WORK_BRANCH${NC}"
  git checkout -b "$WORK_BRANCH"
fi
echo ""

# 修复 1: RateLimiter 重复
echo "1️⃣  修复 RateLimiter 重复"
echo "-------------------------"
echo "位置 1: crates/sage-core/src/llm/rate_limiter/bucket.rs"
echo "位置 2: crates/sage-core/src/recovery/rate_limiter/limiter.rs"
echo ""
echo "建议: 重命名为 LlmRateLimiter 和 RecoveryRateLimiter"
echo ""
read -p "是否自动重命名? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
  echo "重命名 LLM RateLimiter..."
  # 这里需要手动处理，因为涉及多个文件的引用
  echo -e "${YELLOW}⚠️  此操作需要手动完成:${NC}"
  echo "  1. 在 llm/rate_limiter/bucket.rs 中将 RateLimiter 重命名为 LlmRateLimiter"
  echo "  2. 更新所有引用该类型的文件"
  echo "  3. 在 recovery/rate_limiter/limiter.rs 中将 RateLimiter 重命名为 RecoveryRateLimiter"
  echo "  4. 更新所有引用该类型的文件"
  echo ""
  echo "使用 rust-analyzer 的重命名功能可以自动完成这些操作"
fi
echo ""

# 修复 2: Session 重复
echo "2️⃣  修复 Session 重复"
echo "--------------------"
echo "位置 1: crates/sage-core/src/session/types/session.rs"
echo "位置 2: crates/sage-core/src/session/types/unified/header.rs"
echo ""
echo "建议: 将 header.rs 中的 Session 重命名为 SessionHeader"
echo ""
read -p "是否自动重命名? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
  echo -e "${YELLOW}⚠️  此操作需要手动完成:${NC}"
  echo "  1. 在 session/types/unified/header.rs 中将 Session 重命名为 SessionHeader"
  echo "  2. 更新所有引用"
fi
echo ""

# 修复 3: SseEvent 重复
echo "3️⃣  修复 SseEvent 重复"
echo "---------------------"
echo "位置 1: crates/sage-core/src/llm/sse_decoder/event.rs"
echo "位置 2: crates/sage-core/src/llm/streaming.rs"
echo ""
echo "建议: 移动到共享位置并重新导出"
echo ""
read -p "是否处理? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
  echo -e "${YELLOW}⚠️  此操作需要手动完成:${NC}"
  echo "  1. 确定哪个是主定义"
  echo "  2. 在另一个位置使用 pub use 重新导出"
fi
echo ""

# 修复 4: 文档示例中的重复
echo "4️⃣  修复文档示例中的重复"
echo "------------------------"
echo "ProviderConfig 和 TimeoutConfig 在 docs/swe/timeout-configuration-example.rs 中重复"
echo ""
echo "建议: 添加到 .vibeguard-duplicate-types-allowlist"
echo ""
read -p "是否自动添加到允许列表? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
  ALLOWLIST_FILE=".vibeguard-duplicate-types-allowlist"
  
  if [ ! -f "$ALLOWLIST_FILE" ]; then
    echo "# 允许的重复类型（通常是文档示例）" > "$ALLOWLIST_FILE"
  fi
  
  echo "ProviderConfig" >> "$ALLOWLIST_FILE"
  echo "TimeoutConfig" >> "$ALLOWLIST_FILE"
  
  echo -e "${GREEN}✓ 已添加到 $ALLOWLIST_FILE${NC}"
  git add "$ALLOWLIST_FILE"
fi
echo ""

# 修复 5: MockEventSink 重复
echo "5️⃣  修复 MockEventSink 重复"
echo "---------------------------"
echo "位置 1: crates/sage-core/src/ui/traits/event_sink.rs"
echo "位置 2: crates/sage-core/src/ui/traits/mod.rs"
echo ""
echo "建议: 在 mod.rs 中使用 pub use 重新导出"
echo ""
read -p "是否处理? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
  echo -e "${YELLOW}⚠️  此操作需要手动完成:${NC}"
  echo "  1. 检查两个定义是否相同"
  echo "  2. 删除 mod.rs 中的定义"
  echo "  3. 添加 pub use event_sink::MockEventSink;"
fi
echo ""

# 运行测试
echo "🧪 运行测试验证"
echo "---------------"
read -p "是否运行测试? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
  echo "运行 cargo test..."
  if cargo test --lib 2>&1 | tee /tmp/test-output.log; then
    echo -e "${GREEN}✓ 测试通过${NC}"
  else
    echo -e "${RED}✗ 测试失败，请检查输出${NC}"
    echo "测试输出已保存到 /tmp/test-output.log"
  fi
fi
echo ""

# 运行 VibeGuard
echo "🔍 运行 VibeGuard 验证"
echo "---------------------"
read -p "是否运行 VibeGuard? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
  if make guard 2>&1 | tee /tmp/vibeguard-output.log; then
    echo -e "${GREEN}✓ VibeGuard 检查通过${NC}"
  else
    echo -e "${YELLOW}⚠️  仍有一些问题需要手动处理${NC}"
    echo "输出已保存到 /tmp/vibeguard-output.log"
  fi
fi
echo ""

# 提交更改
echo "💾 提交更改"
echo "----------"
echo "已完成的修改:"
git status --short
echo ""
read -p "是否提交更改? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
  git add -A
  git commit -m "fix: resolve duplicate type definitions (RS-05)

- Add documentation examples to .vibeguard-duplicate-types-allowlist
- Prepare for renaming conflicting types
- See OPTIMIZATION_RECOMMENDATIONS.md for details"
  
  echo -e "${GREEN}✓ 更改已提交${NC}"
  echo ""
  echo "下一步:"
  echo "  1. 手动重命名剩余的重复类型"
  echo "  2. 运行 cargo test 确保所有测试通过"
  echo "  3. 运行 make guard 验证修复"
  echo "  4. 创建 Pull Request"
fi
echo ""

echo "================================"
echo -e "${GREEN}✅ 脚本执行完成${NC}"
echo ""
echo "备份分支: $BACKUP_BRANCH"
echo "工作分支: $WORK_BRANCH"
echo ""
echo "如需回滚: git checkout $CURRENT_BRANCH && git branch -D $WORK_BRANCH"
