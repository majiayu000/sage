# Claude Code 工具文档 (v2.x)

本目录包含 Claude Code 所有工具的详细文档，包括每个工具的 prompt 设计、使用场景和设计原理。

## 工具分类

### 文件操作工具
| 工具 | 文件 | 说明 |
|------|------|------|
| Read | [read.md](./read.md) | 读取文件内容 |
| Write | [write.md](./write.md) | 写入文件 |
| Edit | [edit.md](./edit.md) | 精确字符串替换 |
| Glob | [glob.md](./glob.md) | 文件模式匹配 |
| Grep | [grep.md](./grep.md) | 内容搜索 (ripgrep) |
| NotebookEdit | [notebook-edit.md](./notebook-edit.md) | Jupyter notebook 编辑 |

### 执行工具
| 工具 | 文件 | 说明 |
|------|------|------|
| Bash | [bash.md](./bash.md) | Shell 命令执行 |
| Task | [task.md](./task.md) | 子 Agent 启动 |
| TaskOutput | [task-output.md](./task-output.md) | 获取后台任务输出 |
| TaskStop | [task-stop.md](./task-stop.md) | 停止后台任务 |

### 任务管理工具
| 工具 | 文件 | 说明 |
|------|------|------|
| TaskCreate | [task-create.md](./task-create.md) | 创建任务 |
| TaskUpdate | [task-update.md](./task-update.md) | 更新任务状态 |
| TaskList | [task-list.md](./task-list.md) | 列出所有任务 |
| TaskGet | [task-get.md](./task-get.md) | 获取任务详情 |
| TodoWrite | [todo-write.md](./todo-write.md) | Todo 列表管理 |

### 规划工具
| 工具 | 文件 | 说明 |
|------|------|------|
| EnterPlanMode | [enter-plan-mode.md](./enter-plan-mode.md) | 进入规划模式 |
| ExitPlanMode | [exit-plan-mode.md](./exit-plan-mode.md) | 退出规划模式 |

### 交互工具
| 工具 | 文件 | 说明 |
|------|------|------|
| AskUserQuestion | [ask-user-question.md](./ask-user-question.md) | 向用户提问 |

### 网络工具
| 工具 | 文件 | 说明 |
|------|------|------|
| WebFetch | [web-fetch.md](./web-fetch.md) | 获取网页内容 |
| WebSearch | [web-search.md](./web-search.md) | 网络搜索 |

### 扩展工具
| 工具 | 文件 | 说明 |
|------|------|------|
| Skill | [skill.md](./skill.md) | 执行技能 |
| ToolSearch | [tool-search.md](./tool-search.md) | 搜索延迟加载工具 |

### 代码智能工具
| 工具 | 文件 | 说明 |
|------|------|------|
| LSP | [lsp.md](./lsp.md) | Language Server Protocol |

### 浏览器自动化工具
| 工具 | 文件 | 说明 |
|------|------|------|
| Computer | [computer.md](./computer.md) | 浏览器鼠标键盘控制 |

### 团队协作工具
| 工具 | 文件 | 说明 |
|------|------|------|
| TeammateTool | [teammate-tool.md](./teammate-tool.md) | 团队管理 |
| SendMessageTool | [send-message-tool.md](./send-message-tool.md) | 队友消息发送 |

## 工具统计

| 类别 | 数量 |
|------|------|
| 文件操作 | 6 |
| 执行工具 | 4 |
| 任务管理 | 5 |
| 规划工具 | 2 |
| 交互工具 | 1 |
| 网络工具 | 2 |
| 扩展工具 | 2 |
| 代码智能 | 1 |
| 浏览器自动化 | 1 |
| 团队协作 | 2 |
| **总计** | **26** |

## Prompt 设计原则

Claude Code 的工具 prompt 遵循以下设计原则：

### 1. 元数据头部
```markdown
<!--
name: 'Tool Description: [Name]'
description: [Brief description]
ccVersion: [Version number]
variables:
  - VARIABLE_NAME
-->
```

### 2. 结构化内容
- **开头声明**: 单句描述核心功能
- **Usage 部分**: 使用前提、参数要求、输出格式
- **Guidelines 部分**: 何时使用、何时不使用
- **Important Notes**: 警告和关键信息

### 3. 变量插值
使用 `${VARIABLE_NAME}` 进行动态内容替换，支持：
- 工具名称引用
- 配置值
- 条件内容
- 功能开关

### 4. 示例驱动
```markdown
<example>
user: [User request]
assistant: [Assistant response]
</example>

<reasoning>
[Explanation of why this approach was chosen]
</reasoning>
```

## Sage 与 Claude Code 工具对比

| Claude Code 工具 | Sage 实现 | 状态 |
|-----------------|----------|------|
| Read | ReadTool | ✅ 完整 |
| Write | WriteTool | ✅ 完整 |
| Edit | EditTool | ✅ 完整 |
| Glob | GlobTool | ✅ 完整 |
| Grep | GrepTool | ✅ 完整 |
| NotebookEdit | NotebookEditTool | ✅ 完整 |
| Bash | BashTool | ✅ 完整 |
| Task | TaskTool | ✅ 完整 |
| TaskOutput | TaskOutputTool | ✅ 完整 |
| TaskStop | KillShellTool | ✅ 完整 |
| TaskCreate | TaskCreate | ✅ 完整 |
| TaskUpdate | TaskUpdate | ✅ 完整 |
| TaskList | TaskList | ✅ 完整 |
| TaskGet | TaskGet | ✅ 完整 |
| TodoWrite | TodoWriteTool | ✅ 完整 |
| EnterPlanMode | EnterPlanModeTool | ✅ 完整 |
| ExitPlanMode | ExitPlanModeTool | ✅ 完整 |
| AskUserQuestion | AskUserQuestionTool | ✅ 完整 |
| WebFetch | WebFetchTool | ✅ 完整 |
| WebSearch | WebSearchTool | ✅ 完整 |
| Skill | SkillTool | ✅ 完整 |
| ToolSearch | - | ❌ 未实现 |
| LSP | DiagnosticsTool (部分) | ⚠️ 部分 |
| Computer | BrowserTool (替代) | ⚠️ 替代 |
| TeammateTool | - | ❌ 未实现 |
| SendMessageTool | - | ❌ 未实现 |

### Sage 独有工具
| 工具 | 说明 |
|------|------|
| GitTool | Git 版本控制 |
| HttpClientTool | HTTP 客户端 |
| BrowserTool | 浏览器自动化 (Playwright) |
| DockerTool | Docker 操作 |
| SqlTool | SQL 数据库操作 |
| LearnTool | 学习用户偏好 |
| RememberTool | 长期记忆管理 |
| SequentialThinkingTool | 顺序思考模式 |
| LogAnalyzerTool | 日志分析 |
| TestGeneratorTool | 测试生成 |
| SecurityScannerTool | 安全扫描 |
| KubernetesTool | K8s 操作 |
| TerraformTool | 基础设施即代码 |
