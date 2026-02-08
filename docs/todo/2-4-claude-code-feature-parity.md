# Claude Code 功能对齐计划

> 日期: 2026-02-04
> 目标: 实现 Claude Code v2.1.19 的所有缺失功能
> 状态: 规划中

## 概述

本文档详细列出了 Sage 相比 Claude Code v2.1.19 缺失的所有功能，并提供实现计划。

---

## 一、CLI 子命令系统 [高优先级]

### 1.1 MCP 子命令 (`sage mcp`)

Claude Code 命令:
```bash
claude mcp add <name> <commandOrUrl> [args...]    # 添加 MCP 服务器
claude mcp add-json <name> <json>                  # 通过 JSON 添加
claude mcp add-from-claude-desktop                 # 从 Claude Desktop 导入
claude mcp remove <name>                           # 删除服务器
claude mcp list                                    # 列出所有服务器
claude mcp get <name>                              # 获取服务器详情
claude mcp serve                                   # 启动 MCP 服务器
claude mcp reset-project-choices                   # 重置项目选择
```

**实现任务:**
- [ ] 创建 `sage mcp` 子命令框架
- [ ] 实现 `sage mcp add` - 支持 stdio/HTTP/SSE 传输
- [ ] 实现 `sage mcp add-json` - JSON 配置添加
- [ ] 实现 `sage mcp remove` - 删除服务器
- [ ] 实现 `sage mcp list` - 列出服务器
- [ ] 实现 `sage mcp get` - 获取详情
- [ ] 实现 `sage mcp serve` - 作为 MCP 服务器运行
- [ ] 实现 `sage mcp reset-project-choices` - 重置选择
- [ ] 支持项目级配置 (`.mcp.json`)
- [ ] 支持环境变量 (`-e API_KEY=xxx`)
- [ ] 支持 HTTP headers (`--header`)

**文件位置:**
- `crates/sage-cli/src/commands/mcp/mod.rs`
- `crates/sage-cli/src/commands/mcp/add.rs`
- `crates/sage-cli/src/commands/mcp/remove.rs`
- `crates/sage-cli/src/commands/mcp/list.rs`
- `crates/sage-cli/src/commands/mcp/serve.rs`

### 1.2 Plugin 子命令 (`sage plugin`)

Claude Code 命令:
```bash
claude plugin install <plugin>      # 安装插件
claude plugin uninstall <plugin>    # 卸载插件
claude plugin enable <plugin>       # 启用插件
claude plugin disable <plugin>      # 禁用插件
claude plugin list                  # 列出插件
claude plugin update <plugin>       # 更新插件
claude plugin validate <path>       # 验证插件
claude plugin marketplace           # 市场管理
```

**实现任务:**
- [ ] 设计插件系统架构
- [ ] 创建 `sage plugin` 子命令框架
- [ ] 实现插件清单格式 (manifest.json)
- [ ] 实现 `sage plugin install` - 从市场安装
- [ ] 实现 `sage plugin uninstall` - 卸载插件
- [ ] 实现 `sage plugin enable/disable` - 启用/禁用
- [ ] 实现 `sage plugin list` - 列出已安装
- [ ] 实现 `sage plugin update` - 更新插件
- [ ] 实现 `sage plugin validate` - 验证清单
- [ ] 实现插件市场子命令
  - [ ] `sage plugin marketplace add`
  - [ ] `sage plugin marketplace remove`
  - [ ] `sage plugin marketplace list`

**文件位置:**
- `crates/sage-core/src/plugins/mod.rs`
- `crates/sage-core/src/plugins/manifest.rs`
- `crates/sage-core/src/plugins/marketplace.rs`
- `crates/sage-cli/src/commands/plugin/mod.rs`

### 1.3 其他子命令

```bash
sage doctor                    # 健康检查 (已有 /doctor，需要 CLI 版本)
sage install [target]          # 安装管理
sage update                    # 更新检查
sage setup-token               # 长期令牌设置
```

