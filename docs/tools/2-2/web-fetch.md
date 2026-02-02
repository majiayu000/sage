# WebFetch Tool

## 基本信息

| 属性 | 值 |
|------|-----|
| 工具名称 | WebFetch |
| Claude Code 版本 | 2.1.14 |
| 类别 | 网络工具 |
| Sage 实现 | `sage-tools/src/tools/network/web_fetch/` |

## 功能描述

获取 URL 内容并使用 AI 模型处理，将 HTML 转换为 Markdown 格式。

## 完整 Prompt

```markdown
- Fetches content from a specified URL and processes it using an AI model
- Takes a URL and a prompt as input
- Fetches the URL content, converts HTML to markdown
- Processes the content with the prompt using a small, fast model
- Returns the model's response about the content
- Use this tool when you need to retrieve and analyze web content

Usage notes:
  - IMPORTANT: If an MCP-provided web fetch tool is available, prefer using that tool instead of this one, as it may have fewer restrictions.
  - The URL must be a fully-formed valid URL
  - HTTP URLs will be automatically upgraded to HTTPS
  - The prompt should describe what information you want to extract from the page
  - This tool is read-only and does not modify any files
  - Results may be summarized if the content is very large
  - Includes a self-cleaning 15-minute cache for faster responses when repeatedly accessing the same URL
  - When a URL redirects to a different host, the tool will inform you and provide the redirect URL in a special format. You should then make a new WebFetch request with the redirect URL to fetch the content.
  - For GitHub URLs, prefer using the gh CLI via Bash instead (e.g., gh pr view, gh issue view, gh api).
```

## 参数

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| url | string | ✅ | 要获取的 URL |
| prompt | string | ✅ | 处理内容的提示 |

## 设计原理

### 1. AI 处理内容
**为什么**:
- 网页内容通常很长
- 提取关键信息
- 返回结构化结果

### 2. HTML 转 Markdown
**为什么**:
- Markdown 更易读
- 去除无关的样式和脚本
- 保留语义结构

### 3. 自动 HTTPS 升级
**为什么**:
- 安全性考虑
- 大多数网站支持 HTTPS
- 避免混合内容问题

### 4. 15 分钟缓存
**为什么**:
- 减少重复请求
- 提高响应速度
- 自动清理过期缓存

### 5. 重定向处理
**为什么**:
- 跨域重定向需要显式处理
- 安全考虑
- 用户知情

### 6. GitHub 使用 gh CLI
**为什么**:
- gh CLI 有更好的 API 访问
- 支持认证
- 结构化输出

## 使用场景

### ✅ 应该使用
- 获取文档内容
- 分析网页信息
- 提取特定数据

### ❌ 不应该使用
- GitHub 内容 (使用 gh CLI)
- 需要认证的页面
- 有 MCP 工具可用时

## 示例

```json
{
  "url": "https://docs.rust-lang.org/book/ch01-01-installation.html",
  "prompt": "Extract the installation steps for Rust on macOS"
}
```

## 限制

- 不支持需要认证的页面
- 不支持 JavaScript 渲染的内容
- 大内容会被摘要
- 某些网站可能阻止访问

## Sage 实现差异

Sage 的 WebFetchTool 额外支持：
- 自定义 User-Agent
- 代理配置
- 更灵活的缓存策略
- 内容大小限制配置
