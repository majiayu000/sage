# Product Spec

## Linked Issue

GH-83

## 用户问题

Sage 的 CLI、SDK、非交互执行、resume/continue 和 `--stream-json` 已经存在，但入口仍直接耦合到当前 `UnifiedExecutor` 形态。没有稳定 runtime facade，后续 protocol、ThreadStore、服务端 handler 和自动化测试会重复接入执行循环，导致行为不一致。

## 目标

- 定义统一 runtime API facade，用同一入口处理 start/resume/fork/interrupt/status。
- 让 CLI 和 SDK 在保持现有用户语义的前提下逐步走同一 runtime boundary。
- 复用 GH-81 protocol events 和 GH-82 ThreadStore，而不是另起一套 DTO。
- 为后续 handler、contract tests 和 automation 提供可测试边界。

## 非目标

- 不做桌面 app。
- 不做 VS Code、Cursor、Windsurf 等 IDE 入口。
- 不做 app-server client。
- 不改变现有 CLI 参数语义，除非有显式迁移说明。
- 不在本 issue 中实现 ThreadStore schema；这是 GH-82。
- 不重写整个 execution loop。

## Behavior Invariants

1. `sage -p`、`sage -c`、`sage -r`、`--stream-json` 的外部行为保持兼容。
2. CLI 和 SDK 使用同一 runtime facade，不再各自构造不一致的 executor setup。
3. Runtime request/notification/response/error 必须引用 GH-81 的协议类型。
4. Runtime facade 可以先 wrap `UnifiedExecutor`，不能通过复制执行逻辑来制造第二个 loop。
5. Resume/continue/fork/status 必须经过同一 session/thread identity 解析规则。
6. Runtime errors 必须结构化返回；CLI 可以格式化展示，但不能吞掉错误后继续。
7. 任何不支持的 mode 必须明确 fail closed 或返回 unsupported error。

## 验收标准

- [ ] 新增 runtime facade，封装 start/resume/fork/interrupt/status。
- [ ] CLI 和 SDK 入口都经由该 facade 或有明确迁移 seam。
- [ ] 保持 `sage -p`、`sage -c`、`sage -r`、`--stream-json` 行为兼容。
- [ ] 增加 CLI/SDK contract 测试或快照测试。
- [ ] runtime facade 暴露协议 stream hook，但不强迫旧 API 立即破坏性迁移。

## 边界情况

- 没有 ThreadStore 时，runtime facade 可以使用 ephemeral thread，但必须显式记录能力状态。
- SDK interactive mode 的 input handle 必须继续工作，不能因为 facade 抽象丢失 request correlation。
- `--stream-json` 旧消费者必须继续能解析旧 JSONL，协议 stream 应作为 opt-in 或兼容映射。
- Interrupt/status 在底层 loop 尚不支持完整行为时，应返回结构化 unsupported，而不是假成功。

## 发布说明

本 PR 仅添加 GH-83 focused spec。实现 PR 需要说明 CLI/SDK 兼容性、旧 API 迁移策略和 contract test 证据。
