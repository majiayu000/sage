# Product Spec

## Linked Issue

GH-82

## 用户问题

Sage 已有 JSONL session、trajectory、checkpoint 和 storage module，但 thread state 还没有统一为可查询、可迁移、可恢复的 ThreadStore。后续 resume、fork、子代理图、审计和搜索如果继续各自读写文件，会出现状态漂移、损坏数据难恢复、列表/搜索能力不一致的问题。

## 目标

- 定义持久化 `ThreadStore` 边界，承接 GH-81 的 thread/turn/item 协议。
- 为现有 JSONL/session/trajectory 数据建立可回填的索引模型。
- 支持 thread list/read/search/archive/delete、lineage 查询和 restart recovery。
- 提供迁移和损坏数据处理规则，让 #83 runtime facade 和 #84 子代理图能依赖同一状态层。

## 非目标

- 不移除现有 JSONL；第一阶段必须兼容并支持 backfill。
- 不引入远端云同步或多设备同步。
- 不实现 runtime facade；这是 GH-83。
- 不实现子代理图行为；这是 GH-84。
- 不改变 GH-81 protocol envelope。
- 不包含桌面 app、IDE 入口或 app-server client。

## Behavior Invariants

1. ThreadStore 以 `thread_id` 为主键，并能保存 GH-81 定义的 `turn_id`、`item_id` 和 lineage。
2. 现有 JSONL/session metadata 必须可以投影到 ThreadStore；迁移失败不能静默丢数据。
3. `archive` 不等于物理删除；默认 list/search 不显示 archived thread，但 read 可按 ID 访问。
4. 删除必须显式、可测试，并清楚区分 metadata 删除和 payload 文件删除。
5. 搜索和分页结果必须稳定排序，不能依赖文件系统遍历顺序。
6. 启动恢复必须能识别 incomplete turn、损坏 JSONL、缺失 metadata 和 schema version mismatch。
7. 错误必须结构化返回；不允许 warning 后返回空列表伪装成功。

## 验收标准

- [ ] 定义 `ThreadStore` trait 或等价接口：create/resume/append/flush/read/list/search/archive/delete。
- [ ] SQLite migration 覆盖 threads、turns/items metadata、lineage、archive 状态和 schema version。
- [ ] 支持从现有 JSONL/session metadata backfill，并保留源文件引用。
- [ ] 支持分页 list、ID read、text/metadata search、archive/unarchive/delete。
- [ ] 覆盖迁移、查询、损坏数据处理、并发 append 和重启恢复测试。

## 边界情况

- 旧 JSONL 只有 `session_id`，没有 `thread_id`：必须使用 GH-81 的兼容决策生成或映射。
- JSONL 中 tool call/result 缺 parent UUID：允许导入但标记为 partial lineage。
- SQLite 不可写：应 fail closed，并给出恢复提示，不回退到不可查询空状态。
- backfill 中遇到单条损坏记录：记录结构化错误并继续导入可恢复记录，最终返回 partial import 结果。

## 发布说明

本 PR 仅添加 GH-82 focused spec。后续实现 PR 需要说明迁移路径、兼容性、数据目录影响和回滚策略。
