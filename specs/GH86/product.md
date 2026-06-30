# Product Spec

## Linked Issue

GH-86

## 用户问题

Sage 目前已有 skills、MCP、hooks、commands 等能力入口，但缺少一个统一的 extension package 生命周期。用户无法用一个 manifest 安全地发现、安装、校验、启用、禁用和卸载一组扩展资产。

## 目标

- 定义 manifest 驱动的 extension package 格式。
- 支持 package discover、list、read、install、uninstall、enable、disable 生命周期。
- 允许 package 声明 skills、MCP servers、hooks 和 commands。
- 注册时保留 package/source metadata，便于审计和冲突排查。
- 对路径、依赖和权限声明做 fail-closed 校验。

## 非目标

- 不做图形化扩展商店。
- 不执行未启用或未通过校验的 package 代码。
- 不实现 MCP runtime 认证和延迟发现；这是 GH-87。
- 不放宽 GH-88 permission profile。
- 不包含桌面 app、IDE 入口或 app-server client。

## Behavior Invariants

1. Package manifest 必须有版本化 schema 和严格校验。
2. 所有 package 文件访问必须限制在 package root 内。
3. 未启用 package 不得注册 skills、MCP、hooks 或 commands。
4. Package 注册和卸载必须是可逆的，不留下幽灵 registry entry。
5. 缺失依赖、未知字段、路径逃逸、权限声明缺失必须 fail closed。
6. Registry 冲突必须返回结构化错误，不能静默覆盖。

## 验收标准

- [ ] 支持 extension manifest v0，声明 package metadata、assets、dependencies 和 permissions。
- [ ] 支持 discover/list/read/install/uninstall/enable/disable API。
- [ ] 启用 package 时注册 skills、MCP servers、hooks 和 commands。
- [ ] 禁用或卸载 package 时撤销对应注册。
- [ ] 覆盖 manifest fixture、path escape、missing dependency、disabled package 和 registry conflict 测试。

## 边界情况

- 两个 package 声明同名 command：返回冲突错误并保持现有 registry 不变。
- Package manifest 合法但 asset 文件缺失：安装或启用失败，错误指出缺失路径。
- Disable 过程中部分 registry 撤销失败：返回 structured partial failure，并保留可重试状态。
- Package 更新改变权限声明：必须重新校验并要求显式 enable/accept 流程。

## 发布说明

本 PR 仅添加 GH-86 focused spec。实现 PR 需要说明 manifest 兼容策略、package 存储位置、registry 冲突规则和安全边界。
