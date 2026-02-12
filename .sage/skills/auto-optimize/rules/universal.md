# 通用 NEVER 规则（适用于所有语言）

每条规则对应一个已验证的 LLM 代码生成失败模式。
来源标注：学术论文编号 / OWASP CWE / 实战案例。

## 架构完整性

- **U-01**: NEVER duplicate a type/class/interface definition across modules
  - 来源: 局部最优陷阱
  - 检测: 同名 class/struct/interface/type 跨文件出现 >1 次

- **U-02**: NEVER re-implement existing functionality without searching codebase first
  - 来源: 上下文断裂分叉
  - 检测: AI 语义分析（需子 agent）

- **U-03**: NEVER break module boundary contracts — no cross-layer calls, no bypassing abstractions
  - 来源: 架构无感（endorlabs 研究：15/20 AI 补全含架构缺陷）
  - 检测: 依赖图分析

- **U-04**: NEVER add backward-compatibility shims — modify directly, bump version
  - 来源: 保守性退化
  - 检测: grep deprecated/alias/shim

- **U-05**: NEVER leave dead code — no commenting out, no allow(dead_code), delete it
  - 来源: 保守性退化
  - 检测: lint dead_code 警告 + grep 注释块

## 安全边界

- **U-06**: NEVER hardcode credentials/secrets — use env vars or secret manager
  - 来源: OWASP CWE-798
  - 检测: grep password/secret/api_key 字面量赋值

- **U-07**: NEVER trust user input without validation — all external input must be sanitized
  - 来源: OWASP CWE-20（40%+ AI 代码缺少输入验证）
  - 检测: 追踪 request/input 到使用点的数据流

- **U-08**: NEVER use shared secrets across services — each service gets its own key
  - 来源: Cross-Service Trust Coupling（endorlabs）
  - 检测: 配置审计

- **U-09**: NEVER assign elevated privileges by default — least privilege principle
  - 来源: Privilege Escalation by Default（endorlabs）
  - 检测: 权限分配审计

- **U-10**: NEVER skip audit logging for destructive operations
  - 来源: Missing Accountability（endorlabs）
  - 检测: grep delete/remove/drop 路径无 log 调用

## 错误处理

- **U-11**: NEVER silently swallow errors — no empty catch, no `let _ =`, no fake Ok
  - 来源: 68.5% LLM 代码有逻辑错误（arxiv 2503.06327）
  - 检测: grep empty catch/silent discard 模式

- **U-12**: NEVER use catch-all exception handlers in business logic
  - 来源: Error Handling Sprawl（vibe coding 研究）
  - 检测: grep broad catch/except

- **U-13**: NEVER return success from unimplemented functions — use todo!/raise NotImplementedError
  - 来源: Stub Ok(()) 实战案例
  - 检测: AI 审计函数体 vs 签名承诺

## 代码质量

- **U-14**: NEVER copy-paste code blocks — extract shared function/module
  - 来源: 21.14% LLM 代码有重复（arxiv 2503.06327）
  - 检测: 重复检测工具 / AI 语义分析

- **U-15**: NEVER leave TODO/FIXME without tracking — must link to issue/task
  - 来源: 保守性退化
  - 检测: grep TODO/FIXME 无 URL/issue 编号

- **U-16**: NEVER exceed file size limit (200 lines code, 300 lines tests)
  - 来源: 可维护性
  - 检测: wc -l

## 依赖管理

- **U-17**: NEVER add dependencies without checking existing alternatives in project
  - 来源: Dependency Explosion（endorlabs）
  - 检测: 依赖数量变化追踪

- **U-18**: NEVER use deprecated/unmaintained libraries
  - 来源: Stale Library Suggestions（endorlabs）
  - 检测: 依赖审计工具

- **U-19**: NEVER pin to moving version aliases in production — lock exact versions
  - 来源: No Model Version Pinning（arxiv 2512.18020，36% 系统）
  - 检测: 版本锁文件检查

## LLM 集成特有

- **U-20**: NEVER leave token/timeout/retry unbounded — set explicit limits
  - 来源: Unbounded Max Metrics（arxiv 2512.18020，38% 系统）
  - 检测: grep API 调用无 max_tokens/timeout

- **U-21**: NEVER omit system message in LLM API calls
  - 来源: No System Message（arxiv 2512.18020，34.5% 系统）
  - 检测: grep chat/completion 调用无 system

- **U-22**: NEVER accept free-form LLM output where structured output is expected — use JSON schema
  - 来源: No Structured Output（arxiv 2512.18020，40.5% 系统）
  - 检测: grep LLM 响应解析逻辑

- **U-23**: NEVER rely on implicit temperature defaults — set explicitly
  - 来源: TNES（arxiv 2512.18020，36.5% 系统）
  - 检测: grep API 调用无 temperature
