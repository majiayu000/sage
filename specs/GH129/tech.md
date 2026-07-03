# Tech Spec

## Linked Issue

GH-129

## Product Spec

`specs/GH129/product.md`

## Codebase Context

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| URL validation | `crates/sage-tools/src/tools/network/validation.rs` | `validate_url_security` uses `tokio::net::lookup_host` to reject private IPs | 校验与 reqwest 建连分离 |
| WebFetch | `crates/sage-tools/src/tools/network/web_fetch.rs` | calls `validate_url_security(url)` then `client.get(url).send()` | TOCTOU 入口 |
| redirect | `crates/sage-tools/src/tools/network/redirect.rs` | validates Location target string before caller makes a new request | redirect 同样需要绑定 |
| MCP registry | `crates/sage-core/src/mcp/registry.rs` | converts MCP tools to Sage tools and exposes descriptions | trust layer 接入点 |
| schema translator | `crates/sage-core/src/mcp/schema_translator/translator.rs` | sanitizes description types only | 需要内容信任检查 |

## 设计方案

1. **受信解析结果**：把 `validate_url_security` 拆成 `resolve_and_validate_url(url) -> ValidatedEndpoint { url, host, resolved_ips, selected_ip_policy }`。所有 resolved IP 必须通过现有 `is_private_ip`/metadata host 校验。
2. **连接 IP 绑定**：优先实现 reqwest 可支持的 IP pinning connector/resolver：请求建连只能使用 `ValidatedEndpoint.resolved_ips`。如果 reqwest 层无法可靠约束，改为连接后 remote_addr 复核并在响应 body 读取前拒绝不匹配连接。两种方案都必须覆盖 TLS Host/SNI 与 Host header，不能把 HTTPS 退化成 IP URL 后破坏证书校验。
3. **redirect 复用**：`validate_redirect_target` 返回 `ValidatedEndpoint`，调用方不能丢弃解析结果再重新校验字符串。redirect 后下一次请求使用同样的 pinned/verified endpoint。
4. **rebind 测试设施**：新增测试 resolver 或 local HTTP harness，第一次解析返回公网测试 IP，连接阶段返回 loopback/private IP，断言请求失败。redirect 测试也覆盖目标 rebinding。
5. **MCP trust baseline**：新增 `McpToolTrustStore`，按 server id + tool name + normalized description/input_schema 计算 SHA-256 hash，持久化到现有 MCP/config data dir 下。首次发现写 baseline；后续不同 hash 生成 drift event。
6. **description scanner**：新增静态扫描器，检测 override/ignore/system/developer authority claims 等短语。命中高风险时默认拒绝加载该 MCP tool；配置可切到 require-confirm/warn，但非交互执行不得 silently allow。
7. **tool name collision**：registry 聚合工具时检测跨 server 同名 tool。默认产生 warning 并使用 server-qualified internal id；如果当前 tool schema 不支持命名空间，则拒绝 shadowing 并要求用户显式选择 server。

## Product-to-Test Mapping

| Product invariant | Implementation area | Verification |
| --- | --- | --- |
| P1/P3 | validated endpoint + pinned/verified request | DNS rebinding unit/integration test |
| P2 | redirect endpoint flow | redirect rebinding test |
| P4 | `McpToolTrustStore` | first baseline write + drift warning tests |
| P5 | description scanner | override phrase fixture tests |
| P6 | registry collision detection | two servers same tool name test |

## 数据流

WebFetch URL -> resolve+validate endpoint -> pinned/verified HTTP request -> optional redirect returns validated endpoint -> response. MCP server discovery -> normalize tool schema/description -> trust store baseline/drift check -> description scanner -> collision detector -> registry exposes trusted or rejected tools.

## 备选方案

- 只缩短 DNS TTL 或重复校验：不能证明连接 IP 与校验 IP 一致，不满足 SSRF 目标。
- 只记录 MCP drift 不阻断 override 文本：对 prompt injection 风险偏弱，至少高风险 override 语言应 fail closed。

## 风险

- Security: IP pinning must preserve TLS certificate validation for original hostname.
- Compatibility: 一些合法 MCP description 可能触发 scanner，需要清晰的 override/confirm flow。
- Performance: trust baseline hash 与 DNS pinning 增加少量开销。
- Maintenance: reqwest connector 自定义复杂，需集中封装，避免每个网络工具重复实现。

## 测试计划

- [ ] Unit tests: IP validation、endpoint pin/verify mismatch、redirect mismatch。
- [ ] Integration tests: rebinding resolver/harness for WebFetch.
- [ ] MCP tests: baseline write/drift、override scanner、cross-server collision。
- [ ] Security regression: metadata/loopback/private/IPv4-mapped 既有测试保持通过。

## 回滚方案

可通过配置暂时将 MCP drift 从 reject 切到 warn；SSRF IP 绑定不应回滚为单次预解析校验。