**实现任务:**
- [ ] 实现 `sage doctor` CLI 子命令
- [ ] 实现 `sage install` - 安装管理
- [ ] 实现 `sage update` - 更新检查
- [ ] 实现 `sage setup-token` - 令牌设置

---

## 二、高级 CLI 选项 [高优先级]

### 2.1 Agent 相关选项

```bash
--agent <agent>                # 指定 Agent
--agents <json>                # 自定义 Agent 定义
```

**实现任务:**
- [ ] 添加 `--agent` 选项到 CLI
- [ ] 添加 `--agents` 选项支持 JSON 定义
- [ ] 实现 Agent 选择逻辑
- [ ] 实现自定义 Agent 解析

### 2.2 工具过滤选项

```bash
--allowedTools <tools...>      # 允许的工具列表
--disallowedTools <tools...>   # 禁止的工具列表
--tools <tools...>             # 指定可用工具
```

**实现任务:**
- [ ] 添加 `--allowed-tools` 选项
- [ ] 添加 `--disallowed-tools` 选项
- [ ] 添加 `--tools` 选项
- [ ] 实现工具过滤逻辑

### 2.3 MCP 配置选项

```bash
--mcp-config <configs...>      # 加载 MCP 配置
--strict-mcp-config            # 严格 MCP 配置模式
--mcp-debug                    # MCP 调试模式
```

**实现任务:**
- [ ] 添加 `--mcp-config` 选项
- [ ] 添加 `--strict-mcp-config` 选项
- [ ] 实现 MCP 配置加载逻辑

### 2.4 会话管理选项

```bash
--fork-session                 # 会话分叉
--session-id <uuid>            # 指定会话 ID
--no-session-persistence       # 禁用会话持久化
```

**实现任务:**
- [ ] 添加 `--fork-session` 选项
- [ ] 添加 `--session-id` 选项
- [ ] 添加 `--no-session-persistence` 选项
- [ ] 实现会话分叉逻辑

### 2.5 输出格式选项

```bash
--output-format <format>       # 输出格式 (text/json/stream-json)
--input-format <format>        # 输入格式
--json-schema <schema>         # 结构化输出验证
--include-partial-messages     # 包含部分消息
--replay-user-messages         # 重放用户消息
```

**实现任务:**
- [ ] 添加 `--output-format` 选项
- [ ] 添加 `--input-format` 选项
- [ ] 添加 `--json-schema` 选项
- [ ] 实现 JSON Schema 验证
- [ ] 实现流式 JSON 输入/输出

### 2.6 权限和安全选项

```bash
--permission-mode <mode>                    # 权限模式
--dangerously-skip-permissions              # 跳过权限检查
--allow-dangerously-skip-permissions        # 允许跳过权限
--disable-slash-commands                    # 禁用斜杠命令
```

**实现任务:**
- [ ] 添加 `--permission-mode` 选项
- [ ] 添加 `--dangerously-skip-permissions` 选项
- [ ] 实现权限模式逻辑

### 2.7 模型和预算选项

```bash
--fallback-model <model>       # 降级模型
--max-budget-usd <amount>      # 最大预算
--betas <betas...>             # Beta 功能
```

**实现任务:**
- [ ] 添加 `--fallback-model` 选项
- [ ] 添加 `--max-budget-usd` 选项
- [ ] 实现模型降级逻辑
- [ ] 实现预算控制

### 2.8 集成选项

```bash
--chrome                       # Chrome 集成
--no-chrome                    # 禁用 Chrome
--ide                          # IDE 自动连接
--plugin-dir <paths...>        # 插件目录
--add-dir <directories...>     # 添加目录 (已实现)
```

**实现任务:**
- [ ] 添加 `--chrome` 选项
- [ ] 添加 `--ide` 选项
- [ ] 添加 `--plugin-dir` 选项
- [ ] 实现 Chrome 集成
- [ ] 实现 IDE 自动检测和连接

### 2.9 配置选项

