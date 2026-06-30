# Product Spec

## Linked Issue

GH-90

## 用户问题

Sage 缺少一个可审计、可红action、可用户确认的诊断反馈链路。当前诊断信息分散在 telemetry、settings、permission、sandbox 和 provider 错误里，用户很难生成安全的反馈包，也很难知道某个策略或配置来源为什么影响了运行。

## 目标

- 建立 opt-in feedback diagnostic bundle，默认不上传私人数据。
- 使用 bounded event ring 捕获本地诊断事件，避免无界内存增长。
- 复用或扩展 redaction engine，确保 secret/path/token 等敏感字段被处理。
- 引入 managed read-only config/policy source provenance。
- 提供 audit 日志，用于解释 permission、sandbox、provider 和 config 决策。

## 非目标

- 不默认上传用户私有数据。
- 不让 managed config 放宽用户/system safety。
- 不实现云端诊断后台或桌面 UI。
- 不包含桌面 app、IDE 入口或 app-server client。

## Behavior Invariants

1. Feedback bundle 生成和上传必须由用户显式同意。
2. Event capture 必须有容量上限和丢弃策略，不能无界增长。
3. Bundle 中的 secret、credential、token、cookie 和 provider key 必须 redacted。
4. Managed config 只能收紧或声明只读策略，不能绕过更高优先级 deny。
5. Policy denial 必须能指出来源和匹配规则。
6. Unknown managed config field 必须严格失败或明确兼容策略，不能静默忽略安全字段。

## 验收标准

- [ ] 本地诊断事件进入 bounded ring buffer，并记录 dropped count/freshness。
- [ ] Feedback bundle 包含 doctor、config、proxy/provider、sandbox 和 permission 摘要。
- [ ] Bundle 生成前执行 redaction，并需要用户 consent。
- [ ] Managed config 有 strict schema、只读来源和 precedence 规则。
- [ ] Audit 输出包含 policy source、decision reason 和 redacted context。
- [ ] 覆盖 redaction、ring capacity、strict schema、precedence 和 audit capture 测试。

## 边界情况

- Ring buffer 满：丢弃最旧或按策略采样，并记录 dropped count。
- Redaction 无法确认某字段安全：默认 redacted。
- Managed config 与用户 allow 冲突但 system deny 更高：最终 deny，并显示来源链。
- 用户拒绝生成反馈包：不写 bundle、不上传，并记录本地取消状态。

## 发布说明

本 PR 仅添加 GH-90 focused spec。实现 PR 需要说明诊断 consent、redaction 范围、managed config precedence 和 audit retention 策略。
