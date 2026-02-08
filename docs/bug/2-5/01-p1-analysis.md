# P1 问题完整分析与整改方案

## 1. ToolError -> SageError 归因丢失

**根因**
- `impl From<ToolError> for SageError` 在转换时无法携带工具名，导致默认 `unknown`。
- `ConfirmationRequired` 被硬编码到 `bash`。

**影响**
- 生产错误日志/用户提示不可信，影响问题定位。
- 遥测与权限策略归因错误。

**整改方案**
- 移除 `From<ToolError> for SageError`，禁止无工具名转换。
- 增加显式转换入口：`ToolError::into_sage(tool_name)` 或 `SageError::tool_from_error(tool_name, err)`。
- 为 `ToolError` 增加 `ToolErrorKind`，并在转换时保留错误类型。

**验收标准**
- 所有 `ToolError` 转 `SageError` 的场景必须显式提供 `tool_name`。
- 不再出现 `unknown` 或硬编码 `bash` 的错误归因。

---

## 2. 错误分类依赖字符串匹配

**根因**
- `classify_error` 使用 `message.contains(...)` 判断可重试/不可重试。
- 错误结构化信息缺失（HTTP 状态码、IO ErrorKind、LLM 错误类型）。

**影响**
- 文案调整会引发重试策略变更。
- 误分类导致重试风暴或错误提前终止。

**整改方案**
- 为 `SageError` 增加结构化错误类型：
  - `HttpErrorKind`（Status/Timeout/Connection/Dns/Other）
  - `LlmErrorKind`（RateLimit/Overloaded/Timeout/Network/Auth/InvalidRequest/ContextLength/QuotaExceeded/Other）
  - `ToolErrorKind`（与 `ToolError` 对应）
  - `Io` 记录 `std::io::ErrorKind`（可选）
- `classify_error` 仅基于结构化字段判断，不再使用字符串匹配。
- LLM provider 在 HTTP 状态码与传输错误时设置 `LlmErrorKind`。
- `reqwest::Error` 转换时设置 `HttpErrorKind`。

**验收标准**
- `classify_error` 中不出现任何字符串匹配。
- 结构化字段覆盖主要重试/不可重试场景。

---

## 3. 异步运行时中同步 I/O

**根因**
- `SettingsLoader` 提供同步 API，且内部使用 `std::fs`。
- 多处 async 逻辑直接调用同步 API。

**影响**
- 阻塞 Tokio runtime，影响并发与响应。

**整改方案**
- 移除同步 API，仅保留 async 版本：
  - `load_async`, `load_from_file_async`, `save_to_file_async`
- 统一调用方为 `.await`。

**验收标准**
- Settings 加载/保存不再使用 `std::fs`。
- async 场景无阻塞 I/O。

---

## 4. shell 执行路径默认拼接字符串

**根因**
- Bash 工具与后台任务通过 `bash -c` 拼接执行。

**影响**
- 命令注入风险；审计与权限控制困难。

**整改方案**
- Bash 工具 API 改为结构化 `argv: [string]`（必填）。
- 执行使用 `Command::new(argv[0]).args(&argv[1..])`，不再隐式使用 shell。
- 允许用户显式传入 `sh -c`，但不再默认拼接。
- 后台任务 `BackgroundShellTask::spawn` 同步升级为 argv 模式。

**验收标准**
- 生产路径不再默认使用 `bash -c`。
- 单元/集成测试通过。

---

## 测试计划
- `cargo test -p sage-core -p sage-tools`
- 重点覆盖：
  - SettingsLoader async 读写
  - Bash 工具执行与后台任务
  - recovery 分类逻辑

