# Naming Convention Migration Plan

**Project:** Sage Agent
**Document Version:** 1.0
**Created:** 2025-12-23
**Target Version:** 0.2.0 (deprecation) → 1.0.0 (removal)

## Executive Summary

This document outlines the migration plan for aligning the Sage Agent codebase with Rust naming conventions, specifically addressing acronym handling in type names. The plan follows a gradual deprecation strategy to minimize disruption to users while improving code consistency.

**Current State:** 15 types using all-caps acronyms (LLM, SSE, MCP, CLI)
**Target State:** All acronyms follow PascalCase convention (Llm, Sse, Mcp, Cli)
**Breaking Change:** Yes, but mitigated through type aliases and deprecation period

## Background

According to [Rust API Guidelines (RFC 430)](https://rust-lang.github.io/rfcs/0430-finalizing-naming-conventions.html), acronyms in type names should be treated as words and use PascalCase throughout:

- ✅ **Correct:** `HttpClient`, `JsonFormatter`, `LlmProvider`
- ❌ **Incorrect:** `HTTPClient`, `JSONFormatter`, `LLMProvider`

This convention is consistently followed across the Rust ecosystem (tokio, serde, reqwest, etc.) and improves readability and consistency.

## Current State Analysis

### Types Requiring Migration

Based on the comprehensive audit in `docs/naming-convention-audit.md`, the following 15 types need renaming:

#### LLM-prefixed Types (10 instances)

1. **LLMUsage** → **LlmUsage**
   - Location: `crates/sage-core/src/types.rs`
   - Impact: Public API, used for token usage statistics
   - Status: ✅ **COMPLETED** - Renamed with deprecated alias

2. **LLMClient** → **LlmClient**
   - Location: `crates/sage-core/src/llm/client.rs`
   - Impact: Core client structure, widely used
   - Status: ✅ **COMPLETED** - Renamed with deprecated alias

3. **LLMMessage** → **LlmMessage**
   - Location: `crates/sage-core/src/llm/messages.rs`
   - Impact: Core message type, widely used
   - Status: ✅ **COMPLETED** - Renamed with deprecated alias

4. **LLMResponse** → **LlmResponse**
   - Location: `crates/sage-core/src/llm/messages.rs`
   - Impact: Core response type, widely used
   - Status: ✅ **COMPLETED** - Renamed with deprecated alias

5. **LLMCache** → **LlmCache**
   - Location: `crates/sage-core/src/cache/llm_cache.rs`
   - Impact: Caching functionality for LLM responses
   - Status: ✅ **COMPLETED** - Renamed with deprecated alias

6. **LLMCacheBuilder** → **LlmCacheBuilder**
   - Location: `crates/sage-core/src/cache/llm_cache.rs`
   - Impact: Builder pattern for LLM cache
   - Status: ✅ **COMPLETED** - Renamed with deprecated alias

7. **LLMInteractionRecord** → **LlmInteractionRecord**
   - Location: `crates/sage-core/src/trajectory/recorder.rs`
   - Impact: Trajectory recording functionality
   - Status: ✅ **COMPLETED** - Renamed with deprecated alias

8. **LLMResponseRecord** → **LlmResponseRecord**
   - Location: `crates/sage-core/src/trajectory/recorder.rs`
   - Impact: Trajectory recording functionality
   - Status: ✅ **COMPLETED** - Renamed with deprecated alias

9. **LLMProvider** → **LlmProvider**
   - Location: `crates/sage-core/src/llm/provider_types.rs`
   - Impact: Enum for provider types, widely used
   - Status: ✅ **COMPLETED** - Renamed with deprecated alias

10. **LLMStream** → **LlmStream**
    - Location: `crates/sage-core/src/llm/streaming.rs`
    - Impact: Type alias for streaming functionality
    - Status: ✅ **COMPLETED** - Renamed with deprecated alias

#### SSE-prefixed Types (2 instances)

11. **SSEEvent** → **SseEvent**
    - Location: `crates/sage-core/src/llm/sse_decoder.rs`
    - Impact: Server-sent events parsing
    - Status: ✅ **COMPLETED** - Renamed with deprecated alias

12. **SSEDecoder** → **SseDecoder**
    - Location: `crates/sage-core/src/llm/sse_decoder.rs`
    - Impact: Server-sent events decoder
    - Status: ✅ **COMPLETED** - Renamed with deprecated alias

#### MCP-prefixed Types (2 instances)

13. **MCPServerCache** → **McpServerCache**
    - Location: `crates/sage-core/src/session/session_cache.rs`
    - Impact: MCP server caching functionality
    - Status: ✅ **COMPLETED** - Renamed with deprecated alias

14. **MCPServerConfig** → **McpServerConfig**
    - Location: `crates/sage-core/src/session/session_cache.rs`
    - Impact: MCP server configuration
    - Status: ✅ **COMPLETED** - Renamed with deprecated alias

#### CLI-prefixed Types (1 instance)

15. **CLIConsole** → **CliConsole**
    - Location: `crates/sage-cli/src/console.rs`
    - Impact: CLI console implementation
    - Status: ✅ **COMPLETED** - Renamed with deprecated alias

## Migration Strategy

### Phase 1: Deprecation (Version 0.2.0) - **CURRENT PHASE**

**Timeline:** Current development cycle
**Goal:** Introduce new names while maintaining backward compatibility

**Actions:**
1. ✅ Rename primary type definitions to use correct PascalCase
2. 🔄 Add deprecated type aliases for old names
3. 🔄 Update internal code to use new names
4. ✅ Add compiler warnings for deprecated usage
5. 📝 Document migration in CHANGELOG.md

**Deprecation Format:**
```rust
// New correct name
pub struct LlmClient { /* ... */ }

// Deprecated alias for backward compatibility
#[deprecated(since = "0.2.0", note = "Use `LlmClient` instead")]
pub type LLMClient = LlmClient;
```

**Impact:**
- Existing code continues to work
- Compiler shows deprecation warnings
- Users can migrate at their own pace
- Zero runtime performance impact (type aliases are zero-cost)

### Phase 2: Soft Deprecation Period (Versions 0.2.x - 0.9.x)

**Timeline:** Next 3-6 months (multiple minor releases)
**Goal:** Give users time to migrate to new names

**Actions:**
1. Keep deprecated aliases in place
2. Update all examples and documentation to use new names
3. Monitor usage of deprecated names (if analytics available)
4. Communicate migration timeline in release notes
5. Provide codemod/sed scripts for automated migration

**Migration Helper Script Example:**
```bash
# Automated migration script (to be created)
# Usage: ./scripts/migrate-naming.sh <path-to-your-code>

find "$1" -type f -name "*.rs" -exec sed -i '' \
  -e 's/\bLLMClient\b/LlmClient/g' \
  -e 's/\bLLMMessage\b/LlmMessage/g' \
  -e 's/\bLLMResponse\b/LlmResponse/g' \
  -e 's/\bLLMUsage\b/LlmUsage/g' \
  -e 's/\bSSEEvent\b/SseEvent/g' \
  -e 's/\bSSEDecoder\b/SseDecoder/g' \
  -e 's/\bMCPServerCache\b/McpServerCache/g' \
  -e 's/\bMCPServerConfig\b/McpServerConfig/g' \
  -e 's/\bCLIConsole\b/CliConsole/g' \
  {} \;
```

### Phase 3: Final Removal (Version 1.0.0)

**Timeline:** When ready for 1.0.0 release (TBD)
**Goal:** Complete migration to new names

**Actions:**
1. Remove all deprecated type aliases
2. Update version to 1.0.0 (major version bump)
3. Document breaking changes in CHANGELOG and migration guide
4. Announce in release notes with migration instructions

**Breaking Change Notice:**
```markdown
## Version 1.0.0 - Breaking Changes

### Removed Deprecated Type Aliases

All acronym-based type names deprecated in v0.2.0 have been removed.
Please use the PascalCase versions instead:

- LLMClient → LlmClient
- LLMMessage → LlmMessage
- LLMResponse → LlmResponse
- SSEEvent → SseEvent
- SSEDecoder → SseDecoder
- MCPServerCache → McpServerCache
- MCPServerConfig → McpServerConfig
- CLIConsole → CliConsole

See the migration guide for automated migration scripts.
```

## Implementation Checklist

### Immediate Actions (v0.2.0)

- [x] **types.rs**: Add `#[deprecated]` for `LLMUsage`
- [x] **client.rs**: Add `#[deprecated]` for `LLMClient`
- [x] **messages.rs**: Add `#[deprecated]` for `LLMMessage` and `LLMResponse`
- [x] **sse_decoder.rs**: Add `#[deprecated]` for `SSEEvent` and `SSEDecoder`
- [x] **provider_types.rs**: Add `#[deprecated]` for `LLMProvider`
- [x] **llm_cache.rs**: Add `#[deprecated]` for `LLMCache` and `LLMCacheBuilder`
- [x] **recorder.rs**: Add `#[deprecated]` for `LLMInteractionRecord` and `LLMResponseRecord`
- [x] **streaming.rs**: Add `#[deprecated]` for `LLMStream`
- [x] **session_cache.rs**: Add `#[deprecated]` for `MCPServerCache` and `MCPServerConfig`
- [x] **console.rs**: Add `#[deprecated]` for `CLIConsole`

### Documentation Updates

- [ ] Update README.md with new type names
- [ ] Update API documentation examples
- [ ] Update user guide examples
- [ ] Add migration guide to docs/
- [ ] Update CHANGELOG.md with deprecation notices

### Testing

- [ ] Verify all tests pass with new names
- [ ] Verify backward compatibility with deprecated aliases
- [ ] Test that deprecation warnings are shown correctly
- [ ] Update test code to use new names (suppress warnings if needed)

### Communication

- [ ] Announce deprecation in release notes
- [ ] Update project documentation
- [ ] Notify users through appropriate channels
- [ ] Provide migration timeline and support

## Timeline Summary

| Phase | Version | Timeline | Status |
|-------|---------|----------|--------|
| **Phase 1: Deprecation** | 0.2.0 | Current Sprint | 🔄 In Progress |
| **Phase 2: Soft Deprecation** | 0.2.x - 0.9.x | 3-6 months | ⏳ Upcoming |
| **Phase 3: Removal** | 1.0.0 | TBD (6+ months) | 📅 Planned |

## Benefits of This Migration

1. **Standards Compliance:** Aligns with Rust API Guidelines (RFC 430)
2. **Ecosystem Consistency:** Matches conventions used by major Rust projects
3. **Improved Readability:** PascalCase is more readable than all-caps acronyms
4. **Better Tooling:** Automated tools expect standard naming conventions
5. **Professional Quality:** Demonstrates attention to detail and best practices

## Risks and Mitigation

### Risk 1: User Code Breakage
**Mitigation:** Extended deprecation period (3-6 months) with clear warnings and migration guides

### Risk 2: Confusion During Transition
**Mitigation:** Clear documentation, automated migration scripts, and communication

### Risk 3: Internal Code Inconsistency
**Mitigation:** Update all internal code to new names in v0.2.0 before release

### Risk 4: Third-party Integration Issues
**Mitigation:** Type aliases maintain API compatibility during transition

## Support and Resources

- **Audit Report:** `docs/naming-convention-audit.md`
- **Rust API Guidelines:** https://rust-lang.github.io/rfcs/0430-finalizing-naming-conventions.html
- **Migration Script:** `scripts/migrate-naming.sh` (to be created)
- **Support:** GitHub issues with `naming-migration` label

## FAQ

### Q: Why can't we keep the old names?
**A:** While functional, the current naming violates Rust conventions and creates inconsistency with the broader ecosystem. Aligning with standards improves maintainability and professionalism.

### Q: Can I continue using the old names?
**A:** Yes, during the deprecation period (v0.2.x - v0.9.x). However, you'll see compiler warnings encouraging migration.

### Q: Will this affect my existing code?
**A:** Not immediately. Your code will continue to compile and run. You'll see deprecation warnings that you can address at your convenience before v1.0.0.

### Q: How do I migrate my code?
**A:** Use the provided migration script or manually replace old names with new ones. The changes are purely syntactic—no logic changes required.

### Q: When will the old names stop working?
**A:** The deprecated aliases will be removed in v1.0.0. The exact timeline will be announced well in advance.

## Conclusion

This migration plan provides a clear, gradual path to align the Sage Agent codebase with Rust naming conventions while minimizing disruption to users. The extended deprecation period and comprehensive support resources ensure a smooth transition.

**Key Dates:**
- **v0.2.0:** Deprecation begins (current)
- **v0.2.x - v0.9.x:** Transition period (3-6 months)
- **v1.0.0:** Deprecated names removed (TBD, 6+ months out)

For questions or concerns about this migration, please open an issue on GitHub with the `naming-migration` label.

---

**Document Maintainer:** Development Team
**Last Updated:** 2025-12-23
**Next Review:** Before v0.2.0 release
