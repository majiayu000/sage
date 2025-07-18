# Sage Agent 对话模式使用指南

## 概述

Sage Agent 现在支持真正的对话模式！您可以像与人聊天一样与 AI 进行连续对话，AI 会记住之前的对话内容和上下文。

## 启动 Sage Agent

```bash
cargo run --bin sage
# 或者如果已经构建
./target/debug/sage
```

## 基本使用

### 1. 查看帮助
```
sage: help
```

### 2. 开始对话
直接输入您的问题或任务，无需特殊命令：

```
sage: Create a Python function to calculate fibonacci numbers
```

### 3. 继续对话
AI 回复后，您可以继续提问或要求修改：

```
sage: Add memoization to make it more efficient
sage: Now write unit tests for this function
sage: Can you explain how the memoization works?
```

### 4. 查看对话状态
```
sage: conversation
# 或
sage: conv
```

### 5. 开始新对话
如果要切换到完全不同的话题：

```
sage: new
# 或
sage: new-task
```

## 实际使用示例

### 示例 1：Python 开发
```
sage: Create a Python class for managing a todo list

AI: [创建 TodoList 类]

sage: Add a method to mark items as completed

AI: [添加 mark_completed 方法]

sage: Now add persistence to save/load from JSON file

AI: [添加 JSON 持久化功能]

sage: Write comprehensive unit tests

AI: [编写完整的单元测试]
```

### 示例 2：Web 开发
```
sage: Help me set up a React project with TypeScript

AI: [设置 React + TypeScript 项目]

sage: Add a simple component for displaying user profiles

AI: [创建 UserProfile 组件]

sage: Add styling with CSS modules

AI: [添加 CSS 模块样式]

sage: new

sage: Now help me with a completely different task - setting up a Python Flask API

AI: [开始新的 Flask API 任务]
```

## 可用命令

### 系统命令
- `help` / `h` - 显示帮助信息
- `config` - 显示当前配置
- `status` - 显示系统状态
- `exit` / `quit` / `q` - 退出程序

### 显示控制
- `clear` / `cls` - 清屏
- `reset` / `refresh` - 重置终端显示
- `input-help` / `ih` - 输入问题帮助

### 对话控制
- `new` / `new-task` - 开始新对话
- `conversation` / `conv` - 显示对话摘要

## 对话模式的优势

### 1. 上下文保持
```
sage: Create a Python function to read CSV files
AI: [创建 read_csv 函数]

sage: Add error handling for file not found
AI: [在同一个函数中添加错误处理，而不是创建新函数]
```

### 2. 迭代改进
```
sage: Write a sorting algorithm
AI: [实现冒泡排序]

sage: That's too slow, use a faster algorithm
AI: [改为快速排序实现]

sage: Add comments to explain the algorithm
AI: [在现有代码中添加详细注释]
```

### 3. 相关任务链接
```
sage: Create a database schema for a blog
AI: [设计数据库表结构]

sage: Now write the SQL migration scripts
AI: [基于之前的 schema 创建迁移脚本]

sage: Generate the corresponding Python SQLAlchemy models
AI: [基于相同的 schema 创建 ORM 模型]
```

## 最佳实践

### 1. 明确的指令
- ✅ "Add error handling to the function you just created"
- ❌ "Add error handling" (不清楚要添加到哪里)

### 2. 逐步构建
- 先创建基本功能
- 然后逐步添加特性
- 最后完善错误处理和测试

### 3. 适时重置
- 当切换到完全不同的项目时使用 `new`
- 当对话变得太长或混乱时重新开始

### 4. 利用上下文
- 引用之前创建的代码："修改刚才的函数"
- 建立在之前的工作基础上："基于这个设计..."

## 注意事项

1. **内存限制**：非常长的对话可能会影响性能，适时使用 `new` 重置
2. **上下文相关性**：确保您的请求与当前对话上下文相关
3. **错误恢复**：如果出现错误，对话状态会保持，您可以重试或调整请求
4. **轨迹记录**：每次对话都会生成轨迹文件用于调试和分析

## 故障排除

### 对话状态混乱
```
sage: new
sage: [重新开始您的任务]
```

### 输入显示问题
```
sage: reset
```

### 查看当前状态
```
sage: conversation
sage: status
```

## 技术细节

- 对话历史存储在内存中的 `Vec<LLMMessage>`
- 每次 AI 回复都会自动添加到对话历史
- `new` 命令会清除所有对话状态
- 轨迹文件记录完整的执行过程

享受与 Sage Agent 的对话吧！🚀
