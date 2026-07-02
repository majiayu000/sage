# Tech Spec

## Linked Issue

GH-130

## Product Spec

`specs/GH130/product.md`

## Codebase Context

| Area | Files | Current behavior | Why relevant |
| --- | --- | --- | --- |
| release archive extraction | `scripts/release_gate.py` | validates member path then calls `tarfile.extractall(dest)` without `filter="data"` | symlink/hardlink linkname escape risk |
| diagnostics bundle | `crates/sage-core/src/diagnostics/bundle.rs` | `PolicyAuditSummary.redacted_context` exists; redaction also happens later in `redact_sections` | naming/content contract unclear |
| preflight inputs | `crates/sage-core/src/agent/unified/settings_permission_inputs.rs` | multi-path filesystem branch attaches preflights only to index 0 | future bypass trap |
| permission glob | `crates/sage-core/src/tools/permission/cache.rs` | `MatchOptions { case_sensitive: true }` | deny bypass on case-insensitive FS |

## 设计方案

1. **tar extraction**：对 tar members 显式 reject 非 regular file/directory，尤其 `issym()`/`islnk()`；校验 `member.name` 和 linkname resolved path。若 minimum Python 支持，调用 `extractall(dest, filter="data")`，同时保留手动校验作为 defense in depth。
2. **diagnostic field contract**：优先保持字段名 `redacted_context`，确保所有构造点写入前已经使用 `DiagnosticRedactor`。如果有调用方需要原文，则新增 `raw_context` 内部字段并禁止序列化到 bundle。
3. **multi-path preflight**：修改 input builder，让每个 path 都携带 deny preflight 和 scoped allow，或在发现 `paths.len() > 1` 时 debug/assert 并返回 error。优先每路径附着，行为更符合未来多路径工具。
4. **case sensitivity helper**：抽出 `path_glob_match_options(case_sensitive: bool)` 或 matcher struct。生产根据 workspace/path 所在 filesystem 探测大小写敏感性；无法探测时 Windows 默认 insensitive，Unix 默认 sensitive，但 macOS 记录 warning 或通过配置覆盖。测试直接注入 false，证明 deny 大小写变体能命中。
5. **最小文档**：如果不能在本轮可靠探测 filesystem，则在 permission docs 中显式声明 case-sensitive 限制，并开 follow-up；但首选实现修复。

## Product-to-Test Mapping

| Product invariant | Implementation area | Verification |
| --- | --- | --- |
| P1 | release gate extraction | tar symlink/hardlink escape fixture test |
| P2 | diagnostics bundle | `redacted_context` 构造与 serialization 测试 |
| P3 | settings_permission_inputs | two-path fixture 中每个 path 都有 deny preflight |
| P4 | permission cache glob | insensitive matcher deny case variant test |
| P5 | docs/no scope creep | diff review: 不改权限语法 |

## 数据流

release archive -> member safety/type/link checks -> extract. Diagnostics event -> redactor -> `PolicyAuditSummary.redacted_context` -> bundle serialization. Tool call paths -> per-path `PermissionDecisionInput` with preflights. Permission cache key -> path glob matcher with filesystem-aware case sensitivity.

## 备选方案

- 只加注释不改 multi-path behavior：当前无多路径工具时风险低，但不能防未来扩展。
- 只文档化 case-sensitive glob：满足最低验收但留下实际 bypass，建议仅作为探测不可行时的 fallback。

## 风险

- Security: tar extraction must fail closed on unknown tar member types.
- Compatibility: stricter archive extraction may reject unusual artifacts; release artifacts应只含普通文件/目录。
- Maintenance: filesystem case-sensitivity detection must be testable and not tied to developer machine state.

## 测试计划

- [ ] Python tests or script fixture: tar symlink/hardlink escape rejected.
- [ ] Rust unit tests: diagnostic redaction, multi-path preflight, path glob case folding.
- [ ] Manual search: no new permission syntax or broad refactor.

## 回滚方案

四项可独立回滚；tar safety and Transform-like fail-closed behavior should not be relaxed without replacement mitigation.
