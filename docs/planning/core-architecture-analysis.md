# Sage Core 架构分析与最佳方案

## 三项目模块对比

### 1. 模块分类对比

| 分类 | Sage-core (Rust) | OpenClaude (JS) | Crush (Go) | 最佳实践 |
|-----|-----------------|-----------------|-----------|---------|
| **核心执行** | agent (unified executor) | agent, loop | agent (coder.md.tpl) | Crush 的模板化 prompt 更灵活 |
| **LLM 抽象** | llm (providers, parsers, fallback) | api, ai | agent/hyper | Sage 抽象最完善 |
| **工具系统** | tools (executor, permission, cache) | tool, tools | agent/tools | Crush 工具+描述 md 模式优雅 |
| **技能系统** | skills | skills | skills | 三者都采用 SKILL.md 标准 |
| **MCP 协议** | mcp | mcp | agent/tools/mcp | 都支持 MCP |
| **配置** | config, settings | config, settings | config | Sage 分离最清晰 |
| **会话** | session, context | session, transcript | session, history | Sage 最完整 |
| **权限** | tools/permission | permission | permission | Crush 独立模块更好 |
| **UI/TUI** | ui | ui | tui (components) | Crush 组件化最好 |
| **遥测** | telemetry, cost | telemetry, metrics | (无独立模块) | Sage 最完善 |
| **恢复** | recovery (circuit_breaker, rate_limiter) | retry | (无) | Sage 独有优势 |
| **学习** | learning (patterns, engine) | (无) | (无) | Sage 独有特性 |
| **检查点** | checkpoints | (无) | (无) | Sage 独有特性 |
| **存储** | storage (postgres, sqlite) | storage | db (sqlite) | Sage 最完善 |
| **钩子** | hooks | hooks | (集成在 config) | Sage/OpenClaude 独立更好 |
| **命令** | commands | cli/commands | uicmd | 都有类似实现 |
| **模式** | modes (plan mode) | agents/plan | (无独立) | Sage 明确分离 |
| **沙箱** | sandbox | (无) | (无) | Sage 独有安全特性 |
| **工作区** | workspace | project | (无独立) | Sage 分析最完善 |

### 2. 代码量对比

```
Sage-core:     ~47,000 行 Rust (36 模块)
OpenClaude:    ~50,000+ 行 JS (479 模块，高度分散)
Crush:         ~15,000 行 Go (约 30 模块)
```

### 3. 架构风格对比

**Sage-core (Rust)**
- 优势: 类型安全、抽象完善、功能最全面
- 问题: 模块耦合度较高、部分模块过大

**OpenClaude (JavaScript)**
- 优势: 模块极度细分、解耦彻底
- 问题: 过度分散、难以理解整体

**Crush (Go)**
- 优势: 简洁实用、模板化 prompt、SKILL.md 标准
- 问题: 功能相对简单

---

## 最佳方案建议

### 核心原则

1. **采用 Crush 的 SKILL.md 标准格式** (Agent Skills 开放标准)
2. **保持 Sage 的功能完整性** (recovery, learning, checkpoints 等)
3. **学习 Crush 的简洁架构** (工具 = 代码 + 描述文件)
4. **明确模块边界** (每个模块 < 200 行)

### 推荐模块重组

