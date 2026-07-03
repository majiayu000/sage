# Product Spec

## Linked Issue

GH-129

## 用户问题

WebFetch/http client 的 SSRF 校验先解析并检查 IP，随后 reqwest 建连时再次 DNS 解析，存在 DNS rebinding/TOCTOU 间隙。MCP 工具 description 和参数描述直接暴露给 LLM，没有基线 hash、漂移检测、override 语言扫描或跨 server 同名工具冲突告警，存在 tool poisoning 和 shadowing 风险。

## 目标

- WebFetch/http client 在校验过的 IP 与实际连接 IP 之间建立强绑定。
- redirect 路径也消除校验-使用间隙。
- MCP tool description 建立信任层：首连 hash 基线、重连漂移告警、override/authority-claim 扫描、跨 server 同名工具告警。

## 非目标

- 不引入完整 MCP 权限清单系统。
- 不改变现有 URL 编码、私网 IP、metadata host 校验语义。
- 不把所有网络工具重写为新 HTTP stack，除非 tech spec 证明 reqwest 无法满足 IP 绑定。

## Behavior Invariants

1. SSRF 校验使用的 IP 与实际 socket 连接 IP 一致，或连接后复核 remote_addr 并在不一致时拒绝响应。
2. redirect target 必须经过同样的 IP 绑定/复核，不得只校验 Location 字符串。
3. DNS 解析失败、IP 绑定失败、remote_addr 复核失败都返回 error，不静默 fallback。
4. MCP tool description 首次加载时写入 server/tool/schema hash 基线；后续变化产生 warning 或拒绝，取决于配置。
5. 含 override、ignore previous instructions、claim authority 等高风险语言的 MCP description 默认拒绝加载或要求确认。
6. 不同 MCP server 暴露同名 tool 时产生明确冲突告警，并在 schema 中保留 server 命名空间或拒绝 shadowing。

## 验收标准

- [ ] WebFetch/http_client 消除 DNS 二次解析间隙，含 redirect 路径。
- [ ] 有 DNS rebinding 模拟测试证明校验 IP 与连接 IP 不一致时拒绝。
- [ ] MCP description 有 hash 基线持久化与变更告警。
- [ ] 高风险 override 语言被拒绝加载或要求确认。
- [ ] 跨 server 同名工具冲突有告警或拒绝策略。

## 边界情况

- 域名解析多个公网 IP：允许连接解析集合内的公网 IP，但不能连接校验后新增的私网 IP。
- IPv6、IPv4-mapped、metadata 地址继续走现有私网/metadata 判定。
- MCP server 首次连接无基线：创建基线并记录来源；非交互环境按配置默认 fail closed 或 warning。

## 发布说明

安全加固：WebFetch 对 DNS rebinding 更严格；MCP 工具 description 变化和同名冲突会产生告警或阻断。

## 开放问题

- MCP description 漂移默认是 warn 还是 reject？建议本轮默认 warn + 可配置 reject，高风险 override 文本默认 reject。
