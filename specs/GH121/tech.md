# Tech Spec

## Linked Issue

GH-121

## Product Spec

`specs/GH121/product.md`

## Codebase Context

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| 单工具执行编排 | `crates/sage-core/src/agent/unified/step_execution.rs` | `execution_tool_call` 仅被 settings Allowed 分支更新；post_execution/record 用旧 call | 可观测层失真的消费点 |
| destructive 确认 | `crates/sage-core/src/agent/unified/step_execution_permissions.rs` | `confirmed_call` 是局部变量，返回值只有 `ToolResult` | 编辑丢失的根因 |

## 设计方案

把「实际执行的 call」沿调用链返回：

1. `execute_with_permission_check` 返回 `(ToolResult, ToolCall)`，所有 return 点都附带 `without_user_confirmation_marker` 处理后的当前/确认 call。
2. `execute_tool_phase`、`pre_execute_or_block` 返回 `SageResult<(ToolResult, ToolCall)>`；交互/AskUserQuestion/hook-block 路径返回原 call clone。
3. `execute_single_tool` 在 post_execution 与 record 前用返回的 executed call 覆盖 `execution_tool_call`。
4. 确认中发生编辑（`input_modified`）时，在执行前对确认后的 call 调用 `track_file_for_undo`（Ready 分支与重询循环分支都覆盖）。

## Product-to-Test Mapping

| Product invariant | Implementation area | Verification |
| --- | --- | --- |
| P1/P2/P3 | `execute_with_permission_check` 返回值 | `test_destructive_confirmation_edit_propagates_executed_call`（FakeDestructiveBash 复刻 bash 确认契约） |
| P4 | Ready/NeedsDestructiveConfirmation 分支 track_file_for_undo | 代码审查（track 语义依赖 session_manager，unit 层难以直接断言） |
| P5 | 既有测试回归 | `cargo test -p sage-core --lib`（2045 passed） |

## 数据流

输入：LLM 工具调用 + 用户确认响应（可含 modified_input）。输出：ToolResult + 实际执行 call → post hooks、session record、undo tracker。

## 备选方案

- `&mut ToolCall` 出参贯穿调用链：可行，但可变借用跨 await 更易碎；返回元组更显式，拒绝。

## 风险

- Security: 修复审计失真，只收紧。
- Compatibility: 仅 crate 私有方法签名变化，无公共 API 影响。
- Performance: 每次执行多一次 ToolCall clone，可忽略。
- Maintenance: 返回值语义有文档注释。

## 测试计划

- [ ] Unit tests: 新增传播回归测试；既有 destructive/settings 测试回归。
- [ ] Integration tests: workspace 测试回归。
- [ ] Manual verification: 交互模式编辑 destructive 命令后检查会话记录。

## 回滚方案

revert 单一 commit。
