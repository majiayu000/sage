---
name: sage-skill-development
description: Sage Skill 开发规范，遵循 Agent Skills 开放标准，兼容 Claude Code 格式
when_to_use: 当需要创建新 Skill、设计 Skill 系统、或理解 Skill 工作原理时使用
allowed_tools:
  - Read
  - Grep
  - Glob
  - Edit
  - Write
user_invocable: true
priority: 90
---

# Sage Skill 开发规范

## 标准遵循

Sage 的 Skill 系统遵循两个标准：

1. **Agent Skills 开放标准** (https://agentskills.io) - Crush 采用
2. **Claude Code Skill 格式** - 扩展兼容

## SKILL.md 完整格式

```markdown
---
# === 基础字段（Agent Skills 标准）===
name: skill-name                    # 必需，用于调用（/skill-name）
description: 技能的简短描述          # 必需，显示在技能列表
license: MIT                        # 可选，许可证
compatibility: sage >= 0.1.0        # 可选，兼容性说明
metadata:                           # 可选，扩展元数据
  author: your-name
  version: "1.0.0"
  tags: [rust, testing]

# === 扩展字段（Claude Code 兼容）===
when_to_use: 当用户请求...时使用    # AI 自动触发条件
allowed_tools:                      # 允许使用的工具
  - Read
  - Grep
  - Glob
  - Edit
  - Bash
user_invocable: true                # 是否可通过 /skill-name 调用
argument_hint: "[file or directory]" # 参数提示
priority: 10                        # 优先级（高优先匹配）
model: sonnet                       # 可选，指定模型
---

# Skill 标题

## 上下文变量

在 Skill 内容中可使用以下变量：

- `$ARGUMENTS` - 用户传入的参数
- `$USER_MESSAGE` - 用户原始消息
- `$WORKING_DIR` - 当前工作目录
- `$FILE_CONTEXT` - 相关文件上下文

## 详细指令

这里是 Skill 的详细指令内容...
```

## Skill 发现路径

按优先级从高到低：

```
1. .sage/skills/             # 项目级（最高优先）
2. ~/.config/sage/skills/    # 用户级
3. MCP 服务提供              # 动态发现
4. 内置 skills               # 最低优先
```

## 目录结构

### 方式一：单文件（简单 Skill）

```
.sage/skills/
├── git-commit.md
├── code-review.md
└── testing.md
```

### 方式二：目录（复杂 Skill，推荐）

```
.sage/skills/
├── rust-expert/
│   ├── SKILL.md           # 主文件
│   ├── references/        # 参考资料
│   │   ├── idioms.md
│   │   └── patterns.md
│   ├── scripts/           # 辅助脚本
│   │   └── check-style.sh
│   └── assets/            # 资源文件
│       └── logo.png
├── testing/
│   └── SKILL.md
```

## 内置 Skill 示例

### rust-expert

```markdown
---
name: rust-expert
description: Rust 编程专家，提供惯用代码、性能优化和安全最佳实践
when_to_use: 当处理 .rs 文件或用户提到 Rust 时
allowed_tools:
  - Read
  - Grep
  - Glob
  - Edit
  - Bash
priority: 20
---

# Rust 编程专家

你是 Rust 编程专家，遵循以下原则：

## 命名规范（RFC 430）

- 类型：PascalCase，缩写词当普通单词处理
  - 正确：`LlmClient`, `HttpRequest`, `SseDecoder`
  - 错误：`LLMClient`, `HTTPRequest`, `SSEDecoder`

## 错误处理

- 使用 `thiserror` 定义错误类型
- 使用 `anyhow` 处理应用错误
- 永远不要 `unwrap()`，使用 `?` 或 `expect("reason")`

## 异步代码

- 使用 `tokio` 运行时
- 避免在异步上下文中阻塞
- 使用 `async_trait` 处理异步 trait

## 性能

- 优先使用零拷贝（`&str` over `String`）
- 使用 `Cow<'_, str>` 处理可能克隆的场景
- 避免不必要的 `clone()`

$ARGUMENTS
```

### comprehensive-testing

```markdown
---
name: comprehensive-testing
description: TDD 方法论和全面测试策略
when_to_use: 当用户提到测试、TDD、或需要编写测试时
allowed_tools:
  - Read
  - Grep
  - Glob
  - Edit
  - Bash
user_invocable: true
argument_hint: "[test file or module]"
priority: 15
---

# 测试专家

你是测试驱动开发专家，遵循 RED-GREEN-REFACTOR 循环。

## TDD 流程

1. **RED**: 先写失败的测试
2. **GREEN**: 写最少代码使测试通过
3. **REFACTOR**: 重构代码，保持测试通过

## 测试金字塔

```
        /\
       /  \  E2E (10%)
      /────\
     /      \ 集成测试 (20%)
    /────────\
   /          \ 单元测试 (70%)
  /────────────\
```

## Rust 测试模式

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_case() {
        // Arrange
        let input = ...;

        // Act
        let result = function_under_test(input);

        // Assert
        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn test_async_function() {
        // 异步测试
    }
}
```

$ARGUMENTS
```

## Skill 触发机制

### 自动触发（AI 决定）

当 `when_to_use` 匹配用户意图时，AI 自动激活：

```yaml
when_to_use: 当用户请求代码审查、PR review 或代码质量检查时
```

### 手动触发

用户通过 `/skill-name` 调用：

```
用户: /rust-expert 帮我优化这段代码
```

### 触发优先级

```
1. 用户显式调用 (/skill-name) - 最高
2. when_to_use 匹配 + 高 priority 值
3. 关键词匹配（triggers 字段）
4. 文件扩展名匹配
```

## Skill 上下文注入

Skill 被激活后，其内容会注入到 system prompt：

```xml
<active_skill>
<name>rust-expert</name>
<instructions>
[Skill 的 markdown 内容]
</instructions>
</active_skill>
```

## 检查清单

创建新 Skill 前确认：

- [ ] 名称符合 `[a-z0-9]+(-[a-z0-9]+)*` 格式
- [ ] 描述清晰简洁（< 200 字符）
- [ ] `when_to_use` 明确（如需自动触发）
- [ ] `allowed_tools` 最小权限原则
- [ ] 内容包含具体指令而非泛泛而谈
- [ ] 使用变量 `$ARGUMENTS` 接收参数
- [ ] 测试 Skill 能否正确激活

## 高级特性

### 模型覆盖

特定 Skill 使用特定模型：

```yaml
model: haiku  # 快速简单任务
model: opus   # 复杂推理任务
```

### 工具限制

只读 Skill：

```yaml
allowed_tools:
  - Read
  - Grep
  - Glob
  - WebFetch
  - WebSearch
```

### 脚本集成

在 Skill 目录中放置脚本：

```
my-skill/
├── SKILL.md
└── scripts/
    └── analyze.sh
```

在 Skill 中引用：

```markdown
运行分析脚本：
\`\`\`bash
./scripts/analyze.sh $ARGUMENTS
\`\`\`
```