```
sage-core/src/
├── core/                    # 核心抽象 (最小化)
│   ├── types.rs            # 公共类型
│   ├── error.rs            # 错误定义
│   └── result.rs           # Result 类型
│
├── agent/                   # Agent 执行引擎
│   ├── executor/           # 执行器
│   │   ├── unified.rs      # 统一执行循环
│   │   ├── lifecycle.rs    # 生命周期管理
│   │   └── step.rs         # 单步执行
│   ├── state/              # 状态管理
│   └── modes/              # 模式 (plan, code, etc.)
│
├── llm/                     # LLM 层 (保持现有)
│   ├── client.rs           # 客户端抽象
│   ├── providers/          # 各提供者实现
│   ├── fallback/           # 降级策略
│   └── streaming.rs        # 流式处理
│
├── tools/                   # 工具系统 (重构)
│   ├── registry.rs         # 工具注册表
│   ├── executor.rs         # 工具执行器
│   ├── permission/         # 权限系统 (独立)
│   │   ├── rules.rs
│   │   └── handlers.rs
│   ├── sandbox/            # 沙箱 (从顶层移入)
│   └── builtin/            # 内置工具
│       ├── bash/           # 每个工具一个目录
│       │   ├── mod.rs
│       │   └── TOOL.md     # 工具描述 (学习 Crush)
│       ├── read/
│       ├── edit/
│       └── ...
│
├── skills/                  # 技能系统 (增强)
│   ├── registry.rs         # 注册表
│   ├── loader.rs           # SKILL.md 加载器
│   ├── matcher.rs          # 上下文匹配
│   └── builtin/            # 内置 skills
│       ├── rust-expert/
│       │   └── SKILL.md
│       ├── testing/
│       │   └── SKILL.md
│       └── ...
│
├── context/                 # 上下文管理 (合并 session)
│   ├── manager.rs          # 上下文管理器
│   ├── memory.rs           # 短期记忆
│   ├── history.rs          # 对话历史
│   └── compaction.rs       # 压缩策略
│
├── memory/                  # 长期记忆 (保持)
│   ├── manager.rs
│   ├── storage.rs
│   └── retrieval.rs
│
├── config/                  # 配置 (合并 settings)
│   ├── loader.rs           # 配置加载
│   ├── settings.rs         # 运行时设置
│   ├── validation.rs       # 验证
│   └── onboarding.rs       # 初始化向导
│
├── hooks/                   # 钩子系统 (保持)
│   ├── registry.rs
│   ├── executor.rs
│   └── types.rs
│
├── commands/                # 斜杠命令 (保持)
│   ├── registry.rs
│   ├── executor.rs
│   └── builtin/
│
├── mcp/                     # MCP 协议 (保持)
│   ├── client.rs
│   ├── server.rs
│   └── transport.rs
│
├── recovery/                # 恢复系统 (保持，Sage 独有优势)
│   ├── circuit_breaker.rs
│   ├── rate_limiter.rs
│   └── supervisor.rs
│
├── learning/                # 学习系统 (保持，Sage 独有优势)
│   ├── engine.rs
│   ├── patterns.rs
│   └── corrections.rs
│
├── checkpoints/             # 检查点 (保持，Sage 独有优势)
│   ├── manager.rs
│   ├── storage.rs
│   └── restore.rs
│
├── storage/                 # 持久化 (简化)
│   ├── backend.rs          # 后端抽象
│   ├── sqlite.rs
│   └── postgres.rs
│
├── telemetry/               # 遥测 (保持)
│   ├── collector.rs
│   ├── metrics.rs
│   └── cost.rs
│
├── prompts/                 # 提示系统 (增强，学习 Crush)
│   ├── templates/          # 模板目录
│   │   ├── coder.md.tpl    # 主 prompt 模板
│   │   ├── task.md.tpl
│   │   └── ...
│   ├── renderer.rs         # 模板渲染
│   └── variables.rs        # 变量注入
│
├── ui/                      # UI (简化)
│   ├── terminal.rs
│   ├── progress.rs
│   └── components/         # 学习 Crush 组件化
│
├── workspace/               # 工作区分析 (保持)
│   ├── analyzer.rs
│   ├── patterns.rs
│   └── dependencies.rs
│
└── plugins/                 # 插件系统 (保持)
    ├── registry.rs
    ├── loader.rs
    └── types.rs
```

---

## Skill 系统最佳方案

### 采用 Agent Skills 开放标准

参考: https://agentskills.io (Crush 遵循此标准)

```yaml
# SKILL.md 格式
---
name: rust-expert
description: Rust programming expertise for safe, idiomatic code
license: MIT
compatibility: sage >= 0.1.0
metadata:
  author: sage-team
  version: "1.0.0"
---

[Skill instructions in markdown...]
```

### Sage 扩展格式 (兼容 Claude Code)

```yaml
---
name: comprehensive-testing
description: TDD methodology and testing best practices
when_to_use: When user asks for testing, TDD, or test coverage
allowed_tools:
  - Read
  - Grep
  - Bash
  - Edit
user_invocable: true
argument_hint: "[test file or directory]"
priority: 10
model: sonnet  # 可选: 指定模型
---

# Testing Skill

You are an expert in Test-Driven Development...

## Usage

$ARGUMENTS will be substituted with user input.
$USER_MESSAGE contains the original request.
$WORKING_DIR is the current directory.
```

