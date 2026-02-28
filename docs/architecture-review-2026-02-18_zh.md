# Sage Agent 架构审查报告

> 日期: 2026-02-18 | 版本: 0.13.32

## 一、架构设计问题

### P0 — 必须解决

#### 1. sage-core 是 God Crate

lib.rs 有 25 个 `pub use` 块、200+ 公开符号。UI/存储/遥测/LLM/MCP/sandbox/hooks 全塞一个 crate。

**影响**: 任何模块改动触发全量重编译；依赖膨胀（rnk、notify、flate2、dashmap、sha2 等即使只需 LLM 客户端也得全编译）。

**建议拆分**:
| 新 crate | 来源模块 | 理由 |
|----------|----------|------|
| sage-llm | llm/, types/provider.rs | LLM 客户端独立关注点 |
| sage-ui | ui/ | 终端 UI 不应耦合在 core |
| sage-storage | storage/ | 数据库后端是可选能力 |
| sage-telemetry | telemetry/ | 可观测性是横切关注点 |

#### 2. 全局可变单例泛滥（10+个）

| 全局变量 | 文件 |
|----------|------|
| GLOBAL_TELEMETRY | telemetry/tool_usage.rs:228 |
| GLOBAL_METRICS | telemetry/collector/types.rs:77 |
| GLOBAL_INTERRUPT_MANAGER | interrupt.rs:148 |
| BACKGROUND_REGISTRY | tools/background_registry.rs:191 |
| GLOBAL_RUNNER | agent/subagent/runner.rs:385 |
| GLOBAL_CONFIG | sage-tools/config.rs:190 |
| GLOBAL_TODO_LIST | sage-tools/task_mgmt/todo_write.rs:85 |
| GLOBAL_MCP_REGISTRY | sage-tools/mcp_tools/servers_tool.rs:14 |
| GLOBAL_MONITOR | sage-tools/monitoring.rs:242 |
| SHELL_REGISTRY | sage-tools/process/kill_shell.rs:13 |

**影响**: 测试隔离不可能；依赖关系不可见；初始化顺序脆弱。

**建议**: 通过构造函数注入，UnifiedExecutor 作为聚合根持有这些实例。

#### 3. UnifiedExecutor 是 God Object

方法分散在 12 个文件（mod.rs, constructor.rs, builder.rs, executor.rs, execution_loop.rs, step_execution.rs, message_builder.rs, user_interaction.rs, session_manager.rs, session_recording.rs, session_restore.rs, session_branching.rs），但仍通过 `&mut self` 访问所有状态。

**建议**: 将 session 相关方法真正委托给 AgentSessionManager，而非在 UnifiedExecutor 上加 impl 块。

### P1 — 应该解决

| # | 问题 | 建议 |
|---|------|------|
| 4 | LlmOrchestrator 三个重复 streaming 方法 | 删除 `stream_chat_with_display`，统一到 Strategy |
| 5 | traits.rs 定义 5 个 trait 从未使用 | 要么注入 UnifiedExecutor，要么删除 |
| 6 | 6 个无效 extension traits (blanket impl) | 删除，保留 Tool trait 本身 |
| 7 | SageError vs ToolError 边界模糊 | 统一错误传播，避免两次 .to_string() |
| 8 | SubAgentRunner 重复 LLM 客户端构建逻辑 | 提取 `LlmClient::from_config()` 工厂函数 |

### P2 — 可以改进

| # | 问题 | 建议 |
|---|------|------|
| 9 | deprecated UI bridge 仍在核心路径 | 迁移到 EventManager/OutputStrategy |
| 10 | ProviderInstance enum + trait 冗余分发 | 选择 `Box<dyn LlmProviderTrait>` 消除 enum |


## 二、与 Claude Code 的差距

### 关键缺失（P0）

| 能力 | Claude Code | Sage 现状 |
|------|-------------|-----------|
| 项目指令自动加载 | 自动读取 `.claude/CLAUDE.md` 注入 system prompt | 发现和注入机制不完整 |
| Prompt caching | 利用 Anthropic cache_control 减少重复 token 成本 | `cache_control` 字段存在但永远 `None` |
| Context overflow 恢复 | 自动 compact 后重试 | 直接报错，无自动恢复 |

### 重要缺失（P1）

| 能力 | Claude Code | Sage 现状 |
|------|-------------|-----------|
| 真实 tokenizer | tiktoken 精确计数 | 字符估算（4 chars/token），误差 30-50% |
| 流式 token 显示 | 逐 token 输出 + 光标 | 无流式显示 |
| 文件编辑 diff 视图 | 彩色 unified diff | 无 diff 展示 |
| 并行 subagent | 同时 spawn 多个 Task agent | 只能串行执行 |

### 次要缺失（P2）

| 能力 | 说明 |
|------|------|
| LSP/WebSearch/WebFetch 工具 | 代码智能和联网能力缺失 |
| Extended thinking | 无 `--thinking-budget` 支持 |
| CLI flags | 缺 `--model`、`--permission-mode`、`--allowedTools` |
| Session 列表/搜索 | 后端支持但无用户界面 |

### Sage 优势点

- Hook 系统更丰富：13 种事件 vs Claude Code 的 4 种
- Recovery 模块完整：熔断器、双策略重试、模型 fallback 链
- Sandbox 更深入：OS 级隔离、命令验证、路径策略、违规追踪

