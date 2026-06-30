# Product Spec

## Linked Issue

GH-81

## 用户问题

Sage 目前有 `--stream-json`、SDK unified execution、session/trajectory 和 UI event
pieces，但这些输出还不是一个稳定的 runtime protocol。CLI、SDK、未来会话索引、
子代理、权限交互和回放都需要同一套 thread/turn/item/request/notification/error
边界，否则后续 #82/#83/#84 会各自发明事件字段，造成兼容性和迁移风险。

## 目标

- 定义 Sage runtime protocol v0 的稳定 envelope、事件命名和 ID 关系。
- 覆盖 thread、turn、item、request、notification、permission 和 structured error。
- 明确现有 `--stream-json` 事件如何映射到 protocol notification，而不是直接破坏现有输出。
- 提供可审查的 schema/fixture 目标，后续实现 PR 用它做兼容测试。
- 明确 `session_id` 与 `thread_id` 的兼容决策，避免 #82 回填时出现双重身份模型。
- 给 #82 ThreadStore、#83 runtime API facade 和 #84 child-agent graph 提供前置协议合同。

## 非目标

- 不实现桌面 app。
- 不实现 VS Code、Cursor、Windsurf 等 IDE 入口。
- 不实现 app-server client。
- 不在 #81 中实现持久化 ThreadStore；这是 #82。
- 不在 #81 中把 CLI/SDK 全量改到 runtime facade；这是 #83。
- 不在 #81 中实现子代理线程图、后台消息或 context fork；这是 #84/#85。
- 不在 #81 中重写权限策略或 sandbox 决策路径；这是 #88。
- 不默认改变当前 `--stream-json` wire format；先提供兼容映射和测试。

## Behavior Invariants

1. Protocol envelope 必须包含 `protocol_version`、`kind`、`type`、`id`、`timestamp`
   和稳定 correlation 字段；新增字段必须是向后兼容的 optional 字段。
2. `thread_id` 标识一次可恢复会话，`turn_id` 标识一次用户输入到执行完成的回合，
   `item_id` 标识消息、工具调用、权限请求、错误或结果等可索引对象。
3. `request` 表示客户端到 runtime 的命令，`notification` 表示 runtime 到客户端的事件，
   `response` 表示 request 完成，`error` 表示结构化失败。
4. Permission request/result 必须能表达 tool name、risk、reason、decision 和 rule source，
   但不得要求在协议层暴露 secrets。
5. Error payload 必须有稳定 `code`、human-readable `message` 和可选 redacted `details`；
   不允许只输出 free-form string。
6. 现有 `OutputEvent` 和 `ExecutionEvent` 必须能被映射到 v0 notification，
   但 #81 不要求移除或重命名旧事件。
7. JSON key 使用 `snake_case`；对外 schema 不引入未声明字段或 Any-style public API。
8. `thread_id` 是 runtime protocol 的主标识；现有 `session_id` 只有在与 thread 一对一时才能复用，
   否则必须保存在 `legacy_session_id` 或 redacted metadata 中供 #82 回填。
9. Protocol fixture 不包含 App/IDE/app-server-client 字段，不把 UI surface 当作 runtime 必需能力。
10. 文档、issue 和 PR 标题不得包含用户指出的 forbidden typo。

## 验收标准

- [ ] 定义 `sage.runtime.v0` envelope、request、notification、response 和 error 类型。
- [ ] 定义 thread lifecycle：`thread.start`、`thread.resume`、`thread.fork`、`thread.started`、`thread.ended`。
- [ ] 定义 turn lifecycle：`turn.start`、`turn.steer`、`turn.interrupt`、`turn.started`、`turn.completed`。
- [ ] 定义 item notifications：message、tool call start/result、permission request/result、error、result。
- [ ] `--stream-json` 现有事件有明确兼容映射，旧消费者不被破坏。
- [ ] 提供 JSON Schema、runtime stream、legacy stream mapping、permission roundtrip 和 structured error fixtures。
- [ ] 增加事件映射测试计划，覆盖每个当前 `OutputEvent`、核心 `AgentEvent`、`InputRequestDto`/`InputResponseDto`
  和 `ExecutionOutcome` terminal variant。
- [ ] 明确 #82/#83/#84 的依赖边界，避免把 store/API facade/child graph 塞进本 issue。

## 边界情况

- 没有 session recorder 时仍应能生成 `thread_id` 或说明 thread is ephemeral。
- 一次 turn 可能产生多个 assistant message chunk、多个 tool call 和多个 permission request。
- Tool result 可能成功但输出被截断；协议必须能表达 truncation/redaction metadata。
- 用户取消、interrupt、max steps 和 model/provider error 应使用不同 error/terminal event code。
- SDK interactive execution 可能在另一个 task 中响应 input request；correlation 必须靠 `request_id`，
  不能依赖 stdout order。
- Future ThreadStore backfill 必须能从现有 JSONL/trajectory 推导 protocol item，不要求丢弃旧数据。

## 发布说明

本 PR 仅添加 #81 focused spec 和 protocol fixture 草案，无 runtime 行为变化。实现 PR 需要在保持
当前 `--stream-json` 兼容的前提下落地 schema/types 和映射测试。
