# Quick Start Guide / 快速入门指南

## Prerequisites / 前置条件

Before starting, ensure you have:
开始之前,请确保您已:

1. Installed Sage CLI → [Installation Guide](installation.md)
   安装了 Sage CLI → [安装指南](installation.md)

2. Configured API keys → [Configuration Guide](configuration.md)
   配置了 API 密钥 → [配置指南](configuration.md)

---

## Your First Command / 第一个命令

### Quick Test / 快速测试

```bash
# Run a simple task
# 运行一个简单任务
sage run "Create a Python script that prints 'Hello, Sage!'"
```

This will:
这将:
- Execute the task using your default LLM provider
- Create the requested Python file
- Show execution progress
- Display results

---

## Three Execution Modes / 三种执行模式

Sage offers three distinct modes for different use cases:
Sage 为不同用例提供三种不同的模式:

### 1. Interactive Mode (Default) / 交互模式(默认)

**Best for:** Iterative development, exploration, conversations
**最适合:** 迭代开发、探索、对话

```bash
# Start interactive mode
# 启动交互模式
sage interactive

# Or simply
# 或简单地
sage
```

**Example Session / 示例会话:**
```
Sage> Create a calculator.py file with add and subtract functions
[Agent creates calculator.py]

Sage> Now add multiply and divide functions
[Agent modifies calculator.py with new functions]

Sage> Add error handling for division by zero
[Agent updates the divide function]

Sage> /cost
Session Cost & Usage
Total tokens: 2,450
Estimated cost: $0.12

Sage> exit
```

**Interactive Commands / 交互命令:**
- `help` - Show available commands
- `config` - Display configuration
- `status` - System status
- `new` - Start new conversation
- `clear` - Clear screen
- `exit` / `quit` - Exit

---

### 2. Run Mode (One-shot) / 运行模式(一次性)

**Best for:** Single tasks, automation, scripting
**最适合:** 单一任务、自动化、脚本

```bash
# Execute a single task and exit
# 执行单个任务并退出
sage run "Create a README.md with project description"

# With custom settings
# 使用自定义设置
sage run "Fix the bug in main.rs" \
  --provider anthropic \
  --model claude-sonnet-4-20250514 \
  --max-steps 10
```

**Common Use Cases / 常见用例:**

```bash
# Code generation
# 代码生成
sage run "Create a REST API with FastAPI for user management"

# Bug fixing
# 错误修复
sage run "Fix the null pointer exception in auth.py"

# Documentation
# 文档编写
sage run "Write API documentation for the /users endpoint"

# Testing
# 测试
sage run "Create unit tests for the Calculator class"

# Refactoring
# 重构
sage run "Refactor database.py to use async/await"
```

---

### 3. Unified Mode (Advanced) / 统一模式(高级)

**Best for:** Advanced workflows, CI/CD, precise control
**最适合:** 高级工作流、CI/CD、精确控制

```bash
# Interactive unified mode
# 交互统一模式
sage unified "Review and optimize this codebase"

# Non-interactive (autonomous)
# 非交互(自主)
sage unified --non-interactive "Run tests and create report"
```

**Key Features / 关键特性:**
- Inline user input blocking
- More robust execution model
- Better error recovery
- CI/CD friendly

---

## Common Command Options / 常用命令选项

### Override Provider / 覆盖提供商

```bash
# Use specific provider
# 使用特定提供商
sage run "Task" --provider anthropic
sage run "Task" --provider openai
sage run "Task" --provider google
sage run "Task" --provider ollama
```

### Override Model / 覆盖模型

```bash
# Use specific model
# 使用特定模型
sage run "Task" --model "claude-sonnet-4-20250514"
sage run "Task" --model "gpt-4-turbo"
sage run "Task" --model "gemini-2.5-pro"
```

### Set Working Directory / 设置工作目录

```bash
# Execute in specific directory
# 在特定目录执行
sage run "Create tests" --working-dir /path/to/project
```

### Control Execution Steps / 控制执行步骤

```bash
# Limit maximum steps
# 限制最大步骤数
sage run "Complex task" --max-steps 50
```

### Save Trajectory / 保存轨迹

```bash
# Record execution for debugging
# 记录执行用于调试
sage run "Task" --trajectory-file debug.jsonl
```

### Custom Configuration / 自定义配置

```bash
# Use different config file
# 使用不同的配置文件
sage run "Task" --config-file prod_config.json
```

---

## Slash Commands / 斜杠命令

Slash commands provide powerful shortcuts for common operations:
斜杠命令为常见操作提供强大的快捷方式:

### Session Management / 会话管理