```bash
--settings <file-or-json>      # 加载设置
--setting-sources <sources>    # 设置来源
--system-prompt <prompt>       # 系统提示
--append-system-prompt <prompt> # 追加系统提示
```

**实现任务:**
- [ ] 添加 `--settings` 选项
- [ ] 添加 `--setting-sources` 选项
- [ ] 添加 `--system-prompt` 选项
- [ ] 添加 `--append-system-prompt` 选项

---

## 三、Git 集成 [中优先级]

### 3.1 `/commit` 命令

**功能:**
- AI 辅助生成提交消息
- 分析 staged changes
- 遵循项目提交规范
- 支持 conventional commits

**实现任务:**
- [ ] 创建 `/commit` 斜杠命令
- [ ] 实现 git diff 分析
- [ ] 实现提交消息生成
- [ ] 支持 conventional commits 格式
- [ ] 支持自定义提交模板

**文件位置:**
- `crates/sage-core/src/commands/executor/handlers/git.rs`

### 3.2 `/review-pr` 命令

**功能:**
- 自动化 PR 审查
- 代码质量检查
- 安全漏洞检测
- 改进建议

**实现任务:**
- [ ] 创建 `/review-pr` 斜杠命令
- [ ] 实现 PR diff 获取
- [ ] 实现代码审查逻辑
- [ ] 生成审查报告

---

## 四、认证系统增强 [中优先级]

### 4.1 OAuth 2.0 + PKCE

**功能:**
- 完整的 OAuth 2.0 授权码流程
- PKCE (Proof Key for Code Exchange) 支持
- 动态客户端注册 (RFC 7591)
- 令牌刷新机制

**实现任务:**
- [ ] 实现 OAuth 2.0 授权码流程
- [ ] 实现 PKCE 参数生成和验证
- [ ] 实现令牌刷新
- [ ] 实现动态客户端注册

**文件位置:**
- `crates/sage-core/src/auth/oauth.rs`
- `crates/sage-core/src/auth/pkce.rs`
- `crates/sage-core/src/auth/token.rs`

### 4.2 长期令牌 (`setup-token`)

**功能:**
- 设置长期认证令牌
- 安全存储
- 令牌验证

**实现任务:**
- [ ] 实现 `sage setup-token` 命令
- [ ] 实现令牌安全存储
- [ ] 实现令牌验证流程

---

## 五、IDE 集成 [中优先级]

### 5.1 JetBrains IDE 支持

**支持的 IDE:**
- PyCharm
- IntelliJ IDEA
- WebStorm
- PhpStorm
- RubyMine
- CLion
- GoLand
- Rider
- DataGrip
- AppCode
- DataSpell
- Aqua
- Gateway
- Fleet
- Android Studio

**实现任务:**
- [ ] 实现 JetBrains IDE 检测
- [ ] 实现插件安装系统
- [ ] 实现 IDE 通信协议
- [ ] 支持 WSL 路径转换

### 5.2 VS Code 集成

**实现任务:**
- [ ] 实现 VS Code 检测
- [ ] 实现扩展通信
- [ ] 实现工作区同步

### 5.3 IDE 自动连接

**实现任务:**
- [ ] 实现 `--ide` 选项
- [ ] 实现 IDE 自动检测
- [ ] 实现自动连接逻辑

---

## 六、插件系统 [高优先级]

### 6.1 插件架构

**组件:**
- 插件清单 (manifest.json)
- 插件加载器
- 插件沙箱
- 插件 API

**实现任务:**
- [ ] 设计插件清单格式
- [ ] 实现插件加载器
- [ ] 实现插件沙箱隔离
- [ ] 定义插件 API

### 6.2 插件市场

**功能:**
- 多市场支持
- 插件搜索
- 版本管理
- 自动更新

**实现任务:**
- [ ] 实现市场协议
- [ ] 实现插件下载
- [ ] 实现版本检查
- [ ] 实现自动更新

---

## 七、自动更新系统 [低优先级]

### 7.1 更新检查

**实现任务:**
- [ ] 实现版本检查 API
- [ ] 实现更新通知
- [ ] 实现后台更新检查

