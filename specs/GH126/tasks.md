# Task Plan

## Linked Issue

GH-126

## Spec Packet

- Product: `product.md`
- Tech: `tech.md`

## 实现任务

- [ ] `SP126-T01` Owner: config. Done when: `UnifiedConfigLoader` 被封装为执行入口统一 facade，CLI/SDK/TUI/doctor/status 不再直接走旧凭据合并. Verify: `cargo test -p sage-core config::credential` + search check。
- [ ] `SP126-T02` Owner: tools. Done when: 新增统一工具过滤 helper，default tools 与 MCP tools 注册前都按 `settings.tools` 过滤，禁用工具不可见/不可调用. Verify: `cargo test -p sage-core settings` and `cargo test -p sage-tools default_tools`。
- [ ] `SP126-T03` Owner: config. Done when: `config.tools.enabled_tools` 被删除或只在 load 阶段派生到 `settings.tools`，生产代码只读一个权威视图. Verify: `rg -n "enabled_tools|settings\\.tools" crates` 人工确认。
- [ ] `SP126-T04` Owner: config. Done when: `ConfigPersistence` 接入真实写回流程，或删除模块/导出/tests 中的 public dead API. Verify: `cargo check --workspace --all-targets --all-features`。

## 并行拆分

T01 与 T02 可并行但最后需要一起验证所有入口；T03 依赖工具开关权威面决策；T04 独立。

## 验证

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p sage-core config settings`
- `cargo test -p sage-tools default_tools`
- `cargo check --workspace --all-targets --all-features`
- `python3 checks/check_workflow.py --repo <specrail> --spec-dir <repo>/specs/GH126`

## Handoff Notes

禁用工具必须在 schema 暴露前被过滤；不要只在执行时拒绝，否则模型仍会看到不可用工具。