```bash
# Resume previous session
# 恢复之前的会话
sage run "/resume"

# Resume specific session
# 恢复特定会话
sage run "/resume abc123-session-id"

# Show all sessions
# 显示所有会话
sage run "/resume --all"

# Clear conversation history
# 清除对话历史
sage run "/clear"
```

### Cost & Usage / 成本和使用情况

```bash
# View session cost
# 查看会话成本
sage run "/cost"

# View context window usage
# 查看上下文窗口使用情况
sage run "/context"
```

### File Operations / 文件操作

```bash
# Undo last file changes
# 撤销最后的文件更改
sage run "/undo"

# Create checkpoint
# 创建检查点
sage run "/checkpoint my-checkpoint"

# Restore checkpoint
# 恢复检查点
sage run "/restore my-checkpoint"
```

### Planning / 规划

```bash
# Create execution plan
# 创建执行计划
sage run "/plan create"

# View current plan
# 查看当前计划
sage run "/plan open"

# Clear plan
# 清除计划
sage run "/plan clear"
```

### Utilities / 工具

```bash
# List all commands
# 列出所有命令
sage run "/commands"

# Show help
# 显示帮助
sage run "/help"

# Show status
# 显示状态
sage run "/status"

# Initialize project
# 初始化项目
sage run "/init"

# View configuration
# 查看配置
sage run "/config"
```

---

## Practical Examples / 实用示例

### Example 1: Create a New Project / 示例 1: 创建新项目

```bash
sage run "Create a Python project with:
- FastAPI web framework
- PostgreSQL database connection
- User authentication
- README and requirements.txt"
```

### Example 2: Debug an Issue / 示例 2: 调试问题

```bash
sage interactive
```

```
Sage> I'm getting a 'Connection refused' error in app.py line 45
[Agent analyzes the issue]

Sage> Show me the database connection code
[Agent displays relevant code]

Sage> Fix the connection string and add retry logic
[Agent applies fixes]
```

### Example 3: Code Review / 示例 3: 代码审查

```bash
sage run "Review all Python files in src/ and:
1. Check for security vulnerabilities
2. Identify performance issues
3. Suggest improvements
4. Generate a review report"
```

### Example 4: Add Tests / 示例 4: 添加测试

```bash
sage run "Create comprehensive unit tests for calculator.py with:
- Test all functions
- Edge cases
- Error handling
- Use pytest framework"
```

### Example 5: Documentation / 示例 5: 文档

```bash
sage run "Generate API documentation for all endpoints in api.py:
- OpenAPI/Swagger format
- Request/response examples
- Error codes
- Authentication info"
```

### Example 6: Refactoring / 示例 6: 重构

```bash
sage interactive
```

```
Sage> Refactor user_service.py to follow SOLID principles
[Agent refactors the code]

Sage> Add type hints to all functions
[Agent adds type annotations]

Sage> Create interface for database operations
[Agent extracts database interface]

Sage> /cost
Total tokens: 5,200
```

---

## Configuration Management / 配置管理

### View Configuration / 查看配置

```bash
# Display current config
# 显示当前配置
sage config show

# Validate config file
# 验证配置文件
sage config validate
```

### Initialize New Config / 初始化新配置

```bash
# Create default config
# 创建默认配置
sage config init

# Force overwrite existing
# 强制覆盖现有配置
sage config init --force

# Custom location
# 自定义位置
sage config init --config-file ~/.config/sage/config.json
```

---

## Trajectory Analysis / 轨迹分析

Trajectories record complete execution history for debugging:
轨迹记录完整的执行历史用于调试:

### List Trajectories / 列出轨迹

```bash
# List all trajectory files
# 列出所有轨迹文件
sage trajectory list

# List in specific directory
# 列出特定目录中的轨迹
sage trajectory list --directory ./trajectories
```

### View Trajectory / 查看轨迹

```bash
# Show trajectory details
# 显示轨迹详情
sage trajectory show trajectory_20250101_120000.jsonl

# Show statistics
# 显示统计信息
sage trajectory stats trajectory_20250101_120000.jsonl
```

### Analyze Performance / 分析性能

```bash
# Analyze execution patterns
# 分析执行模式
sage trajectory analyze ./trajectories
```

---

## Available Tools / 可用工具

View all tools Sage can use:
查看 Sage 可以使用的所有工具:

```bash
sage tools
```

**Tool Categories / 工具类别:**

1. **File Operations / 文件操作**
   - Read, Write, Edit
   - Glob (pattern search)
   - Grep (content search)

2. **Shell Operations / Shell 操作**
   - Bash (execute commands)
   - Background tasks
   - Process management

3. **Task Management / 任务管理**
   - TodoWrite (create task lists)
   - ViewTasklist
   - UpdateTasks

