# Product Spec

## Linked Issue

GH-123

## 用户问题

`/model` 拉取实时模型列表失败时静默回退静态列表，用户无法察觉凭据或网络故障，也可能看不到新发布的模型。

## 目标

- 拉取失败时留下可观测证据（provider 与错误原因）。
- 保持现有回退行为不变（仍展示静态列表）。

## 非目标

- 不接线 ModelCatalogManager（见 GH-124）。
- 不改变模型选择 UI。

## Behavior Invariants

1. 任一 provider 的实时模型拉取失败必须产生一条含 provider 名与错误内容的 warn 级日志。
2. 拉取失败后仍回退到静态模型列表，`/model` 不因网络失败而中断。
3. 拉取成功路径行为不变。

## 验收标准

- [ ] 三个 fetch 分支（anthropic 系、openai 系、ollama）失败时均输出 `tracing::warn!`。
- [ ] 回退列表内容与修复前一致。

## 边界情况

- provider 无静态列表（`provider_info` 为 None）：回退为空列表并走已有 "No models available" 提示。

## 发布说明

无迁移或兼容性影响；仅新增日志。
