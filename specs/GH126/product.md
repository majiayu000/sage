# Product Spec

## Linked Issue

GH-126

## 用户问题

配置子系统有并行加载管线和多个不生效配置面：`UnifiedConfigLoader` 只有 onboarding 使用，执行路径仍走旧 `load_config`；`ConfigPersistence` 完整实现但生产不调用；工具启用/禁用配置不会过滤实际注册工具。用户以为配置已经生效，运行时却继续使用旧凭据合并和全量工具注册。

## 目标

- 所有执行入口使用同一条配置加载管线和凭据解析来源。
- 工具启用/禁用配置实际影响工具注册。
- 配置写回层被生产写入路径消费，或未使用代码被删除。
- 配置所有权清晰：工具开关只保留一个权威运行时视图。

## 非目标

- 不改变 LLM provider 的语义。
- 不新增配置格式。
- 不把 #124 模型目录缓存接线并入本 issue。

## Behavior Invariants

1. CLI execute、TUI/rnk、slash commands、doctor/status、SDK 执行路径都经由统一 loader，凭据解析结果一致。
2. 旧 `defaults.rs` 内联凭据合并不再作为执行路径的一部分。
3. 工具注册前必须应用有效工具过滤；被禁用工具不可见且不可调用。
4. 只存在一个权威工具开关运行时视图。若保留旧 schema 字段，只能在 load 阶段派生到该视图，不能被生产代码独立读取。
5. `ConfigPersistence` 要么接入真实写回流程，要么删除导出与死代码。
6. 配置加载失败、写回失败、工具过滤配置冲突必须返回 error 或明确 warning，不得静默使用错误配置。

## 验收标准

- [ ] 所有 `load_config*` 执行入口经由统一 loader。
- [ ] 工具启用/禁用配置实际过滤注册，禁用后调用被拒或不可见。
- [ ] 只保留一个权威工具开关面；另一个被删除或只在 load 阶段派生。
- [ ] `ConfigPersistence` 被接入配置写回流程或删除。
- [ ] 行为差异有测试覆盖。

## 边界情况

- enabled 与 disabled 同时包含同一工具：配置验证失败或明确 error。
- 所有工具被禁用：agent 可启动，但工具列表为空；请求禁用工具返回明确错误。
- 统一 loader 找不到配置文件：沿用可用 defaults，但附带 status/warning。

## 发布说明

配置行为修复：执行入口统一使用新凭据解析，工具开关开始真实生效。若删除旧工具开关 schema，需要按仓库规则记录破坏性变更。

## 开放问题

- 是否删除 `config.tools.enabled_tools` schema 字段？建议在本 spec 中认定删除是必要破坏性调整，运行时权威面固定为 `settings.tools`。
