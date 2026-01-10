# Crush - Slash Command 架构分析

## 概述

Crush 项目基于 BubbleTea 框架实现了一套完善的 slash command 系统。架构将命令定义、加载、显示和执行分离到不同层，支持三种命令类别：System（内置）、User（Markdown 模板）、MCP（Model Context Protocol）。

---

## 架构图

```
用户输入 "/" (输入框为空时)
       │
       ▼
┌─────────────────────────────────────────────────────────┐
│              Editor 组件检测触发                         │
│  editor.go - "/" on empty input                        │
└──────────────────────┬──────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────┐
│              Command Dialog 打开                        │
│  ├─ 加载 System Commands                               │
│  ├─ 加载 User Commands (from markdown)                  │
│  └─ 加载 MCP Prompts (from servers)                     │
└──────────────────────┬──────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────┐
│              用户选择/过滤命令                           │
│  - 上下键导航                                           │
│  - Tab 切换分类                                         │
│  - 输入过滤                                            │
└──────────────────────┬──────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────┐
│                 命令执行                                │
│  ├─ System Command → 直接执行 Handler                   │
│  ├─ User Command → Arguments Dialog (如有 $VARS)        │
│  │   └─ 模板替换 → 发送消息                             │
│  └─ MCP Prompt → Arguments Dialog (如有 args)           │
│      └─ MCP 执行 → 发送消息                            │
└─────────────────────────────────────────────────────────┘
```

---

## 组件层次

```
┌─────────────────────────────────────────────────────────┐
│                   UI Layer (TUI)                        │
│  - ChatPage (page/chat/chat.go)                         │
│  - EditorComponent (components/chat/editor/editor.go)   │
└─────────────────────┬───────────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────┐
│            Dialog Management Layer                      │
│  - DialogCmp (components/dialogs/dialogs.go)            │
│  - CommandsDialog (components/dialogs/commands/)        │
│  - ArgumentsDialog (components/dialogs/commands/)       │
└─────────────────────┬───────────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────┐
│        Command Definition & Loading Layer               │
│  - uicmd Package (internal/uicmd/uicmd.go)              │
│  - Markdown Parser                                      │
│  - MCP Integration (internal/agent/tools/mcp/)          │
└─────────────────────┬───────────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────┘
│         Execution & Message Handling                    │
│  - Command Handlers (tea.Cmd)                           │
│  - Custom Messages (CommandRunCustomMsg, etc.)          │
└─────────────────────────────────────────────────────────┘
```

---

## 命令类型

### CommandType 枚举

```go
type CommandType uint

const (
    SystemCommands CommandType = iota  // 内置命令
    UserCommands                       // 用户 Markdown 模板
    MCPPrompts                         // MCP 服务器 prompts
)
```

### Command 结构

```go
type Command struct {
    ID          string                        // 唯一标识
    Title       string                        // 显示名称
    Description string                        // 帮助文本
    Shortcut    string                        // 可选快捷键
    Handler     func(cmd Command) tea.Cmd     // 执行处理器
}
```

---

## 三类命令详解

### A. System Commands（内置）

动态生成，基于：
- 当前模型能力（reasoning 支持）
- 窗口尺寸（sidebar 仅 >120px 时显示）
- 环境变量（外部编辑器可用性）
- 活动会话

| 分类 | 命令 | 功能 |
|-----|------|------|
| 会话管理 | New Session, Switch Session | 管理聊天会话 |
| 模型控制 | Switch Model | 切换 LLM 模型 |
| AI 特性 | Enable/Disable Thinking, Select Reasoning Effort | 高级 LLM 功能 |
| UI 控制 | Toggle Sidebar, Toggle Compact Mode | 布局管理 |
| 文件 | Open File Picker | 附加文件到 prompt |
| 编辑器 | Open External Editor | 在 $EDITOR 中编辑 |
| 项目 | Initialize Project | 创建/更新 memory 文件 |
| 工具 | Summarize Session, Toggle Yolo Mode, Toggle Help, Quit | 各种实用功能 |

### B. User Commands（Markdown 模板）

**目录结构：**

```
XDG_CONFIG_HOME/crush/commands/   # 全局用户命令
~/.crush/commands/                 # 用户主目录命令
{project}/.crush/commands/         # 项目特定命令
```

**文件格式：** Markdown 文件 (`.md`)，支持命名参数

**参数模式：** `$PARAMETER_NAME`（大写字母、数字、下划线）

```regex
\$([A-Z][A-Z0-9_]*)
```

**示例命令：**

```markdown
# Generate Unit Tests
You are a testing expert. Generate comprehensive unit tests for:
$FILE_PATH

Focus on edge cases and $TEST_TYPE
```

