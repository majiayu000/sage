# LSP Tool

## 基本信息

| 属性 | 值 |
|------|-----|
| 工具名称 | LSP |
| Claude Code 版本 | 2.0.73 |
| 类别 | 代码智能 |
| Sage 实现 | 未实现 (有 DiagnosticsTool 部分替代) |

## 功能描述

与 Language Server Protocol (LSP) 服务器交互，获取代码智能功能。

## 完整 Prompt

```markdown
Interact with Language Server Protocol (LSP) servers to get code intelligence features.

Supported operations:
- goToDefinition: Find where a symbol is defined
- findReferences: Find all references to a symbol
- hover: Get hover information (documentation, type info) for a symbol
- documentSymbol: Get all symbols (functions, classes, variables) in a document
- workspaceSymbol: Search for symbols across the entire workspace
- goToImplementation: Find implementations of an interface or abstract method
- prepareCallHierarchy: Get call hierarchy item at a position (functions/methods)
- incomingCalls: Find all functions/methods that call the function at a position
- outgoingCalls: Find all functions/methods called by the function at a position

All operations require:
- filePath: The file to operate on
- line: The line number (1-based, as shown in editors)
- character: The character offset (1-based, as shown in editors)

Note: LSP servers must be configured for the file type. If no server is available, an error will be returned.
```

## 参数

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| operation | string | ✅ | LSP 操作类型 |
| filePath | string | ✅ | 文件路径 |
| line | number | ✅ | 行号 (1-based) |
| character | number | ✅ | 字符偏移 (1-based) |
| query | string | ❌ | 搜索查询 (workspaceSymbol) |

## 支持的操作

| 操作 | 说明 |
|------|------|
| goToDefinition | 跳转到符号定义 |
| findReferences | 查找所有引用 |
| hover | 获取悬停信息 (文档、类型) |
| documentSymbol | 获取文档中所有符号 |
| workspaceSymbol | 在工作区搜索符号 |
| goToImplementation | 查找接口/抽象方法的实现 |
| prepareCallHierarchy | 获取调用层次项 |
| incomingCalls | 查找调用当前函数的所有函数 |
| outgoingCalls | 查找当前函数调用的所有函数 |

## 设计原理

### 1. 基于位置的操作
**为什么**:
- LSP 协议基于位置
- 精确定位符号
- 与编辑器行为一致

### 2. 1-based 行号
**为什么**:
- 与编辑器显示一致
- 用户更直观
- 避免 off-by-one 错误

### 3. 多种操作类型
**为什么**:
- 覆盖常见代码导航需求
- 支持代码理解
- 提高开发效率

### 4. 需要 LSP 服务器
**为什么**:
- 依赖语言特定的分析
- 提供准确的语义信息
- 支持多种语言

## 使用场景

### ✅ 应该使用
- 查找函数定义
- 查找变量引用
- 理解代码结构
- 分析调用关系

### ❌ 不应该使用
- 简单的文本搜索 (使用 Grep)
- 文件查找 (使用 Glob)
- 没有 LSP 服务器的语言

## 示例

```json
// 跳转到定义
{
  "operation": "goToDefinition",
  "filePath": "/src/main.rs",
  "line": 42,
  "character": 15
}

// 查找引用
{
  "operation": "findReferences",
  "filePath": "/src/utils.rs",
  "line": 10,
  "character": 8
}

// 工作区符号搜索
{
  "operation": "workspaceSymbol",
  "filePath": "/src/main.rs",
  "line": 1,
  "character": 1,
  "query": "UserService"
}
```

## Sage 实现状态

Sage 目前未实现完整的 LSP 工具，但有：
- `DiagnosticsTool`: 提供部分 IDE 诊断功能
- 未来计划集成 LSP 支持
