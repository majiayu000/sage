# Tech Spec

## Linked Issue

GH-126

## Product Spec

`specs/GH126/product.md`

## Codebase Context

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| unified loader | `crates/sage-core/src/config/credential/unified_loader.rs` | `UnifiedConfigLoader` returns `LoadedConfig` with credential status and warnings | 应成为唯一加载入口 |
| execute path | `crates/sage-cli/src/commands/unified/execute.rs` | 仍用 `load_config_from_file` / `load_config` | 主要可达入口 |
| onboarding | `crates/sage-cli/src/commands/interactive/onboarding.rs` | 已调用 `load_config_unified` | 可作为迁移参考 |
| persistence | `crates/sage-core/src/config/persistence.rs` | `ConfigPersistence`/`ConfigUpdate::apply` 有单测无生产调用 | 要接线或删除 |
| tool registration | `sage_tools::get_default_tools*` 调用点，SDK run/unified，TUI/rnk | 默认全量注册工具 | 工具开关目前不生效 |
| settings tools | `crates/sage-core/src/settings/*` | `settings.tools.enabled/disabled` 已有验证语义 | 建议作为权威运行时视图 |

## 设计方案

1. **统一加载 facade**：在 `sage_core::config` 暴露一个执行入口专用函数，例如 `load_runtime_config(args)`，内部使用 `UnifiedConfigLoader` 并返回 `LoadedConfig`。CLI/SDK/TUI/doctor/status 统一消费它；需要 fatal 的场景从 `LoadedConfig.status` 决策，而不是回退旧 loader。
2. **移除旧凭据合并路径**：执行路径不得再调用 `load_config` 后手写 env credential merge。`defaults.rs` 只保留默认值构造，不负责运行时凭据解析。
3. **工具开关权威面**：选择 `settings.tools` 作为唯一运行时工具开关视图。`config.tools.enabled_tools` 若仍存在于配置文件，只能在 load/迁移阶段转换为 `settings.tools.enabled`，随后生产代码只读 `settings.tools`。若仓库接受破坏性调整，则删除 `config.tools.enabled_tools` 字段、示例和测试。
4. **注册过滤 helper**：在工具注册前统一调用 helper，例如 `filter_tools_by_settings(all_tools, &settings.tools)`。CLI execute、SDK run/unified、TUI/rnk、MCP 合并后的工具列表都使用同一个 helper。禁用工具不可出现在 schema 列表；直接调用禁用工具返回明确 unknown/disabled error。
5. **ConfigPersistence 决策**：优先接入真实写回路径，包括 onboarding 写入、slash `/model` 切换持久化或现有 config update 命令。如果没有生产写回路径需要它，则删除 `ConfigPersistence`/`ConfigUpdate` 导出与模块，避免 public dead API。
6. **错误处理**：loader warnings 向 CLI/TUI 显示或记录；配置冲突使用 validation error，不能 warning 后继续忽略。

## Product-to-Test Mapping

| Product invariant | Implementation area | Verification |
| --- | --- | --- |
| P1/P2 | config facade + call sites | search check 确认可达执行路径不直接调用旧 loader；credential status 一致测试 |
| P3/P4 | tool filtering helper | enabled-only、disabled-only、冲突、禁用工具调用测试 |
| P5 | persistence module | production call search 或删除后的 compile verification |
| P6 | validation/error handling | invalid tool config 返回 error，不静默忽略 |

## 数据流

CLI/SDK/TUI args -> `UnifiedConfigLoader` -> `LoadedConfig { config, status, warnings }` -> settings tools runtime view -> build default tools + MCP tools -> `filter_tools_by_settings` -> executor.register_tools(filtered_tools)。

## 备选方案

- 保留 `config.tools.enabled_tools` 作为权威面：会继续和 `settings.tools` 重叠，除非删除 settings 工具开关。当前 settings 已有 enabled/disabled 验证，保留它的改动面更小。
- 只在 CLI 过滤工具：SDK/TUI 仍会全量注册，不满足一条配置语义。

## 风险

- Compatibility: 删除 `config.tools.enabled_tools` 是破坏性调整；按仓库规则更新版本/文档，不提供 deprecated shim。
- Security: 禁用危险工具必须 fail closed；MCP 动态工具也要经过同一过滤。
- Maintenance: 所有入口都要走同一 helper，避免新增入口忘记过滤。

## 测试计划

- [ ] Unit tests: settings tool filter helper enabled/disabled/conflict。
- [ ] Integration tests: CLI/SDK 注册禁用工具后不可见/不可调用。
- [ ] Search checks: 执行路径旧 loader 调用清零；`ConfigPersistence` 有生产调用或模块删除。

## 回滚方案

回滚统一 loader 和过滤 helper 会恢复旧配置行为；若已删除 schema 字段，回滚需同步恢复示例和 tests。