**命令 ID 格式：**

```
user:subdir:command_name
project:command_name
```

### C. MCP Prompts

- 从连接的 MCP 服务器动态加载
- 每个服务器可提供多个 prompts
- Prompts 可有必需/可选参数
- 通过 pub/sub 系统实时同步

**命令 ID 格式：**

```
mcp_name:prompt_name
```

---

## 关键数据结构

### Dialog Model 层次

```
util.Model
    ↓
DialogModel (interface)
    ├─ Position() (row, col)
    ├─ ID() DialogID
    ├─ Update(msg) (Model, Cmd)
    ├─ View() string
    └─ Init() Cmd
        ↓
    ├─ CommandsDialog
    │   └─ commandDialogCmp
    │       ├─ commandList (FilterableList)
    │       ├─ userCommands []Command
    │       ├─ mcpPrompts *csync.Slice[Command]
    │       └─ selected (CommandType)
    │
    └─ CommandArgumentsDialog
        └─ commandArgumentsDialogCmp
            ├─ inputs []textinput.Model
            ├─ arguments []Argument
            └─ onSubmit func
```

### 消息类型

| 消息类型 | 来源 | 用途 |
|---------|------|------|
| `CommandRunCustomMsg` | User command handler | 发送模板内容到聊天 |
| `ShowArgumentsDialogMsg` | User/MCP commands | 显示参数输入对话框 |
| `ShowMCPPromptArgumentsDialogMsg` | MCP handler | MCP 特定参数对话框 |
| System Messages | System commands | 会话、模型、UI 变更 |

---

## 执行流程

### 1. 命令调用

**触发：** 用户在空 prompt 时按 "/"

```go
// editor/editor.go - Update()
case msg.String() == "/" && m.IsEmpty():
    return m, util.CmdHandler(dialogs.OpenDialogMsg{
        Model: commands.NewCommandDialog(m.session.ID),
    })
```

### 2. Dialog 初始化

```
NewCommandDialog(sessionID)
    ├─ LoadCustomCommands()
    │   ├─ 发现 user/project 目录中的 markdown 文件
    │   ├─ 解析 $PARAMETER_NAME 变量
    │   └─ 构建命令 ID: user:/project:/mcp:
    │
    ├─ LoadMCPPrompts()
    │   └─ 从 mcp.Prompts() 获取 prompts
    │
    └─ Init() → setCommandType(SystemCommands)
        └─ 填充 FilterableList
```

### 3. 命令选择

用户操作：
- 输入过滤/搜索
- 上下箭头导航
- Tab 切换分类
- Enter 选择

```go
case key.Matches(msg, c.keyMap.Select):
    selectedItem := c.commandList.SelectedItem()
    command := (*selectedItem).Value()
    return c, tea.Sequence(
        util.CmdHandler(dialogs.CloseDialogMsg{}),
        command.Handler(command),  // 执行 handler
    )
```

### 4. 执行路径

**路径 A: System Command（立即执行）**

```
System Command Handler
    └─ tea.Cmd 返回消息
        ├─ NewSessionsMsg → 创建会话
        ├─ SwitchSessionsMsg → 显示会话对话框
        ├─ SwitchModelMsg → 显示模型对话框
        └─ chat.SendMsg → 发送消息到 agent
```

**路径 B: User Command（带参数）**

```
User Command Handler
    └─ ShowArgumentsDialogMsg
        └─ ArgumentsDialog.Update()
            ├─ 每个 $PARAMETER_NAME 一个 TextInput
            └─ 确认时 → execUserPrompt()
                ├─ 用用户输入替换 $VAR
                └─ CommandRunCustomMsg {Content: result}
                    └─ chat.go handler → sendMessage(content)
```

**路径 C: MCP Prompt（带参数）**

```
MCP Prompt Handler
    └─ ShowMCPPromptArgumentsDialogMsg
        └─ ArgumentsDialog (MCP 变体)
            ├─ 从 prompt.Arguments 生成输入字段
            └─ 确认时 → execMCPPrompt()
                ├─ mcp.GetPromptMessages(ctx, clientName, promptName, args)
                └─ chat.SendMsg {Text: concatenated messages}
```

---

## 文件组织

