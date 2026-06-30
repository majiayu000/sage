# Product Spec

## Linked Issue

GH-89

## 用户问题

Sage 需要安全地保存 provider 凭据，同时保持 env override 和现有配置兼容。模型目录和 capability 信息也需要可刷新、可离线回退，并避免多个来源各自维护不一致的能力判断。

## 目标

- 引入 secure-by-default credential backend abstraction。
- 保持 env override 优先级明确，并支持 legacy plaintext JSON import/fallback。
- 提供 save、logout、revoke 和恢复提示。
- 建立可刷新 provider/model catalog，支持 TTL、ETag 和 offline fallback。
- 将 model capability lookup 收敛到一个 manager。

## 非目标

- 不在本轮实现所有 provider 的 OAuth 流程。
- 不移除静态/离线 fallback。
- 不在测试或配置中写入真实 API key。
- 不包含桌面 app、IDE 入口或 app-server client。

## Behavior Invariants

1. Env credential override 必须优先于 durable saved credential。
2. Durable save 应优先使用平台安全存储；不支持时必须明确 fallback 状态。
3. Logout/revoke 失败必须返回可恢复错误和 provider identity。
4. Model catalog 刷新失败时可使用未过期 cache 或静态 fallback，但必须标记 freshness。
5. Unknown model capability 必须走统一 fallback 策略，不能由多个模块各自猜测。
6. 测试必须使用 fake backend 和 mock HTTP，不读取真实 secret。

## 验收标准

- [ ] 定义 credential backend trait 和 source precedence 表。
- [ ] 支持 env override、legacy plaintext import 和安全 backend save。
- [ ] 支持 logout/revoke，并返回恢复提示和结构化错误。
- [ ] 支持 provider/model catalog cache，包含 TTL、ETag、freshness 和 static fallback。
- [ ] 覆盖 credential precedence、revoke failure、offline catalog、remote merge 和 unknown capability 测试。

## 边界情况

- Env 和 saved credential 同时存在：使用 env，并标记 source=env。
- Secure backend 不可用：明确返回 unsupported 或受控 fallback，不静默写 plaintext。
- Catalog ETag 未变化：保留 cache 并更新 freshness metadata。
- Remote catalog 返回未知字段：按 schema 策略 fail closed 或记录兼容处理，不能污染 capability manager。

## 发布说明

本 PR 仅添加 GH-89 focused spec。实现 PR 需要说明凭据存储优先级、fallback 安全策略、catalog 缓存规则和 capability fallback 行为。
