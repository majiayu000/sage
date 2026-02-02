# ToolSearch Tool

## 基本信息

| 属性 | 值 |
|------|-----|
| 工具名称 | ToolSearch |
| Claude Code 版本 | 2.1.19 |
| 类别 | 扩展工具 |
| Sage 实现 | 未实现 |

## 功能描述

搜索或选择延迟加载的工具，使其可用于调用。这是调用延迟工具的强制前提。

## 完整 Prompt

```markdown
Search for or select deferred tools to make them available for use.

**MANDATORY PREREQUISITE - THIS IS A HARD REQUIREMENT**

You MUST use this tool to load deferred tools BEFORE calling them directly.

This is a BLOCKING REQUIREMENT - deferred tools listed below are NOT available until you load them using this tool. Both query modes (keyword search and direct selection) load the returned tools — once a tool appears in the results, it is immediately available to call.

**Why this is non-negotiable:**
- Deferred tools are not loaded until discovered via this tool
- Calling a deferred tool without first loading it will fail

**Query modes:**

1. **Keyword search** - Use keywords when you're unsure which tool to use or need to discover multiple tools at once:
   - "list directory" - find tools for listing directories
   - "notebook jupyter" - find notebook editing tools
   - "slack message" - find slack messaging tools
   - Returns up to 5 matching tools ranked by relevance
   - All returned tools are immediately available to call — no further selection step needed

2. **Direct selection** - Use `select:<tool_name>` when you know the exact tool name and only need that one tool:
   - "select:mcp__slack__read_channel"
   - "select:NotebookEdit"
   - Returns just that tool if it exists

**IMPORTANT:** Both modes load tools equally. Do NOT follow up a keyword search with `select:` calls for tools already returned — they are already loaded.

3. **Required keyword** - Prefix with `+` to require a match:
   - "+linear create issue" - only tools from "linear", ranked by "create"/"issue"
   - "+slack send" - only "slack" tools, ranked by "send"
   - Useful when you know the service name but not the exact tool
```

## 参数

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| query | string | ✅ | 搜索查询或 `select:<tool_name>` |

## 设计原理

### 1. 延迟加载机制
**为什么**:
- 减少初始加载时间
- 节省内存资源
- 按需加载工具

### 2. 强制前提要求
**为什么**:
- 确保工具在调用前已加载
- 避免运行时错误
- 明确的工具发现流程

### 3. 两种查询模式
**为什么**:
- 关键词搜索：不确定具体工具时
- 直接选择：知道确切工具名时
- 灵活适应不同场景

### 4. 必需关键词 (+)
**为什么**:
- 限定搜索范围
- 知道服务名但不知道具体工具
- 提高搜索精度

## 使用场景

### ✅ 正确用法

```markdown
# 关键词搜索
用户: 我需要使用 slack
Agent: [调用 ToolSearch query: "slack"]
Agent: 找到 mcp__slack__read_channel
Agent: [直接调用 mcp__slack__read_channel - 已加载]

# 直接选择
用户: 编辑 Jupyter notebook
Agent: [调用 ToolSearch query: "select:NotebookEdit"]
Agent: [调用 NotebookEdit]
```

### ❌ 错误用法

```markdown
# 错误：未先加载就调用
Agent: [直接调用 mcp__slack__read_channel]
结果: 失败 - 工具未加载

# 错误：重复加载
Agent: [调用 ToolSearch query: "slack"] → 返回 mcp__slack__read_channel
Agent: [调用 ToolSearch query: "select:mcp__slack__read_channel"]
结果: 冗余 - 关键词搜索已加载该工具
```

## Sage 实现状态

Sage 目前未实现 ToolSearch，所有工具在启动时即加载。未来可能实现：
- MCP 工具的延迟加载
- 工具发现机制
- 动态工具注册