```
internal/
├── uicmd/
│   └── uicmd.go (314 lines)
│       ├─ Command 结构 & 类型
│       ├─ LoadCustomCommands()
│       ├─ LoadMCPPrompts()
│       ├─ 模板变量解析
│       └─ 命令文件发现 & 加载
│
├── tui/
│   ├── components/dialogs/
│   │   ├── dialogs.go (Dialog 栈管理)
│   │   └── commands/
│   │       ├── commands.go (CommandsDialog - 482 lines)
│   │       ├── arguments.go (ArgumentsDialog - 246 lines)
│   │       └── keys.go (KeyBindings)
│   │
│   ├── components/chat/
│   │   └── editor/
│   │       └── editor.go ("/" 触发命令对话框)
│   │
│   ├── page/chat/
│   │   └── chat.go (CommandRunCustomMsg 处理)
│   │
│   └── tui.go (Ctrl+/ 重新触发命令对话框)
│
└── agent/tools/
    └── mcp/
        ├── prompts.go (MCP prompt 同步)
        ├── tools.go
        └── init.go
```

---

## 优势

### 1. 关注点分离

| 关注点 | 位置 | 好处 |
|-------|------|------|
| 命令加载 | uicmd.go | 可复用、可测试 |
| UI 渲染 | commands.go | 与加载解耦 |
| 键盘处理 | keys.go | 可复用 keymap |
| 参数解析 | arguments.go | 通用输入处理器 |
| 消息路由 | chat.go | 集中执行 |

### 2. 扩展性

1. **新增 System Command** - 添加到 `defaultCommands()` 方法
2. **User Commands** - 放入 `~/.crush/commands/` 目录
3. **MCP 集成** - MCP 服务器动态暴露 prompts

### 3. 多分类组织

- Radio 选择器切换命令类型
- 智能可用性（仅显示有内容的分类）
- 所有类型一致的过滤
- 按来源分组清晰

### 4. 模板系统

```go
// 模式: $大写_下划线
namedArgPattern = regexp.MustCompile(`\$([A-Z][A-Z0-9_]*)`)

// 支持同一变量多次出现
// extractArgNames() 去重
// execUserPrompt() 替换
```

---

## 劣势

### 1. 同步复杂性

MCP prompts 使用并发安全 map：

```go
mcpPrompts *csync.Slice[Command]  // 需要异步同步
```

### 2. Dialog 类型断言

运行时类型断言，无编译时安全：

```go
d.dialogs[lastIndex] = u.(DialogModel)  // 类型断言
```

### 3. 有限的参数验证

- 参数都是字符串
- 缺少类型提示（int, bool, enum）
- 无必需/可选区分
- 无默认值
- 无验证规则

### 4. 命令 ID 冲突

ID 作为显示名称：

```go
Title: id,  // 使用 ID 作为标题！
```

`user:subdir:command_name` 不够友好，应支持单独的 Title 字段。

### 5. 无命令元数据持久化

- 每次打开对话框重新加载
- 无缓存
- 无最近命令追踪
- 无使用统计

### 6. 静默错误处理

```go
cmd, err := l.loadCommand(path, source.path, source.prefix)
if err != nil {
    return nil // 静默跳过无效文件
}
```

---

## 设计模式

### 1. Model-Update-View (MUV) 模式

遵循 BubbleTea 核心架构：

```go
type Model interface {
    Init() tea.Cmd
    Update(msg tea.Msg) (Model, tea.Cmd)
    View() string
}
```

### 2. 消息驱动架构

命令通过类型化消息通信：

```go
type CommandRunCustomMsg struct {
    Content string
}
```

### 3. Builder 模式

命令对话框构造：

```go
NewCommandDialog(sessionID) CommandsDialog
    ├─ 创建 listModel
    ├─ 创建 keyMap
    ├─ 创建 help 显示
    └─ 初始化 SystemCommands
```

### 4. Handler 函数

命令包含可执行处理器：

```go
type Command struct {
    Handler func(cmd Command) tea.Cmd
}
```

Handler 通过闭包捕获变量（ID、描述、模板）。

### 5. 目录插件模式

通过文件系统发现用户命令：

```
~/.crush/commands/
├─ test.md          → user:test
├─ api/
│   └─ fetch.md    → user:api:fetch
```

**约定优于配置** 的方式。

---

## 适用于 Sage 的建议

### 高优先级

1. **采用多分类系统** - 分离 system、user、MCP 命令
2. **Markdown 模板支持** - 解析用户命令文件，支持 `$PARAMETER`
3. **MCP 集成模式** - 将 MCP prompts 作为一等命令

### 中优先级

1. **命令元数据** - YAML frontmatter 支持标题、描述、快捷键
2. **缓存 & 性能** - 缓存已加载命令，追踪使用统计
3. **参数增强** - 类型提示、默认值、验证规则

### 低优先级

1. **高级特性** - before/after 钩子、撤销支持、命令链
2. **UX 改进** - 命令收藏、最近命令、模糊搜索排序
