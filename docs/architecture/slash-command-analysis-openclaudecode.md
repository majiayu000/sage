# Open Claude Code - Slash Command 架构分析

## 概述

Open Claude Code 的 slash command 系统采用**多层架构**设计，具有强大的权限控制和灵活的扩展机制。

---

## 架构图

```
用户输入 "/command"
       │
       ▼
┌─────────────────────────────────────────────────────────┐
│                 UI Layer (React/Ink)                    │
│  - Terminal UI with status bar                          │
│  - Command suggestions                                  │
└──────────────────────┬──────────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────────┐
│          Command Processing Layer                       │
│  - Slash command parser                                 │
│  - Skill Tool (AI-accessible)                           │
└──────────────────────┬──────────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────────┐
│       Permission & Authorization Layer                  │
│  - Multi-level permission evaluation                    │
│  - Tool restrictions per skill                          │
│  - Sandbox execution                                    │
└──────────────────────┬──────────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────────┐
│              Execution Layer                            │
│  - Bash command handlers                                │
│  - MCP CLI integration                                  │
│  - Built-in command handlers                            │
└─────────────────────────────────────────────────────────┘
```

---

## 命令类型

### 1. Local-JSX Commands

立即执行，返回 React 组件用于 UI 显示。

| 命令 | 功能 |
|-----|------|
| `/help` | 显示帮助信息 |
| `/config` | 配置管理 |
| `/status` | 显示状态 |

### 2. Prompt Commands (Skills)

注入 Markdown prompt 到 AI，可限制工具访问权限。

| 命令 | 功能 |
|-----|------|
| `/commit` | 智能提交助手 |
| `/review` | 代码审查 |
| `/test` | 测试生成 |

---

## Skill 系统（核心创新）

Skills 是 **Markdown + YAML Frontmatter** 文件，支持：

- 定义可复用的 AI 工作流
- 三级组织：user、project、plugin
- 通过 `allowed-tools` 限制工具访问
- 使用 `$ARGUMENTS` 参数注入
- 支持 model 选择覆盖

### Skill 文件示例

```markdown
---
description: "Smart commit helper"
allowed-tools: [Bash, Read, Grep]
when_to_use: "When user needs to commit code"
model: "sonnet"
---

Help me commit: $ARGUMENTS
```

### Skill 目录结构

```
~/.claude-code/skills/          # 用户级
{project}/.claude-code/skills/  # 项目级
{plugin}/skills/                # 插件级
```

---

## 权限系统

### 五级评估链

```
Policy (系统策略)
    ↓
User (用户配置)
    ↓
Project (项目配置)
    ↓
Skill (技能限制)
    ↓
Tool (工具默认)
```

### 规则行为

| 行为 | 说明 |
|-----|------|
| `allow` | 自动允许，无需确认 |
| `deny` | 直接拒绝 |
| `ask` | 需要用户确认 |

### 工具限制

每个 Skill 可以定义允许使用的工具列表：

```yaml
allowed-tools: [Bash, Read, Grep]
```

### 沙箱执行

危险命令会在隔离环境中执行：
- `rm -rf` 相关
- `fork bomb`
- 系统关键命令

---

## 安全特性

### 命令排除列表

```
su, sudo -i, shutdown, reboot, halt, init, poweroff
```

### 危险命令检测

```
rm -rf /
:(){ :|:& };:
dd if=/dev/zero
```

### 敏感数据编辑

自动识别并屏蔽：
- API Keys
- Tokens
- Passwords

---

## 内置命令列表

| 命令 | 类型 | 功能 |
|-----|------|------|
| `/help` | local-jsx | 显示帮助 |
| `/config` | local-jsx | 配置管理 |
| `/status` | local-jsx | 状态显示 |
| `/commit` | prompt | Git 提交 |
| `/review` | prompt | 代码审查 |
| `/test` | prompt | 测试生成 |
| `/explain` | prompt | 代码解释 |
| `/fix` | prompt | Bug 修复 |
| `/refactor` | prompt | 代码重构 |
| `/docs` | prompt | 文档生成 |
| `/init` | local | 项目初始化 |
| `/clear` | local | 清除对话 |
| `/model` | local | 切换模型 |
| `/compact` | local | 紧凑模式 |