### 7.2 安装管理

**支持的安装方式:**
- 原生二进制 (native)
- npm 全局 (npm-global)
- npm 本地 (npm-local)
- 包管理器 (homebrew 等)

**实现任务:**
- [ ] 实现安装方式检测
- [ ] 实现原生安装
- [ ] 实现版本锁定
- [ ] 实现清理旧版本

---

## 八、遥测和监控 [低优先级]

### 8.1 使用数据追踪

**实现任务:**
- [ ] 实现使用数据收集
- [ ] 实现数据上报
- [ ] 实现隐私控制

### 8.2 错误报告

**实现任务:**
- [ ] 实现错误捕获
- [ ] 实现错误上报
- [ ] 实现 Sentry 集成 (可选)

---

## 九、其他缺失功能

### 9.1 Chrome 集成

**实现任务:**
- [ ] 实现 Chrome Native Host
- [ ] 实现浏览器通信
- [ ] 实现网页交互

### 9.2 文件资源下载

```bash
--file <specs...>    # 启动时下载文件资源
```

**实现任务:**
- [ ] 添加 `--file` 选项
- [ ] 实现文件下载逻辑

### 9.3 调试增强

```bash
-d, --debug [filter]    # 带过滤的调试模式
```

**实现任务:**
- [ ] 增强 `--debug` 选项
- [ ] 实现分类过滤 (api, hooks, mcp 等)
- [ ] 实现排除过滤 (!statsig, !file 等)

---

## 实现优先级

### P0 - 必须实现 (本周)
1. MCP CLI 子命令 (`sage mcp`)
2. 高级 CLI 选项 (工具过滤、会话管理)
3. `/commit` 命令

### P1 - 重要 (下周)
4. 插件系统基础
5. OAuth 2.0 + PKCE
6. IDE 集成基础

### P2 - 一般 (本月)
7. 插件市场
8. `/review-pr` 命令
9. 自动更新系统

### P3 - 可选 (后续)
10. Chrome 集成
11. 遥测系统
12. 完整 IDE 支持

---

## 文件结构规划

```
crates/
├── sage-cli/
│   └── src/
│       └── commands/
│           ├── mcp/
│           │   ├── mod.rs
│           │   ├── add.rs
│           │   ├── remove.rs
│           │   ├── list.rs
│           │   ├── serve.rs
│           │   └── config.rs
│           ├── plugin/
│           │   ├── mod.rs
│           │   ├── install.rs
│           │   ├── uninstall.rs
│           │   ├── list.rs
│           │   └── marketplace.rs
│           ├── doctor.rs
│           ├── install.rs
│           ├── update.rs
│           └── setup_token.rs
├── sage-core/
│   └── src/
│       ├── auth/
│       │   ├── mod.rs
│       │   ├── oauth.rs
│       │   ├── pkce.rs
│       │   └── token.rs
│       ├── plugins/
│       │   ├── mod.rs
│       │   ├── manifest.rs
│       │   ├── loader.rs
│       │   ├── sandbox.rs
│       │   └── marketplace.rs
│       ├── ide/
│       │   ├── mod.rs
│       │   ├── jetbrains.rs
│       │   ├── vscode.rs
│       │   └── detection.rs
│       └── commands/
│           └── executor/
│               └── handlers/
│                   └── git.rs
└── sage-tools/
    └── src/
        └── tools/
            └── git/
                ├── mod.rs
                ├── commit.rs
                └── review.rs
```

---

## 参考资源

- Claude Code v2.1.19 CLI: `/opt/homebrew/Caskroom/claude-code/2.1.19/claude`
- 分析文档: `/Users/apple/Desktop/code/AI/code-agent/open-claude-code/`
- MCP 协议: https://modelcontextprotocol.io/
- OAuth 2.0: RFC 6749
- PKCE: RFC 7636

---

## 更新日志

| 日期 | 更新内容 |
|------|----------|
| 2026-02-04 | 初始文档创建 |