## 三、代码质量问题

### P0 — 安全

| 文件 | 问题 | 修复 |
|------|------|------|
| `sandbox/policy/command_policy.rs:89` | `bash -c "rm -rf /"` 绕过 sandbox — `bash` 不在 blocked 列表 | 加入 `bash`；递归解析 `sh -c` 内部命令 |

### P1 — 并发/逻辑

| 文件 | 问题 | 修复 |
|------|------|------|
| `storage/manager/kv_store.rs:40` | `let _ = backend_ref` 不会释放 read guard，注释与行为不符 | 显式 `drop(backend_lock)` |
| `signal_handler.rs:206` | 持有 parking_lot Mutex 跨 await，潜在死锁 | 改用 `tokio::sync::Mutex` |

### P2 — 质量

| 文件 | 问题 |
|------|------|
| `rnk_app/executor/creation.rs:79` | `let _ =` 静默吞掉 subagent/session 初始化错误 |
| `log_analyzer.rs:170`, `template.rs:79` | 生产代码 `Regex::new().unwrap()` 违反 RS-03 |
| `hooks/registry.rs:40` | `.read().ok()` 静默吞掉 poisoned lock，hooks 全部失效无日志 |
| `commands/unified/stream.rs:98` | `u64 as usize` 窄化转换，32-bit 平台截断 |

## 四、VibeGuard 集成状态

### 实际路径

vibeguard 仓库位于 `~/Desktop/code/AI/tool/vibeguard/`（注意：`tool` 不是 `tools`）。

### 全局 CLAUDE.md（`~/.claude/CLAUDE.md`）

全局配置中的 VibeGuard 部分（第 40-138 行）与源模板 `vibeguard/claude-md/vibeguard-rules.md` 内容一致，包含：
- 7 层核心约束（L1-L7）
- 守卫 ID 索引（RS/U/SEC/TS/PY/GO）
- 7 个 Hooks 规则（4 Block + 3 Warn）
- 6 个 slash commands + 2 个 MCP 工具
- 可观测性（events.jsonl）
- 12 个专项 Agent
- 修复流程和优先级

### 项目 CLAUDE.md（`sage/CLAUDE.md`）

项目级已集成：
- 3 个 Rust 守卫脚本（RS-05 重复类型、RS-01 嵌套锁、RS-03 unwrap）
- 6 个架构守卫测试（RS-ERR-01/02、RS-LLM-01、RS-MCP-01、RS-SIZE-01、RS-NAME-01）
- Makefile 有 `make guard` / `make guard-strict` / `make arch-guard`

### 问题

| 问题 | 影响 | 位置 |
|------|------|------|
| 路径写错 `tools` → 应为 `tool` | `make guard` 命令失败 | Makefile:95, 全局 CLAUDE.md:44, vibeguard-rules.md:5 |
| `.vibeguard-duplicate-types-allowlist` 不存在 | 无法抑制已知重复类型误报 | 项目根目录 |
| vibeguard 未提供 Rust 专项守卫 RS-02/RS-04 | 缺少 `as` 窄化转换和 Vec::remove(0) 检测 | guards/rust/ |

### 与 vibeguard 最新版对比

| 能力 | vibeguard 提供 | Sage 使用情况 |
|------|---------------|--------------|
| 规则注入（CLAUDE.md） | ✅ 7 层约束 | ✅ 全局已注入 |
| Hooks 实时拦截 | ✅ 5 个 hook 脚本 | ✅ 全局已配置 |
| MCP Server | ✅ guard_check / compliance_report / metrics_collect | ⚠️ 未确认是否启用 |
| Rust 守卫脚本 | ✅ 4 个（duplicate/nested_locks/unwrap/workspace_consistency） | ✅ Makefile 集成 3 个 |
| Rust 项目模板 | ✅ `project-templates/rust-CLAUDE.md` | ⚠️ 未采用（Sage 有自己的 CLAUDE.md 但结构不同） |
| Skills | ✅ vibeguard/eval-harness/iterative-retrieval/strategic-compact | ✅ 可用 |
| Workflows | ✅ auto-optimize/fixflow/optflow/plan-folw/plan-mode | ✅ 可用 |
| Context profiles | ✅ dev/review/research | ❌ 未使用 |
| CI 验证脚本 | ✅ validate-guards/hooks/rules | ❌ 未集成到 CI |

## 五、建议优先级路线图

### 短期（1-2 周）
1. 修复 P0 安全：sandbox `bash -c` 绕过
2. 修复 P1 并发：kv_store lock 释放、signal_handler Mutex
3. 提取 `LlmClient::from_config()` 消除重复
4. 删除无效 extension traits 和未使用 traits
5. 修复 Makefile vibeguard 路径

### 中期（1-2 月）
1. 全局单例 → 依赖注入（从 telemetry/metrics 开始）
2. UnifiedExecutor session 方法委托给 AgentSessionManager
3. 完成 deprecated UI bridge 迁移
4. 实现 context overflow 自动恢复
5. 接入 prompt caching

### 长期
1. sage-core crate 拆分（sage-llm, sage-ui, sage-storage, sage-telemetry）
2. 真实 tokenizer 集成
3. 并行 subagent 执行
4. 流式 token 显示 + diff 视图
5. LSP/WebSearch 工具
