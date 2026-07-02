# Tech Spec

## Linked Issue

GH-136

## Product Spec

`specs/GH136/product.md`

## Codebase Context

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| 记忆管理 | `crates/sage-core/src/memory/`，`crates/sage-tools/src/tools/diagnostics/memory/` | core 有 manager/storage；`init_global_memory_manager` / `get_memories_for_context(limit)` 目前在 `sage-tools` 诊断层 | 需先明确 runtime API 所属，避免 `sage-core` 反向依赖 `sage-tools` |
| 学习引擎 | `crates/sage-core/src/learning/`，`tools/diagnostics/learning/` | core 有 engine；`init_global_learning_engine` 只是 `create_learning_engine(config)`，未挂 memory manager / 持久存储，`get_learning_patterns_for_context(limit)` 在 `sage-tools` | 需接线的能力 |
| 上下文/prompt 构建 | `crates/sage-core/src/agent/unified/context_builder.rs`，`crates/sage-core/src/prompts/` | 不拉取记忆/学习 | 注入点 |
| 启动构造 | `crates/sage-core/src/agent/unified/constructor.rs` | 未初始化全局记忆/学习 | 初始化点 |
| 上下文压缩 | `crates/sage-core/src/context/auto_compact` | 溢出压缩 | 注入需与之协作 |

## 设计方案

1. **runtime API 归属**：先把 agent runtime 需要的初始化与 recall API 下沉到 `sage-core`（例如 `memory::runtime` / `learning::runtime`），`sage-tools` 只保留诊断工具的薄封装；`sage-core` 不得依赖 `sage-tools`。新增 `RecallQuery { task_text, recent_messages, touched_paths, limit }`，替代 limit-only 查询。
2. **初始化**：在 `constructor.rs`（或 CLI/SDK 启动路径）按项目/存储路径初始化 core-owned memory manager 与 learning engine；learning engine 必须通过 `with_memory_manager`（或等价持久化接线）挂上同一 memory manager，并在第一次 prompt 前加载持久化数据。初始化 registry 按 project/storage path 分区且幂等：同一路径重复初始化复用，路径不同相互隔离，失败 error 级上报，不 panic、不静默。
3. **注入**：在 `context_builder.rs` 增加一个「recall」步骤，使用 `RecallQuery` 调用 core-owned memory/learning recall，把结果渲染进 `prompts` 的专用系统区段（复用 `system_reminders` 风格）。
4. **上界与 redaction**：注入前经 `diagnostics/redaction` 脱敏，并按相关性排序截断到配置上界，记录 dropped count。
5. **outcome 学习**：在任务收尾（`agent/completion.rs` 或 execution_loop 结束）先最小化并脱敏轨迹，再写入带成功/失败标记的 outcome；失败先提炼为教训再落库（W-37），不得把原始 secret/token/敏感路径持久化。
6. **开关**：配置项（如 `memory.enabled`）控制初始化+注入；关闭时跳过 recall，与基线一致。
7. **与 compact 协作**：recall 注入的 token 计入 estimator，避免挤爆窗口；必要时记忆区段参与裁剪优先级。

## Product-to-Test Mapping

| Product invariant | Implementation area | Verification |
| --- | --- | --- |
| P1 | core runtime 初始化 + constructor 接线 | 持久存储加载、learning engine 挂 memory manager、同路径幂等/不同路径隔离、初始化失败上报 单测 |
| P2/P7 | context_builder recall | `RecallQuery` 有任务/消息/路径→相关注入、无记忆→不注入 单测 |
| P3 | 上界截断 | 超上界截断且记 dropped count 单测 |
| P4 | redaction | outcome 落库前脱敏 + prompt 注入前脱敏 fixture 测试 |
| P5 | outcome 学习 | 成功/失败轨迹写入与检索单测 |
| P6 | 开关 | enabled=false 回归无注入基线 单测 |

## 数据流

会话开始→按项目/存储路径幂等初始化 core runtime→加载持久化 memory/learning→每步构建 context 时用 `RecallQuery` recall（脱敏+截断）→注入 prompt→任务结束先最小化/脱敏 outcome→落库供下次 recall。

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
