# Skill Tool

## 基本信息

| 属性 | 值 |
|------|-----|
| 工具名称 | Skill |
| Claude Code 版本 | 2.1.23 |
| 类别 | 扩展工具 |
| Sage 实现 | `sage-tools/src/tools/extensions/skill.rs` |

## 功能描述

在主对话中执行技能，提供专门的能力和领域知识。

## 完整 Prompt

```markdown
Execute a skill within the main conversation

When users ask you to perform tasks, check if any of the available skills match. Skills provide specialized capabilities and domain knowledge.

When users reference a "slash command" or "/<something>" (e.g., "/commit", "/review-pr"), they are referring to a skill. Use this tool to invoke it.

How to invoke:
- Use this tool with the skill name and optional arguments
- Examples:
  - `skill: "pdf"` - invoke the pdf skill
  - `skill: "commit", args: "-m 'Fix bug'"` - invoke with arguments
  - `skill: "review-pr", args: "123"` - invoke with arguments
  - `skill: "ms-office-suite:pdf"` - invoke using fully qualified name

Important:
- Available skills are listed in system-reminder messages in the conversation
- When a skill matches the user's request, this is a BLOCKING REQUIREMENT: invoke the relevant Skill tool BEFORE generating any other response about the task
- NEVER mention a skill without actually calling this tool
- Do not invoke a skill that is already running
- Do not use this tool for built-in CLI commands (like /help, /clear, etc.)
- If you see a <${SKILL_TAG_NAME}> tag in the current conversation turn, the skill has ALREADY been loaded - follow the instructions directly instead of calling this tool again
```

## 参数

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| skill | string | ✅ | 技能名称 |
| args | string | ❌ | 技能参数 |

## 设计原理

### 1. 斜杠命令映射
**为什么**:
- 用户熟悉的交互方式
- 快速调用常用功能
- 与 CLI 工具一致

### 2. 阻塞式要求
**为什么**:
- 技能应该立即执行
- 避免先描述再执行
- 保持响应简洁

### 3. 避免重复调用
**为什么**:
- 技能可能已经加载
- 检查 skill tag 避免重复
- 提高效率

### 4. 完全限定名
**为什么**:
- 支持命名空间
- 避免名称冲突
- 如 `ms-office-suite:pdf`

## 使用场景

### ✅ 应该使用
- 用户输入 `/commit`
- 用户请求 "review this PR"
- 匹配可用技能的任务

### ❌ 不应该使用
- 内置 CLI 命令 (`/help`, `/clear`)
- 技能已经在运行
- 技能 tag 已经存在

## 常见技能

| 技能 | 用途 |
|------|------|
| commit | Git 提交 |
| review-pr | PR 审查 |
| pdf | PDF 处理 |
| test | 运行测试 |

## 示例

```markdown
用户: /commit -m "Fix login bug"
Agent: [调用 Skill 工具]
{
  "skill": "commit",
  "args": "-m 'Fix login bug'"
}
```

## 变量说明

| 变量 | 说明 |
|------|------|
| SKILL_TAG_NAME | 技能标签名称 (如 "command-name") |

## Sage 实现差异

Sage 的 SkillTool 支持：
- 自定义技能注册
- 技能参数验证
- 技能执行历史
- 技能依赖管理
