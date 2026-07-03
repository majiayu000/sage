# Product Spec

## Linked Issue

GH-124

## 用户问题

`ModelCatalogManager` 已经实现了磁盘缓存、TTL、ETag 与失败回退语义，但 `/model` 的可达路径仍直接调用 `ModelsApiClient`。用户看到的是每次实时拉取或静态回退，无法获得缓存复用、陈旧状态或失败原因，代码里也保留了一套看似完成但生产未消费的模型目录子系统。

## 目标

- 让 `/model` 的实时刷新和后续模型目录消费方统一经由 `ModelCatalogManager`。
- 远端刷新成功时写入缓存；远端失败时明确返回静态或缓存回退，并携带 freshness/source/last_error。
- 保证 workspace 内不存在零调用方的模型目录缓存子系统。

## 非目标

- 不新增模型 provider。
- 不改变模型选择 UI 的交互流程。
- 不把 #126 的配置加载管线收敛并入本 issue。

## Behavior Invariants

1. `/model` 列表获取不得绕过 `ModelCatalogManager` 直接使用 `ModelsApiClient` 作为唯一数据路径。
2. 远端返回新模型列表时，缓存文件写入成功或失败都可观测；写入失败不得吞掉。
3. 远端不可用时，优先返回未过期或陈旧缓存；无缓存时返回静态 provider 列表，并附带明确的 fallback warning。
4. freshness/source/last_error 语义从 manager 传到调用方，不丢失。
5. ETag 和 TTL 生效：未过期缓存不重复远端请求；远端 304 更新 fetched_at。
6. provider 未配置或缺少凭据时按现有 UI 行为处理，但不能伪装成实时刷新成功。

## 验收标准

- [ ] `/model` 的实时刷新经由 `ModelCatalogManager`，含 ETag/TTL/失败回退。
- [ ] 网络失败路径有可观测日志与用户可见陈旧/回退标记。
- [ ] 行为有测试覆盖：fresh cache、stale cache、remote success、remote failure、not modified、cache write failure。
- [ ] `rg -l ModelCatalogManager crates/` 显示至少一个生产调用方。

## 边界情况

- 缓存目录不可写：模型列表仍可用，但 last_error 传出并记录 warning。
- provider 不支持在线模型列表：返回静态列表，并标注 static fallback。
- 缺少 API key：不发起需要凭据的远端请求，返回静态或缓存结果。

## 发布说明

开发者可见行为变化：`/model` 会复用本地模型目录缓存，并在远端失败时显示缓存或静态回退来源。

## 开放问题

- 缓存目录使用现有 `default_data_dir_or_warn()` 下的哪个子路径？建议由 tech spec 固定为 `model_catalog/`。
