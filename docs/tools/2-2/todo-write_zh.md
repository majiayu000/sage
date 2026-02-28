# TodoWrite Tool

## 基本信息

| 属性 | 值 |
|------|-----|
| 工具名称 | TodoWrite |
| Claude Code 版本 | 2.0.14 |
| 类别 | 任务管理 |
| Sage 实现 | `sage-tools/src/tools/task_mgmt/todo_write/` |

## 功能描述

创建和管理结构化的任务列表，跟踪编码会话中的进度。

## 完整 Prompt

```markdown
Use this tool to create and manage a structured task list for your current coding session. This helps you track progress, organize complex tasks, and demonstrate thoroughness to the user.
It also helps the user understand the progress of the task and overall progress of their requests.

## When to Use This Tool
Use this tool proactively in these scenarios:

1. Complex multi-step tasks - When a task requires 3 or more distinct steps or actions
2. Non-trivial and complex tasks - Tasks that require careful planning or multiple operations
3. User explicitly requests todo list - When the user directly asks you to use the todo list
4. User provides multiple tasks - When users provide a list of things to be done (numbered or comma-separated)
5. After receiving new instructions - Immediately capture user requirements as todos
6. When you start working on a task - Mark it as in_progress BEFORE beginning work. Ideally you should only have one todo as in_progress at a time
7. After completing a task - Mark it as completed and add any new follow-up tasks discovered during implementation

## When NOT to Use This Tool

Skip using this tool when:
1. There is only a single, straightforward task
2. The task is trivial and tracking it provides no organizational benefit
3. The task can be completed in less than 3 trivial steps
4. The task is purely conversational or informational

NOTE that you should not use this tool if there is only one trivial task to do. In this case you are better off just doing the task directly.

## Task States and Management

1. **Task States**: Use these states to track progress:
   - pending: Task not yet started
   - in_progress: Currently working on (limit to ONE task at a time)
   - completed: Task finished successfully

   **IMPORTANT**: Task descriptions must have two forms:
   - content: The imperative form describing what needs to be done (e.g., "Run tests", "Build the project")
   - activeForm: The present continuous form shown during execution (e.g., "Running tests", "Building the project")

2. **Task Management**:
   - Update task status in real-time as you work
   - Mark tasks complete IMMEDIATELY after finishing (don't batch completions)
   - Exactly ONE task must be in_progress at any time (not less, not more)
   - Complete current tasks before starting new ones
   - Remove tasks that are no longer relevant from the list entirely

3. **Task Completion Requirements**:
   - ONLY mark a task as completed when you have FULLY accomplished it
   - If you encounter errors, blockers, or cannot finish, keep the task as in_progress
   - When blocked, create a new task describing what needs to be resolved
   - Never mark a task as completed if:
     - Tests are failing
     - Implementation is partial
     - You encountered unresolved errors
     - You couldn't find necessary files or dependencies
```

## 参数

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| todos | array | ✅ | 任务列表 |
| todos[].content | string | ✅ | 任务描述 (祈使句) |
| todos[].activeForm | string | ✅ | 进行中显示 (现在进行时) |
| todos[].status | string | ✅ | 状态: pending/in_progress/completed |

## 设计原理

### 1. 双重描述形式
**为什么**:
- `content`: 描述要做什么 ("Run tests")
- `activeForm`: 显示正在做什么 ("Running tests")
- 提供更好的用户体验

### 2. 单任务进行中
**为什么**:
- 保持专注
- 清晰的进度指示
- 避免并行混乱

### 3. 立即更新状态
**为什么**:
- 实时反馈给用户
- 避免状态不一致
- 便于跟踪进度

### 4. 严格的完成标准
**为什么**:
- 防止虚假完成
- 确保质量
- 遇到问题时创建新任务

### 5. 主动使用
**为什么**:
- 复杂任务需要组织
- 帮助用户理解进度
- 展示工作的系统性

## 使用场景

### ✅ 应该使用

**示例 1: 添加暗色模式**
```
用户: 添加暗色模式切换功能，完成后运行测试和构建
任务列表:
1. Creating dark mode toggle component
2. Adding dark mode state management
3. Implementing CSS styles for dark theme
4. Updating components to support theme switching
5. Running tests and build process
```

**示例 2: 重命名函数**
```
用户: 将 getCwd 重命名为 getCurrentWorkingDirectory
(搜索后发现 15 处引用)
任务列表:
1. Updating getCwd in src/utils.rs
2. Updating getCwd in src/main.rs
...
```

### ❌ 不应该使用

**示例 1: 简单问答**
```
用户: Python 怎么打印 Hello World?
回答: print("Hello World")
(无需任务列表)
```

**示例 2: 单个简单修改**
```
用户: 给 calculateTotal 函数添加注释
(直接编辑，无需任务列表)
```

## Sage 实现差异

Sage 提供两个相关工具：
- `TodoWriteTool`: 兼容 Claude Code 的实现
- `TaskCreate/TaskUpdate/TaskList`: 更强大的任务管理系统，支持依赖关系