### Skill 发现路径 (优先级从高到低)

1. `.sage/skills/` - 项目级 skills
2. `~/.config/sage/skills/` - 用户级 skills
3. MCP 服务提供的 skills
4. 内置 skills

### Skill 加载示例

```rust
// skills/loader.rs
pub fn discover_skills(paths: &[PathBuf]) -> Vec<Skill> {
    let mut skills = Vec::new();

    for path in paths {
        // 支持两种格式:
        // 1. skill-name.md (直接文件)
        // 2. skill-name/SKILL.md (目录)
        for entry in walkdir::WalkDir::new(path) {
            if entry.file_name() == "SKILL.md"
               || entry.path().extension() == Some("md") {
                if let Ok(skill) = parse_skill_file(&entry.path()) {
                    skills.push(skill);
                }
            }
        }
    }

    skills.sort_by(|a, b| b.priority.cmp(&a.priority));
    skills
}
```

---

## 工具描述文件方案 (学习 Crush)

Crush 的优秀实践: 每个工具有配套的 `.md` 描述文件

```
tools/
├── bash.go
├── bash.tpl       # 工具 prompt 模板
├── edit.go
├── edit.md        # 工具描述和使用说明
├── grep.go
├── grep.md
...
```

### 建议 Sage 采用类似结构

```
crates/sage-tools/src/
├── bash/
│   ├── mod.rs           # 实现
│   ├── TOOL.md          # 描述 (注入 system prompt)
│   └── safety.rs        # 安全检查
├── edit/
│   ├── mod.rs
│   ├── TOOL.md
│   └── diff.rs
...
```

### TOOL.md 格式

```markdown
---
name: Bash
description: Execute shell commands
dangerous: true
requires_permission: true
---

## Description

Executes bash commands in a persistent shell session.

## Parameters

- `command` (required): The command to execute
- `timeout` (optional): Timeout in milliseconds (default: 120000)
- `description` (optional): Brief description of what this command does

## Usage Notes

- Always quote file paths with spaces
- Use && to chain dependent commands
- Avoid interactive commands

## Examples

```bash
# Good
git status && git diff HEAD

# Bad (interactive)
git add -i
```
```

---

## 行动计划

### Phase 1: 模块边界优化
1. 将 `sandbox` 移入 `tools/sandbox/`
2. 将 `modes` 移入 `agent/modes/`
3. 合并 `config` 和 `settings`
4. 合并 `session` 到 `context`

### Phase 2: 技能系统增强
1. 完善 SKILL.md 解析器 (已有基础)
2. 添加 hot-reload 支持 (已有 watcher)
3. 实现 skill 优先级匹配
4. 添加 MCP skill 发现

### Phase 3: 工具描述系统
1. 为每个工具创建 TOOL.md
2. 实现工具描述自动注入 system prompt
3. 统一工具权限管理

### Phase 4: Prompt 模板化
1. 将硬编码 prompt 提取到 .tpl 文件
2. 实现 Go template 风格的渲染器
3. 支持运行时 prompt 定制

---

## 关键差异总结

| 特性 | Sage | OpenClaude | Crush | 推荐 |
|-----|------|------------|-------|------|
| 熔断/恢复 | ✅ | ❌ | ❌ | 保持 Sage |
| 学习系统 | ✅ | ❌ | ❌ | 保持 Sage |
| 检查点 | ✅ | ❌ | ❌ | 保持 Sage |
| SKILL.md 标准 | ✅ | ✅ | ✅ | 统一标准 |
| TOOL.md 描述 | ❌ | ❌ | ✅ | 采用 Crush |
| Prompt 模板化 | 部分 | 部分 | ✅ | 采用 Crush |
| 组件化 TUI | 部分 | ✅ | ✅ | 增强 |
| 多存储后端 | ✅ | ❌ | SQLite | 保持 Sage |

---

## 结论

Sage-core 已有最完善的功能集，主要需要：

1. **简化结构**: 合并相关模块，明确边界
2. **学习 Crush**: 工具描述文件、prompt 模板化
3. **保持优势**: recovery、learning、checkpoints 是独有特性
4. **统一标准**: 完全兼容 Agent Skills 开放标准的 SKILL.md

最终目标：在保持 Sage 功能完整性的同时，达到 Crush 的简洁优雅。
