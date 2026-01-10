---
name: sage-prompt-engineering
description: Sage Prompt 工程规范，学习 Crush 的模板化方案，包含 system prompt 设计最佳实践
when_to_use: 当需要设计 prompt、修改 system prompt、或优化 AI 交互时使用
allowed_tools:
  - Read
  - Grep
  - Glob
  - Edit
  - Write
user_invocable: true
priority: 85
---

# Sage Prompt 工程规范

## 核心原则：模板化（学习 Crush）

**永远不要硬编码 prompt**，使用模板文件：

```
sage-core/src/prompts/
├── templates/
│   ├── coder.md.tpl        # 主 system prompt
│   ├── task.md.tpl         # 任务 agent prompt
│   ├── plan.md.tpl         # Plan mode prompt
│   └── initialize.md.tpl   # 初始化引导
├── renderer.rs             # 模板渲染器
├── variables.rs            # 变量定义
└── mod.rs
```

## 模板格式（.tpl 文件）

使用类似 Go template 的语法：

```markdown
You are {{.AgentName}}, a powerful AI assistant.

<env>
Working directory: {{.WorkingDir}}
Platform: {{.Platform}}
Today's date: {{.Date}}
{{if .IsGitRepo}}
Git branch: {{.GitBranch}}
{{end}}
</env>

{{if .ContextFiles}}
<memory>
{{range .ContextFiles}}
<file path="{{.Path}}">
{{.Content}}
</file>
{{end}}
</memory>
{{end}}

{{if .AvailableSkills}}
{{.SkillsXML}}
{{end}}
```

## System Prompt 结构（学习 Crush coder.md.tpl）

```markdown
# 1. 身份定义
You are {AgentName}, a powerful AI assistant...

# 2. 关键规则（critical_rules）
<critical_rules>
必须遵守的核心规则，覆盖其他所有内容：
1. 编辑前先读取文件
2. 自主决策，不要频繁询问
3. 修改后立即测试
...
</critical_rules>

# 3. 沟通风格（communication_style）
<communication_style>
- 保持简洁（默认 < 4 行）
- 无前言后缀
- 必要时可详细
</communication_style>

# 4. 工作流程（workflow）
<workflow>
执行任务的标准流程：
- 行动前：搜索、阅读、理解
- 行动中：精确编辑、测试验证
- 结束前：验证完成、保持简洁
</workflow>

# 5. 决策原则（decision_making）
<decision_making>
何时自主决策 vs 询问用户
</decision_making>

# 6. 具体指南
<editing_files>...</editing_files>
<error_handling>...</error_handling>
<testing>...</testing>
<tool_usage>...</tool_usage>

# 7. 环境信息（动态注入）
<env>
Working directory: {{.WorkingDir}}
...
</env>

# 8. 技能（动态注入）
{{.SkillsXML}}

# 9. 记忆文件（动态注入）
<memory>
{{.ContextFiles}}
</memory>
```

## 变量系统

### 内置变量

| 变量 | 类型 | 说明 |
|-----|------|------|
| `{{.AgentName}}` | string | Agent 名称 |
| `{{.WorkingDir}}` | string | 工作目录 |
| `{{.Platform}}` | string | 操作系统 |
| `{{.Date}}` | string | 当前日期 |
| `{{.IsGitRepo}}` | bool | 是否 Git 仓库 |
| `{{.GitBranch}}` | string | Git 分支 |
| `{{.GitStatus}}` | string | Git 状态 |
| `{{.ContextFiles}}` | []File | 上下文文件 |
| `{{.AvailableSkills}}` | []Skill | 可用技能 |
| `{{.SkillsXML}}` | string | 技能 XML |

### 条件渲染

```markdown
{{if .IsGitRepo}}
Git repository detected. Use git commands for version control.
{{else}}
No git repository. Consider initializing one with `git init`.
{{end}}
```

### 循环渲染

```markdown
{{range .ContextFiles}}
<file path="{{.Path}}">
{{.Content}}
</file>
{{end}}
```

## Rust 实现

### 模板渲染器

```rust
use handlebars::Handlebars;

pub struct PromptRenderer {
    handlebars: Handlebars<'static>,
}

impl PromptRenderer {
    pub fn new() -> Result<Self, RenderError> {
        let mut hb = Handlebars::new();

        // 加载所有模板
        hb.register_template_string("coder", include_str!("templates/coder.md.tpl"))?;
        hb.register_template_string("task", include_str!("templates/task.md.tpl"))?;
        hb.register_template_string("plan", include_str!("templates/plan.md.tpl"))?;

        Ok(Self { handlebars: hb })
    }

    pub fn render(&self, template: &str, vars: &PromptVariables) -> Result<String, RenderError> {
        self.handlebars.render(template, vars)
    }
}
```

### 变量结构

```rust
#[derive(Serialize)]
pub struct PromptVariables {
    pub agent_name: String,
    pub working_dir: PathBuf,
    pub platform: String,
    pub date: String,
    pub is_git_repo: bool,
    pub git_branch: Option<String>,
    pub git_status: Option<String>,
    pub context_files: Vec<ContextFile>,
    pub available_skills: Vec<SkillInfo>,
    pub skills_xml: String,
}

#[derive(Serialize)]
pub struct ContextFile {
    pub path: String,
    pub content: String,
}

#[derive(Serialize)]
pub struct SkillInfo {
    pub name: String,
    pub description: String,
    pub location: String,
}
```

## XML 标签规范

### 结构化内容使用 XML 标签

```markdown
<critical_rules>
内容...
</critical_rules>

<env>
环境信息...
</env>

<memory>
记忆内容...
</memory>

<available_skills>
<skill>
<name>skill-name</name>
<description>描述</description>
</skill>
</available_skills>
```

### 标签命名规范

- 使用 `snake_case`
- 语义清晰
- 避免嵌套过深（最多 3 层）

## 最佳实践

### 1. 分层组织

```
高优先级规则（覆盖其他）
    ↓
通用行为指南
    ↓
具体任务指南
    ↓
动态上下文（环境、技能、记忆）
```

### 2. 简洁有力

```markdown
# 不好
Please make sure that you read the file before you edit it, as this is very important for ensuring accuracy.

# 好
Read files before editing. No exceptions.
```

### 3. 使用示例

```markdown
# 不好
Use concise responses.

# 好
<communication_style>
Examples:
user: what is 2+2?
assistant: 4

user: list files in src/
assistant: [uses ls tool]
foo.c, bar.c, baz.c
</communication_style>
```

### 4. 明确边界

```markdown
# 好
**Only stop/ask user if**:
- Truly ambiguous business requirement
- Multiple valid approaches with big tradeoffs
- Could cause data loss

**Never stop for**:
- Task seems too large
- Multiple files to change
- Work will take many steps
```

## Skill 注入格式

```xml
<available_skills>
  <skill>
    <name>rust-expert</name>
    <description>Rust 编程专家</description>
    <location>.sage/skills/rust-expert/SKILL.md</location>
  </skill>
  <skill>
    <name>testing</name>
    <description>TDD 和测试最佳实践</description>
    <location>.sage/skills/testing/SKILL.md</location>
  </skill>
</available_skills>

<skills_usage>
When a user task matches a skill's description, read the skill's SKILL.md file.
Skills are activated by reading their location path.
</skills_usage>
```

## 检查清单

编写/修改 prompt 前确认：

- [ ] 使用模板文件而非硬编码
- [ ] 变量命名清晰一致
- [ ] 规则按优先级排序
- [ ] 有具体示例说明
- [ ] XML 标签正确闭合
- [ ] 测试模板渲染
- [ ] 检查变量替换正确
