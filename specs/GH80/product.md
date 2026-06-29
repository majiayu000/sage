# Product Spec

## Linked Issue

GH-80

## 用户问题

Sage 已经有 CLI、SDK、工具、会话恢复、MCP、skill、plugin 和 sandbox 雏形，但缺少一份经过 issue 化的能力差距地图。没有这份地图，后续容易把 runtime、state、多代理、扩展、安全和运维成熟度混在一个不可审查的大改里。

## 目标

- 把 Sage 与参考 runtime 的能力差距拆成可跟踪的 GitHub issues。
- 明确桌面 app、IDE 入口和 app-server client 不在本轮范围内。
- 为后续实现提供 SpecRail packet、优先级、依赖顺序和验收边界。
- 保留 human gates：本 PR 只提出 roadmap/spec，不授权实现、最终审批或 merge。

## 非目标

- 不实现桌面 app。
- 不实现 VS Code、Cursor、Windsurf 等 IDE 入口。
- 不实现 app-server client。
- 不在本 PR 中修改 runtime 代码。
- 不把子 issue 直接标记为 `ready_to_implement`。

## Behavior Invariants

1. Roadmap 必须列出 #80-#91 的完整 issue map，并把每个子 issue 归入 runtime/state/API、多代理/扩展/MCP、安全/认证/运维之一。
2. Roadmap 必须明确排除桌面 app、IDE 入口和 app-server client。
3. 每个子 issue 必须有用户问题、期望结果、非目标和验收标准。
4. P0 runtime/state/API 工作必须排在多代理、扩展和运维成熟度之前。
5. SpecRail packet 必须链接 GH-80，并用稳定 task ID 描述后续执行顺序。
6. 本 PR 不得声明 implementation ready、final approval、merge authorization 或 release readiness。
7. 文档和 issue/PR 标题不得包含用户指出的 forbidden typo。

## 验收标准

- [ ] `docs/analysis/sage-runtime-capability-roadmap-2026-06-30.md` 存在，并包含 scope、evidence、issue map、feature matrix、recommended order 和 SpecRail status。
- [ ] `specs/GH80/product.md`、`specs/GH80/tech.md`、`specs/GH80/tasks.md` 存在，并链接 GH-80。
- [ ] GitHub issues #80-#91 均存在且 open。
- [ ] App/IDE/app-server-client 被明确标为 out of scope 或 not applicable。
- [ ] Forbidden typo 字符串检查为 0 命中。

## 边界情况

- Sage 某些能力是 partial，而不是 missing；roadmap 必须保留 partial 状态，不能把已有能力当作不存在。
- Sage 当前没有完整 SpecRail workflow pack；验证应只校验本 PR 新增的 spec packet，而不是要求 Sage vendored 整个 workflow pack。
- 如果后续 maintainer 不接受某个子 issue 的优先级，应只调整该 issue，不重写整个 roadmap。

## 发布说明

这是 planning/spec 变更，无用户可见 runtime 行为变化。后续实现 PR 需要分别提供迁移说明、兼容性说明和测试证据。