4. **Web / Network / 网络**
   - WebSearch
   - WebFetch
   - Browser

5. **Planning / 规划**
   - EnterPlanMode
   - ExitPlanMode
   - Sequential thinking

---

## Tips & Best Practices / 技巧和最佳实践

### 1. Be Specific / 要具体

**Good / 好:**
```bash
sage run "Create a FastAPI endpoint /users that returns a list of users with pagination"
```

**Bad / 差:**
```bash
sage run "Create an API"
```

### 2. Use Interactive Mode for Exploration / 使用交互模式进行探索

When you're not sure exactly what you need:
当您不确定究竟需要什么时:

```bash
sage interactive
```

### 3. Save Important Sessions / 保存重要会话

```bash
# Use trajectory recording
# 使用轨迹记录
sage run "Complex task" --trajectory-file important_task.jsonl
```

### 4. Control Token Usage / 控制令牌使用

```bash
# Limit steps to control costs
# 限制步骤以控制成本
sage run "Task" --max-steps 10
```

### 5. Use Checkpoints for Long Tasks / 为长任务使用检查点

```
Sage> /checkpoint before-refactor
Sage> Refactor the entire codebase
[If something goes wrong...]
Sage> /restore before-refactor
```

### 6. Leverage Slash Commands / 利用斜杠命令

```
Sage> /resume    # Continue yesterday's work
Sage> /cost      # Check spending
Sage> /plan      # Review execution plan
```

### 7. Use Git Integration / 使用 Git 集成

```
Sage> /undo      # Revert file changes
```

### 8. Choose the Right Provider / 选择合适的提供商

- **Anthropic Claude**: Best for complex reasoning
- **OpenAI GPT**: Great for general tasks
- **Google Gemini**: Excellent with large context
- **Ollama**: Free local execution

---

## Troubleshooting / 故障排除

### Issue: Agent Gets Stuck / 问题: Agent 卡住

**Solution / 解决方案:**
```bash
# Reduce max steps
# 减少最大步骤数
sage run "Task" --max-steps 5

# Or use Ctrl+C to interrupt
# 或使用 Ctrl+C 中断
```

### Issue: High Costs / 问题: 成本高

**Solution / 解决方案:**
```bash
# Check costs regularly
# 定期检查成本
/cost

# Use cheaper models
# 使用更便宜的模型
sage run "Task" --model "gpt-3.5-turbo"

# Switch to local models
# 切换到本地模型
sage run "Task" --provider ollama
```

### Issue: Wrong Working Directory / 问题: 工作目录错误

**Solution / 解决方案:**
```bash
# Always specify working directory
# 始终指定工作目录
sage run "Task" --working-dir /correct/path

# Or set in config file
# 或在配置文件中设置
{
  "working_directory": "/path/to/project"
}
```

### Issue: API Rate Limits / 问题: API 速率限制

**Solution / 解决方案:**
```bash
# Wait a moment and retry
# 等待片刻后重试

# Or switch providers
# 或切换提供商
sage run "Task" --provider google
```

---

## Next Steps / 下一步

Now that you know the basics:
现在您已了解基础知识:

1. **Explore Examples / 探索示例**
   ```bash
   cd /path/to/sage
   make examples
   ```

2. **Read Advanced Guides / 阅读高级指南**
   - Custom tools development
   - SDK integration
   - Advanced configuration

3. **Join the Community / 加入社区**
   - GitHub: https://github.com/majiayu000/sage
   - Report issues
   - Contribute

4. **Try Real Projects / 尝试真实项目**
   ```bash
   sage run "Help me build a web application"
   ```

---

## Quick Reference / 快速参考

### Essential Commands / 基本命令

```bash
# Run a task
sage run "your task here"

# Interactive mode
sage interactive

# Resume previous session
sage run "/resume"

# Check costs
sage run "/cost"

# View configuration
sage config show

# List available tools
sage tools

# Get help
sage --help
sage run "/help"
```

### Common Flags / 常用标志

```bash
--provider <name>         # LLM provider
--model <name>            # Model name
--max-steps <n>           # Maximum steps
--working-dir <path>      # Working directory
--config-file <path>      # Config file
--trajectory-file <path>  # Trajectory output
--verbose                 # Verbose output
--non-interactive         # Non-interactive mode
```

---

## Additional Resources / 其他资源

- **Installation Guide**: [installation.md](installation.md)
- **Configuration Guide**: [configuration.md](configuration.md)
- **GitHub**: https://github.com/majiayu000/sage
- **Examples**: `/examples` directory
- **Documentation**: `/docs` directory

---

**Happy Coding with Sage! / 使用 Sage 愉快编码!** 🦀✨
