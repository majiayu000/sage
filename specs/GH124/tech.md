# Tech Spec

## Linked Issue

GH-124

## Product Spec

`specs/GH124/product.md`

## Codebase Context

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| 模型目录缓存 | `crates/sage-core/src/config/model_catalog.rs` | `ModelCatalogManager` 可生成 snapshot、merge remote、处理 304 与 cache write error，但生产路径未调用 | 本 issue 要接线的核心能力 |
| 在线模型 API | `crates/sage-core/src/config/models_api.rs` | `ModelsApiClient` 提供 provider-specific fetch | manager 需要通过它刷新远端列表 |
| `/model` 入口 | `crates/sage-cli/src/commands/unified/slash_commands.rs` | `ModelSelect` 直接 `ModelsApiClient::new()` 并手写静态 fallback warning | 当前绕过缓存与 freshness |
| provider 元数据 | `ProviderRegistry` + `Config.model_providers` | 静态模型和用户 base_url/api_key 在 CLI 中拼装 | 刷新 API 需要复用相同输入 |

## 设计方案

1. **新增 core 层刷新 API**：在 `sage_core::config` 中新增窄接口，例如 `ModelCatalogService` / `refresh_provider_catalog`，内部组合 `ModelCatalogManager` 与 `ModelsApiClient`。CLI 只传 provider id、provider info、base_url/api_key 与 cache root，不再手写 fetch/fallback。
2. **缓存路径**：默认使用 `default_data_dir_or_warn().join("model_catalog")`，测试可注入临时目录。缓存文件格式继续使用 `CatalogCacheEntry`，避免引入第二套 schema。
3. **TTL/ETag 行为**：
   - 未过期缓存直接返回 `CatalogSource::Cache`/`CatalogFreshness::Fresh`。
   - 过期缓存发起远端刷新，带上已有 ETag。
   - 远端 304 调用 `not_modified_snapshot` 更新 `fetched_at`。
   - 远端成功调用 `merge_remote` 写缓存。
   - 远端失败调用 `snapshot` 返回 stale cache 或 static fallback，并填充 `last_error`。
4. **CLI 接线**：`ModelSelect` 调用 core 刷新 API，展示模型 id 时同时保留 fallback warning。如果 freshness/source 表示 stale/static fallback，输出一条明确但短的说明。
5. **错误策略**：网络失败、cache write failure、provider 不支持在线列表都不得 silent swallow。用户可继续选静态或缓存模型，但输出必须包含原因。

## Product-to-Test Mapping

| Product invariant | Implementation area | Verification |
| --- | --- | --- |
| P1/P4 | core 刷新 API + CLI 接线 | mock `ModelsApiClient` 或 trait-backed fetcher 断言 `/model` 不直接绕过 manager |
| P2 | `merge_remote` error path | cache dir 不可写测试，last_error 非空且 warning 记录 |
| P3 | remote failure fallback | stale cache vs no cache 两类测试 |
| P5 | TTL/ETag | 未过期不 fetch、过期 fetch、304 更新 fetched_at 单测 |
| P6 | provider/credential handling | 缺 key 不伪装 live refresh 的 CLI/action 测试 |

## 数据流

`/model` -> load config/provider info -> core catalog refresh API -> `ModelCatalogManager.snapshot` 判断缓存 freshness -> optional `ModelsApiClient` fetch with ETag -> manager merge/not_modified/fallback -> CLI 渲染模型列表与 fallback warning。

## 备选方案

- 删除 `ModelCatalogManager` 并保留直接 fetch：能消除零调用方，但放弃已实现缓存与失败处理，不符合 issue 期望，除非维护者明确撤销模型目录缓存能力。

## 风险

- Security: API key 只用于 provider fetch，不写入模型目录缓存。
- Compatibility: UI 流程不变；新增 warning 文案可能影响 snapshot 测试。
- Performance: 未过期缓存减少网络请求；过期刷新仍需遵守现有超时。
- Maintenance: `ModelsApiClient` 和 manager 组合应在 core 层集中，避免 CLI 再次复制 provider 逻辑。

## 测试计划

- [ ] Unit tests: TTL/ETag、remote success/failure、cache write failure、static fallback。
- [ ] CLI/action tests: `/model` 使用 manager 结果并展示 stale/static fallback。
- [ ] Search check: `rg -n "ModelsApiClient::new\\(\\)" crates/sage-cli/src/commands/unified/slash_commands.rs` 不再命中 `/model` 路径。

## 回滚方案

回滚 core 刷新 API 与 CLI 接线后恢复直接 fetch；缓存文件格式未变，无迁移需求。
