# Task Plan

## Linked Issue

GH-129

## Spec Packet

- Product: `product.md`
- Tech: `tech.md`

## 实现任务

- [ ] `SP129-T01` Owner: tools-network. Done when: WebFetch/http client 使用 validated endpoint + IP pinning/remote_addr verification，校验 IP 与连接 IP 不一致时拒绝. Verify: `cargo test -p sage-tools network`。
- [ ] `SP129-T02` Owner: tools-network. Done when: redirect target 返回并使用 validated endpoint，redirect rebinding 被拒绝. Verify: `cargo test -p sage-tools redirect`。
- [ ] `SP129-T03` Owner: mcp. Done when: MCP trust store 持久化 server/tool/schema description hash，首次写 baseline、后续 drift warning/reject. Verify: `cargo test -p sage-core mcp trust`。
- [ ] `SP129-T04` Owner: mcp. Done when: description scanner 阻断高风险 override/authority claim 文本，跨 server 同名工具产生 warning 或拒绝 shadowing. Verify: `cargo test -p sage-core mcp registry`。

## 并行拆分

T01/T02 同在 network，串行；T03/T04 同在 MCP，可并行于 network 任务。

## 验证

- `cargo fmt --all -- --check`
- `cargo clippy -p sage-tools -p sage-core --all-targets -- -D warnings`
- `cargo test -p sage-tools network`
- `cargo test -p sage-core mcp`
- `python3 checks/check_workflow.py --repo <specrail> --spec-dir <repo>/specs/GH129`

## Handoff Notes

不要通过把 HTTPS URL 改写成 IP URL 来实现 pinning，除非同时保留原始 Host/SNI 和证书校验；否则会引入新的 TLS 风险。