---

## 关键数据结构

### Command Definition

```typescript
interface CommandDefinition {
  name: string;
  type: 'local-jsx' | 'local' | 'prompt';
  description: string;
  handler?: (args: string) => Promise<CommandResult>;
  promptTemplate?: string;
  allowedTools?: string[];
  requiresConfirmation?: boolean;
}
```

### CommandResult

```typescript
interface CommandResult {
  type: 'local' | 'prompt';
  content?: string | React.ReactNode;
  promptInjection?: string;
  toolRestrictions?: string[];
}
```

### Skill Metadata

```typescript
interface SkillMetadata {
  description: string;
  allowedTools?: string[];
  whenToUse?: string;
  model?: string;
  arguments?: ArgumentDefinition[];
}
```

---

## 优势

1. **多层权限系统** - 细粒度的访问控制
2. **Markdown 技能定义** - 易于编写和维护
3. **工具限制** - 安全地限制 AI 访问
4. **沙箱执行** - 危险命令隔离
5. **深度 AI 集成** - 自动推荐命令
6. **跨平台支持** - 统一的命令接口
7. **插件扩展** - 社区贡献的技能

---

## 劣势

1. **文件体积大** - 单文件达 372KB
2. **Skill 元数据有限** - YAML 解析基础
3. **代码混淆** - 社区贡献困难
4. **15,000 字符限制** - 单个 Skill 内容上限
5. **文档不足** - 示例和教程较少

---

## 设计模式

### 1. 命令定义模式

声明式对象定义：

```typescript
const commands = {
  commit: {
    name: 'commit',
    type: 'prompt',
    description: 'Smart git commit',
    allowedTools: ['Bash', 'Read', 'Grep'],
    promptTemplate: '...',
  },
};
```

### 2. 权限级联模式

多级权限回退评估：

```typescript
function evaluatePermission(tool, context) {
  // Policy level
  if (policy.has(tool)) return policy.get(tool);

  // User level
  if (userConfig.has(tool)) return userConfig.get(tool);

  // Project level
  if (projectConfig.has(tool)) return projectConfig.get(tool);

  // Skill level
  if (context.skill?.allowedTools) {
    return context.skill.allowedTools.includes(tool) ? 'allow' : 'deny';
  }

  // Default
  return tool.defaultPermission;
}
```

### 3. Skill 注册模式

全局会话状态跟踪：

```typescript
class SkillRegistry {
  private invokedSkills: Set<string> = new Set();

  invoke(skillName: string) {
    this.invokedSkills.add(skillName);
    // Update global session state
  }
}
```

---

## 对比总结

| 特性 | Claude Code Skills | Shell Scripts | 传统插件 |
|-----|-------------------|---------------|---------|
| 定义方式 | Markdown + YAML | Shell 语法 | 代码 |
| AI 感知 | 原生支持 | 否 | 否 |
| 安全性 | 工具限制 | 无限制 | 完全访问 |
| 上手难度 | 低 | 中 | 高 |
| 跨平台 | 优秀 | 差 | 中等 |

---

## 适用于 Sage 的建议

### 高优先级

1. **采用 Skill 系统** - Markdown + YAML 定义命令
2. **实现权限层级** - 至少 3 级（User、Project、Tool）
3. **工具限制** - 每个命令可限制可用工具

### 中优先级

1. **命令发现** - 自动扫描 skills 目录
2. **参数模板** - 支持 `$ARGUMENTS` 注入
3. **模型选择** - 命令可指定使用的模型

### 低优先级

1. **沙箱执行** - 危险命令隔离
2. **插件系统** - 社区技能分发
3. **AI 推荐** - 根据上下文推荐命令
