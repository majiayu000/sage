# Task Plan

## Linked Issue

GH-124

## Spec Packet

- Product: `product.md`
- Tech: `tech.md`

## 实现任务

- [ ] `SP124-T01` Owner: config. Done when: core 提供组合 `ModelCatalogManager` + `ModelsApiClient` 的刷新 API，支持 cache root 注入、TTL/ETag、304、remote failure fallback. Verify: `cargo test -p sage-core model_catalog`。
- [ ] `SP124-T02` Owner: cli. Done when: `/model` 的 `ModelSelect` 调用 core 刷新 API，不再直接手写 live fetch/fallback，stale/static fallback 对用户可见. Verify: `cargo test -p sage-cli slash_commands`。
- [ ] `SP124-T03` Owner: config. Done when: cache write failure、缺 key、provider 不支持在线列表都有可观测 last_error/warning 且不吞掉. Verify: `cargo test -p sage-core model_catalog`。

## 并行拆分

T01 与 T02 串行，T02 依赖刷新 API；T03 与 T01 同区域，和 T01 合并实现更稳。

## 验证

- `cargo fmt --all -- --check`
- `cargo clippy -p sage-core -p sage-cli --all-targets -- -D warnings`
- `cargo test -p sage-core model_catalog`
- `cargo test -p sage-cli slash_commands`
- `python3 checks/check_workflow.py --repo <specrail> --spec-dir <repo>/specs/GH124`

## Handoff Notes

不要把 API key 写入缓存；fallback 允许继续返回模型列表，但必须带 freshness/source/last_error，不能表现成实时刷新成功。
