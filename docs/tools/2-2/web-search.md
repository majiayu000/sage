# WebSearch Tool

## 基本信息

| 属性 | 值 |
|------|-----|
| 工具名称 | WebSearch |
| Claude Code 版本 | 2.1.8 |
| 类别 | 网络工具 |
| Sage 实现 | `sage-tools/src/tools/network/web_search/` |

## 功能描述

网络搜索工具，提供最新信息以补充 Claude 的知识截止日期。

## 完整 Prompt

```markdown
- Allows Claude to search the web and use the results to inform responses
- Provides up-to-date information for current events and recent data
- Returns search result information formatted as search result blocks, including links as markdown hyperlinks
- Use this tool for accessing information beyond Claude's knowledge cutoff
- Searches are performed automatically within a single API call

CRITICAL REQUIREMENT - You MUST follow this:
  - After answering the user's question, you MUST include a "Sources:" section at the end of your response
  - In the Sources section, list all relevant URLs from the search results as markdown hyperlinks: [Title](URL)
  - This is MANDATORY - never skip including sources in your response
  - Example format:

    [Your answer here]

    Sources:
    - [Source Title 1](https://example.com/1)
    - [Source Title 2](https://example.com/2)

Usage notes:
  - Domain filtering is supported to include or block specific websites
  - Web search is only available in the US

IMPORTANT - Use the correct year in search queries:
  - Today's date is ${GET_CURRENT_DATE_FN()}. You MUST use this year when searching for recent information, documentation, or current events.
  - Example: If the user asks for "latest React docs", search for "React documentation ${CURRENT_YEAR}", NOT "React documentation ${CURRENT_YEAR-1}"
```

## 参数

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| query | string | ✅ | 搜索查询 |
| domains | array | ❌ | 域名过滤 |

## 设计原理

### 1. 强制引用来源
**为什么**:
- 透明度和可验证性
- 用户可以深入了解
- 避免信息来源不明

### 2. 使用当前年份
**为什么**:
- 搜索最新信息
- 避免过时结果
- 动态注入当前日期

### 3. 域名过滤
**为什么**:
- 限制或排除特定网站
- 提高结果质量
- 满足特定需求

### 4. Markdown 超链接
**为什么**:
- 可点击的链接
- 统一的格式
- 便于用户访问

## 使用场景

### ✅ 应该使用
- 查询最新信息
- 当前事件
- 最新文档版本
- 超出知识截止日期的内容

### ❌ 不应该使用
- 已知的稳定信息
- 代码库内部搜索
- 本地文件搜索

## 响应格式

```markdown
[回答内容]

根据搜索结果，React 18 的主要新特性包括...

Sources:
- [React 18 Release Notes](https://react.dev/blog/2022/03/29/react-v18)
- [What's New in React 18](https://example.com/react-18-features)
```

## 变量说明

| 变量 | 说明 |
|------|------|
| GET_CURRENT_DATE_FN | 返回当前日期的函数 |
| CURRENT_YEAR | 当前年份 |

## 限制

- 仅在美国可用
- 结果数量有限
- 某些网站可能被过滤

## Sage 实现差异

Sage 的 WebSearchTool 支持：
- 多搜索引擎后端
- 自定义结果数量
- 搜索历史记录
- 结果缓存
