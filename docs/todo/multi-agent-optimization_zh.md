# Multi-Agent 系统优化计划

基于对 Sage Agent 和 openClaude (Claude Code 反编译) 的对比分析，记录后续优化方向。

## 分析日期
2024-12-24

## 对比总结

### 架构对比

| 维度 | Sage (Rust) | Claude Code (JS) |
|------|-------------|------------------|
| **语言** | Rust | JavaScript (Node.js) |
| **Agent 类型** | 3 种 (GeneralPurpose/Explore/Plan) | 3 种 (Task/Explore/Plan) |
| **输出方式** | 完整结果返回 | 异步生成器 (流式 yield) |
| **工具系统** | Tool trait + registry | 25 个内置工具 |
| **状态管理** | Arc<RwLock<T>> | React context |
| **多 Agent 调度** | 显式 (SubAgentRunner) | 隐式 (提示词指导) |

### 关键差异

1. **流式输出**: Claude Code 使用 `async function*` 生成器实现流式输出，Sage 返回完整结果
2. **Hook 系统**: Claude Code 有内置事件点 (SubagentStart/UserPromptSubmit/SubagentStop)
3. **并行调度**: Claude Code 通过提示词隐式指导并行，Sage 需要显式调用

---

## 优化建议

### P0 - 高优先级

#### 1. 流式输出支持
**现状**: Sage 的 `SubAgentRunner::execute()` 返回完整 `SubAgentResult`

**目标**: 支持流式输出，提升用户体验

**参考**: `openClaude/agents/agents_004.js:1306` - `runAgentAsync()`

```rust
// 建议实现
pub fn execute_stream(&self, config: SubAgentConfig)
    -> impl Stream<Item = AgentEvent> + '_ {
    // 返回流式事件
}
```

#### 2. Hook 系统增强
**现状**: 有 LifecycleHook trait，但事件点较少

**目标**: 添加更多 Hook 点，对齐 Claude Code

**需要添加的 Hook**:
- `SubagentStart` - 子代理启动
- `SubagentStop` - 子代理停止
- `ToolExecutionStart/End` - 工具执行前后
- `UserPromptSubmit` - 用户输入提交

**文件位置**: `crates/sage-core/src/agent/lifecycle/`

### P1 - 中优先级

#### 3. 并行 Agent 调度优化
**现状**: 需要显式调用多个 Task

**目标**: 支持提示词隐式并行

**参考**: `openClaude/agents/agents_011.js:224`
```
"You can launch up to ${Q} agent(s) in parallel."
```

**实现思路**:
- 在系统提示词中加入并行指导
- SubAgentRunner 支持批量任务提交
- 结果聚合和依赖管理

#### 4. Agent 执行记录聚合
**现状**: 每个 Agent 独立记录

**目标**: 实现 `fetchAgentTranscripts()` 功能，聚合多个 Agent 结果

**参考**: `openClaude/agents/agents_006.js`

### P2 - 低优先级

#### 5. 动态 Agent 定义
**现状**: 静态 Rust struct 定义

**目标**: 支持运行时动态加载 Agent 定义（Markdown/YAML）

#### 6. Agent 间消息传递
**现状**: 无直接通信机制

**目标**: 支持 Agent 之间的消息传递和协作

---

## 关键文件位置

### Sage 项目
| 组件 | 位置 |
|------|------|
| Subagent 模块 | `crates/sage-core/src/agent/subagent/` |
| Agent 类型 | `subagent/types/agent_type.rs` |
| Agent 定义 | `subagent/types/agent_definition.rs` |
| 运行器 | `subagent/runner.rs` |
| 注册表 | `subagent/registry.rs` |
| 生命周期 | `agent/lifecycle/` |

### openClaude 参考
| 组件 | 位置 |
|------|------|
| 执行主循环 | `agents/agents_004.js:1306` - `runAgentAsync()` |
| Hook 执行 | `agents/agents_005.js:323` - `executeAgentHook()` |
| 多 Agent 管理 | `agents/agents_006.js` - `fetchAgentTranscripts()` |
| 并行指令 | `agents/agents_011.js:224` |

---

## 实施路线

```
Phase 1: 流式输出
├── 定义 AgentEvent 枚举
├── 实现 execute_stream() 方法
└── CLI 集成流式显示

Phase 2: Hook 系统
├── 添加新 Hook 事件类型
├── 实现 Hook 执行器
└── 集成到执行循环

Phase 3: 并行调度
├── 批量任务提交 API
├── 结果聚合逻辑
└── 系统提示词优化
```

---

## 备注

- openClaude 项目位置: `/Users/Zhuanz/Desktop/code/Open/AI/code-agent/openClaude`
- 该项目是 Claude Code v2.0.62 的反编译分析，提供了官方实现的参考
