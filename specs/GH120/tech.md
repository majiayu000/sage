# Tech Spec

## Linked Issue

GH-120

## Product Spec

`specs/GH120/product.md`

## Codebase Context

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| 决策引擎规则匹配 | `crates/sage-core/src/permissions/decision_engine.rs` | allow/deny 用 `permission_pattern_matches` 逐 key 匹配 | 匹配注入点 |
| 匹配 key 辅助 | `crates/sage-core/src/permissions/decision_engine_keys.rs` | 纯文本/路径/URL glob | 新增 bash 感知包装 |
| bash 权限 key | `crates/sage-core/src/agent/unified/settings_permission_keys.rs` | key = 原始命令串 | 输入形态 |

## 设计方案

新增 `permissions/shell_safety.rs`：
- `contains_shell_control_metachar`：检测 `&& || ; | & $( \` > < >> << \n \r`。
- `command_segments`：把命令按分隔符切成候选段（整串 + 各链段 + 替换体 + 重定向余段），并剥离前置 `VAR=val` 赋值。
- `is_partial_wildcard_pattern`：`git *` 为真，`*` 为假。

在 `decision_engine_keys.rs` 增加两个包装并在 `decision_engine.rs` 的 allow/deny 匹配处替换：
- `bash_aware_allow_matches`：先走原匹配；若 pattern 为部分通配且命令含控制元字符则拒绝该 allow（降级默认行为）。
- `bash_aware_deny_matches`：先走原匹配；否则对每个 command segment 重新以 `Bash(segment)` 匹配 deny，任一命中即 deny。

只影响 Bash key（`bash_key_tool/argument` 守卫），其他工具匹配不变。

## Product-to-Test Mapping

| Product invariant | Implementation area | Verification |
| --- | --- | --- |
| P1/P6 | `bash_aware_allow_matches` | `settings_permission_shell_tests::test_wildcard_allow_does_not_match_chained_command`、`test_unattended_chained_command_is_not_auto_allowed` |
| P2 | 原匹配保留 | `test_exact_allow_still_matches_full_command` |
| P3 | 全信任短路 | `test_full_trust_allow_still_matches_chained_command` |
| P4/P5 | `bash_aware_deny_matches` + `command_segments` | `test_deny_matches_chained_command_segment` |
| 单元 | shell_safety helpers | `shell_safety::tests` |

## 数据流

输入：Bash key（`Bash(<raw command>)`）。匹配层把命令拆段/检测元字符后再与规则比对；输出：Allow/Deny/Ask 决策，无持久化。

## 备选方案

- 在 bash 工具执行层强制元字符拦截：已存在但仅在配置了 allowlist 时生效，生产 allowlist 恒空，故必须在 settings 决策层修复。
- 完整 shell parser（shlex/tree-sitter-bash）：更精确但引入依赖与复杂度，且本类攻击用分段+元字符检测即可闭合，拒绝（U-06）。

## 风险

- Security: 修复 allow 逃逸与 deny 绕过（核心目标）。
- Compatibility: allow-list 下复合命令语义收紧；已在发布说明声明。
- Performance: 每次 Bash 决策多一次字符串扫描/切分，可忽略。
- Maintenance: 独立模块 + 单测，元字符集合与 bash 工具 guard 注释交叉引用。

## 测试计划

- [ ] Unit tests: `shell_safety` 单测 + `settings_permission_shell_tests` 5 项。
- [ ] Integration tests: workspace lib 测试回归（2055 passed）。
- [ ] Manual verification: 配置 `allow:["Bash(git *)"], default:deny`，尝试 `git status && id`，确认被拦。

## 回滚方案

revert 单一 commit（新增文件 + 两处匹配包装替换）。
