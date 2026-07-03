# Product Spec

## Linked Issue

GH-130

## 用户问题

仓库有四个低风险但容易在未来变成陷阱的卫生项：release tar 提取未防 symlink/linkname 逃逸，`redacted_context` 命名与构造时内容不一致，多路径 preflight 只附着第一个路径，路径权限 glob 在大小写不敏感文件系统上使用 case-sensitive 匹配。

## 目标

- tar 提取拒绝 symlink/hardlink 逃逸或使用安全 data filter。
- `redacted_context` 字段名与实际内容一致。
- 多路径 filesystem preflight 行为显式化并有守卫。
- 路径权限 glob 的大小写语义修复或明确声明。

## 非目标

- 不重构权限匹配架构。
- 不改变 release gate 的 artifact 发现流程。
- 不解决 #125 的完整权限系统收敛。

## Behavior Invariants

1. tar archive 中 symlink/hardlink/member path 不能逃逸目标目录。
2. 名为 `redacted_context` 的字段在被外部读取前必须已经脱敏；如果存放原文，字段必须改名。
3. 多路径 filesystem input 不得只对 index 0 应用 deny preflight，除非代码显式拒绝多路径工具。
4. 路径 glob 匹配在大小写不敏感文件系统上不能因大小写变体绕过 deny 规则。
5. 四项改动互相独立，最小修复，不引入新的权限语法。

## 验收标准

- [ ] tar 提取传入 `filter="data"` 或显式拒绝 symlink/hardlink 成员。
- [ ] `redacted_context` 命名与内容一致。
- [ ] 多路径 preflight 行为有注释、守卫与测试。
- [ ] 路径大小写敏感性被修复或文档显式声明限制。

## 边界情况

- Python 版本不支持 `filter="data"`：显式检查 tar member type、linkname 与 resolved path。
- macOS 上大小写敏感卷：按实际 filesystem 探测或提供明确配置/文档，不用平台名盲判。
- 当前无多路径工具：测试可构造多路径 input builder fixture，防未来回归。

## 发布说明

开发者卫生修复：release gate archive extraction 更严格，诊断字段命名更准确，权限 preflight 与路径 glob 行为更清晰。

## 开放问题

- 大小写匹配采用实际 filesystem 探测还是平台默认？建议 helper 支持注入 case-sensitivity，生产按路径探测，测试直接注入。
