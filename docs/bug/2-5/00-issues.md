# Sage 设计问题记录（2-5）

## P1（必须优先修复）
1. **ToolError -> SageError 归因丢失**
   - 现象：`ToolError` 转 `SageError` 时工具名被标成 `unknown`，`ConfirmationRequired` 甚至硬编码为 `bash`。
   - 影响：错误追踪、用户提示、遥测归因失真；权限/确认逻辑可能被错误关联。
   - 位置：`crates/sage-core/src/tools/base/error.rs`.

2. **错误分类依赖字符串匹配**
   - 现象：`classify_error` 通过 `message.contains(...)` 识别可重试/不可重试。
   - 影响：消息格式变更导致重试策略失效或错误分类。
   - 位置：`crates/sage-core/src/recovery/mod.rs`.

3. **异步运行时中存在同步 I/O**
   - 现象：SettingsLoader 使用 `std::fs`，在 async 调用中阻塞运行时。
   - 影响：吞吐下降、卡顿、超时放大。
   - 位置：`crates/sage-core/src/settings/loader.rs`.

4. **shell 执行路径默认拼接字符串**
   - 现象：Bash 工具/后台任务使用 `bash -c` 执行用户输入。
   - 影响：命令注入风险、审计困难。
   - 位置：`crates/sage-tools/src/tools/process/bash/execution.rs`, `crates/sage-core/src/tools/background_task.rs`.

## P2（下个迭代）
1. SDK 与 core 耦合过深：SDK 直接暴露 `sage_core::Config` 并吞掉 `Default` 错误。
2. 工具参数抽取不一致：已有 `require_*` 却仍大量手写 `get_string().ok_or_else(...)`。
3. `sage-tools` 仍大量使用 `anyhow`，与 `ToolError` 体系混用。
4. `sage-core` 过重，缺少 feature flag/裁剪能力。

## P3（技术债）
1. 超大文件过多（>500 行），职责不清，重构成本高。

