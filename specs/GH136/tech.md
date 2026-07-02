# Tech Spec

## Linked Issue

GH-136

## Product Spec

`specs/GH136/product.md`

## Codebase Context

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| 记忆管理 | `crates/sage-core/src/memory/`，`crates/sage-tools/src/tools/diagnostics/memory/` | 实现完整，`init_global_memory_manager` 仅 re-export，无生产调用 | 需接线的能力 |
| 学习引擎 | `crates/sage-core/src/learning/`，`tools/diagnostics/learning/` | 同上，`get_learning_patterns_for_context` 无消费者 | 需接线的能力 |
| 上下文/prompt 构建 | `crates/sage-core/src/agent/unified/context_builder.rs`，`crates/sage-core/src/prompts/` | 不拉取记忆/学习 | 注入点 |
| 启动构造 | `crates/sage-core/src/agent/unified/constructor.rs` | 未初始化全局记忆/学习 | 初始化点 |
| 上下文压缩 | `crates/sage-core/src/context/auto_compact` | 溢出压缩 | 注入需与之协作 |

## 设计方案

1. **初始化**：在 `constructor.rs`（或 CLI/SDK 启动路径）按配置调用 `init_global_memory_manager` / `init_global_learning_engine`，存储路径来自配置；失败 error 级上报，不 panic、不静默。
2. **注入**：在 `context_builder.rs` 增加一个「recall」步骤，调用 `get_memories_for_context` / `get_learning_patterns_for_context`（以当前任务/最近消息为查询），把结果渲染进 `prompts` 的专用系统区段（复用 `system_reminders` 风格）。
3. **上界与 redaction**：注入前经 `diagnostics/redaction` 脱敏，并按相关性排序截断到配置上界，记录 dropped count。
4. **outcome 学习**：在任务收尾（`agent/completion.rs` 或 execution_loop 结束）写入带成功/失败标记的轨迹；失败先提炼为教训再落库（W-37）。
5. **开关**：配置项（如 `memory.enabled`）控制初始化+注入；关闭时跳过 recall，与基线一致。
6. **与 compact 协作**：recall 注入的 token 计入 estimator，避免挤爆窗口；必要时记忆区段参与裁剪优先级。

## Product-to-Test Mapping

| Product invariant | Implementation area | Verification |
| --- | --- | --- |
| P1 | constructor 初始化 | 初始化成功/失败单测 |
| P2/P7 | context_builder recall | 有记忆→注入、无记忆→不注入 单测 |
| P3 | 上界截断 | 超上界截断且记 dropped count 单测 |
| P4 | redaction | secret 注入被脱敏 fixture 测试 |
| P5 | outcome 学习 | 成功/失败轨迹写入与检索单测 |
| P6 | 开关 | enabled=false 回归无注入基线 单测 |

## 数据流

会话开始→加载/初始化存储→每步构建 context 时 recall（脱敏+截断）→注入 prompt→任务结束写 outcome 轨迹→落库供下次 recall。

## 备选方案

- 把记忆做成一个工具让模型自行查询（而非自动注入）：更省上下文但依赖模型主动性（W-38 knowing-doing gap），建议二者结合：自动注入高相关 + 保留查询工具。
- 保持诊断定位、不能力化：符合 U-32 downgrade，但放弃跨会话学习能力。

## 风险

- Security: 注入历史内容需严格 redaction，否则跨会话泄漏 secret。
- Compatibility: 默认 opt-in 则无行为变更；默认开启需在发布说明声明。
- Performance: 每步 recall 增加检索与 token 开销；用上界与相关性阈值控制。
- Maintenance: 记忆区段与 compact 优先级需保持一致。

## 测试计划

- [ ] Unit tests: 初始化、注入有/无、上界截断、redaction、outcome 学习、开关。
- [ ] Integration tests: 一次会话写入→下次会话 recall 命中。
- [ ] Manual verification: 连续两次会话，确认第二次注入了第一次的记忆。

## 回滚方案

配置开关关闭即回到无注入基线；或 revert 接线 commit。
