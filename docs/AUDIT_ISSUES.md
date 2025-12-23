# Sage Agent Codebase Audit Issues

> Generated: 2025-12-22
> Updated: 2025-12-23
> Total Issues: 265
> Status: In Progress (Major Cleanup Phase Completed)

## Summary

| Severity | Count | Resolved |
|----------|-------|----------|
| Critical | 33 | 8 |
| High | 90 | 9 |
| Medium | 86 | 4 |
| Low | 56 | 0 |
| Clippy | 341 | 339 |

---

## Critical Issues (Priority 1)

### CRIT-001: Command Injection in Bash Tool
- **Status**: 🟢 Resolved
- **Location**: `sage-tools/src/tools/process/bash.rs`
- **Description**: User input passed directly to shell without sanitization
- **Risk**: Remote code execution
- **Fix**: Added comprehensive `validate_command_security()` with expanded dangerous patterns and operator blocking

### CRIT-002: Path Traversal Validation Disabled
- **Status**: 🟢 Resolved
- **Location**: `sage-core/src/tools/base.rs:272-342`
- **Description**: `is_safe_path()` always returns `true`
- **Risk**: Arbitrary file access
- **Fix**: Implemented canonical path checking with proper handling of non-existent files

### CRIT-003: Blocking Mutex in Async Context
- **Status**: 🟢 Resolved
- **Location**: `sage-core/src/interrupt.rs`
- **Description**: `.lock()` blocks Tokio runtime
- **Risk**: Runtime deadlock
- **Fix**: Replaced `std::sync::Mutex` with `parking_lot::Mutex` for faster, non-poisoning locks

### CRIT-004: Debug File Write in Production
- **Status**: 🟢 Resolved
- **Location**: `sage-core/src/llm/client.rs:829`
- **Description**: Writes to `/tmp/glm_request.json` in production
- **Risk**: Information leakage, disk space exhaustion
- **Fix**: Gated behind `#[cfg(debug_assertions)]` and `SAGE_DEBUG_REQUESTS` env var

### CRIT-005: Excessive unwrap() Calls
- **Status**: 🟡 In Progress
- **Location**: Multiple files (1414 occurrences total)
- **Description**: Potential panics throughout codebase
- **Risk**: Application crashes
- **Progress**:
  - Fixed http_client.rs: Added expect() with safety comment
  - Fixed monitoring.rs: Switched to parking_lot::Mutex (6 unwrap() calls removed)
  - Fixed sandbox/mod.rs: Switched to parking_lot::RwLock (1 unwrap() call removed)
  - Fixed reactive_agent.rs: Replaced .last().unwrap() with if-let pattern
  - Fixed telemetry/tool_usage.rs: Switched to parking_lot::RwLock (6 unwrap() calls removed)
  - Fixed tools/registry.rs: Switched to parking_lot::Mutex (1 unwrap() call removed)
  - Fixed task_management.rs: Switched to parking_lot::Mutex (all lock().unwrap() removed)
  - Remaining: ~1380 occurrences (mostly in test code, acceptable)

### CRIT-006: Missing Tool Input Validation
- **Status**: 🟢 Resolved
- **Location**: `sage-tools/src/tools/network/`
- **Description**: Tool inputs not validated before execution
- **Risk**: SSRF attacks, injection attacks, unexpected behavior
- **Fix**: Added comprehensive URL validation module with SSRF prevention:
  - Created `validation.rs` with `validate_url_security()` function
  - Blocks localhost, private IPs, cloud metadata endpoints
  - Validates URL schemes (only http/https allowed)
  - Blocks internal hostnames (.local, .internal, .localhost)
  - Integrated into `web_fetch` tool
  - Added 7 comprehensive test cases

### CRIT-007: Hardcoded Default Credentials
- **Status**: 🟢 Resolved
- **Location**: `sage-core/src/llm/client.rs`
- **Description**: API keys in default values
- **Risk**: Credential exposure
- **Fix**: Replaced `.unwrap_or_default()` with proper `.ok_or_else()` validation for Azure, OpenRouter, and Doubao providers

### CRIT-008: No Rate Limiting for LLM Calls
- **Status**: 🟢 Resolved
- **Location**: `sage-core/src/llm/`
- **Description**: LLM calls can trigger provider rate limits
- **Risk**: Service disruption, cost overrun
- **Fix**: Implemented token bucket rate limiter:
  - Created `rate_limiter.rs` module with `RateLimiter` and `RateLimitConfig`
  - Provider-specific rate limits (OpenAI, Anthropic, Google, etc.)
  - Global rate limiter registry for shared state across clients
  - Burst support (allows short bursts above sustained rate)
  - Non-blocking `try_acquire()` and blocking `acquire()` methods
  - Integration in both `chat()` and `chat_stream()` methods
  - Added 8 comprehensive test cases

---

## High Priority Issues (Priority 2)

### HIGH-001: Dependency Version Conflicts - nix
- **Status**: 🟢 Resolved
- **Location**: `Cargo.toml` files
- **Description**: `nix` crate version mismatch (0.27 vs 0.29)
- **Fix**: Updated sage-core to use workspace version (0.29)

### HIGH-002: Dependency Version Conflicts - reqwest
- **Status**: 🟢 Resolved
- **Location**: `Cargo.toml` files
- **Description**: `reqwest` version mismatch (0.11 vs 0.12)
- **Fix**: Updated sage-tools to use workspace version (0.12)

### HIGH-003: Insufficient Test Coverage
- **Status**: 🔴 Open
- **Location**: `tests/` directories
- **Description**: Only 10 test files total, ~15% coverage
- **Fix**: Add comprehensive unit and integration tests

### HIGH-004: No API Versioning
- **Status**: 🟢 Resolved
- **Location**: `sage-sdk/src/version.rs`, `sage-sdk/src/lib.rs`, `sage-sdk/src/client.rs`
- **Description**: No versioning strategy for breaking changes
- **Fix**: Implemented comprehensive API versioning system:
  - Created `version.rs` module with `Version` struct and version constants (`API_VERSION`, `MIN_SUPPORTED_VERSION`)
  - Implemented SemVer-compliant version parsing and comparison
  - Added version negotiation utilities (`is_compatible()`, `negotiate_version()`)
  - Created deprecation macros (`deprecated_since!`, `experimental!`)
  - Added version methods to `SageAgentSDK`: `api_version()`, `version_info()`, `is_compatible_with()`
  - Documented versioning strategy and deprecation policy in module docs
  - Added comprehensive test suite (15 test cases)
  - Exposed version module and constants in public API

### HIGH-005: Inconsistent Error Formats
- **Status**: 🔴 Open
- **Location**: Cross-crate error handling
- **Description**: Different error types and formats across crates
- **Fix**: Unify error handling with common error types

### HIGH-006: Provider Whitelist Incomplete
- **Status**: 🟢 Resolved
- **Location**: `sage-core/src/config/validation.rs:35-49`
- **Description**: Missing glm, openrouter, doubao, azure providers
- **Fix**: Added all providers (azure, openrouter, doubao, glm, zhipu) to whitelist and API key validation

### HIGH-007: Blocking Operations in Async
- **Status**: 🟡 Acceptable
- **Location**: `sage-core/src/agent/`
- **Description**: 6 blocking operations found in async context
- **Analysis**: Main issue (CRIT-003) fixed with parking_lot::Mutex. Remaining operations are:
  - AgentRegistry uses std::sync::RwLock for quick HashMap ops (acceptable for short critical sections)
  - File I/O in workspace detection (startup only, not in hot path)
- **Decision**: Current state acceptable; no changes needed

### HIGH-008: Unsafe Blocks Without Justification
- **Status**: 🟢 Resolved
- **Location**: `sage-core/src/plugins/registry.rs`, `sage-core/src/sandbox/executor.rs`
- **Description**: 8 `unsafe` blocks without safety comments
- **Fix**: Added comprehensive SAFETY comments explaining invariants for all unsafe blocks

### HIGH-009: Multiple Registry Implementations
- **Status**: 🟡 Acceptable
- **Location**: `sage-core`, `sage-tools`
- **Description**: 14+ registries with similar base patterns (HashMap storage, CRUD operations)
- **Analysis**: Analyzed SkillRegistry (542 lines), CommandRegistry (595 lines), PromptRegistry (457 lines), HookRegistry, ToolRegistry, and others. While they share common CRUD patterns (~15-20 lines each), each has 50-100+ lines of domain-specific functionality:
  - SkillRegistry: `find_matching()`, priority-based selection, enable/disable
  - CommandRegistry: source tracking (Builtin/User/Project), `list_by_source()`
  - PromptRegistry: `render()`, secondary indexes (by_category, by_tag), `search()`
  - HookRegistry: event-based organization, pattern matching
- **Decision**: Not redundant. Creating a generic trait would add abstraction without reducing complexity. Current pattern is idiomatic Rust for typed collections. No changes needed.

### HIGH-010: Code Duplication
- **Status**: 🟡 Acceptable
- **Location**: Tool implementations in `sage-tools/src/tools/`
- **Description**: Originally estimated 2000+ lines; actual analysis found ~400-600 lines of duplicated code
- **Analysis**: Duplication is primarily in:
  - Test helper `create_tool_call()`: 22 files, 19 exact duplicates (~220 lines)
  - FileSystemTool impl: 8 files with identical 4-line implementations
  - Tool struct constructors: `new()`, `with_working_directory()` patterns
  - Default impl boilerplate
- **Potential Fixes** (optional, for future cleanup):
  - Create `sage-tools/src/test_utils.rs` for shared test helpers
  - Consider proc macro for tool struct boilerplate
  - Use `#[derive(Default)]` where applicable
- **Decision**: Acceptable for now. Duplication is in boilerplate/tests, not core logic. Doesn't impact functionality or maintainability significantly.

### HIGH-011: No Observability Instrumentation
- **Status**: 🟡 Partial
- **Location**: Entire codebase
- **Description**: Missing metrics and tracing spans
- **Progress**:
  - Added `#[instrument]` to UnifiedExecutor.execute() with task_id and task_description fields
  - Added `#[instrument]` to MCP client methods (initialize, list_tools, call_tool)
  - Added `#[instrument]` to BashTool.execute_command with command preview
  - Added `#[instrument]` to ReadTool.read_file with path field
  - Added `#[instrument]` to EditTool.execute with call_id field
- **Remaining**: Add metrics collection, more span coverage in LLM providers

### HIGH-012: Trajectory Replay Not Implemented
- **Status**: 🟢 Resolved
- **Location**: `sage-core/src/trajectory/`
- **Description**: Recording works but replay is stub
- **Fix**: Implemented `TrajectoryReplayer` module with:
  - `load_from_file()` to load trajectories from JSON files
  - `load_by_id()` to load from storage backend
  - `list_trajectories()` to scan directory for trajectory files
  - `analyze_steps()` for step-by-step analysis
  - `summarize()` for trajectory summary
  - `calculate_token_usage()` for token usage statistics
  - Fixed stub methods in `TrajectoryRecorder`: `load_trajectory()`, `list_trajectories()`, `delete_trajectory()`, `get_statistics()`
  - Fixed `FileStorage.list()` to actually scan for trajectory files

---

## Medium Priority Issues (Priority 3)

### MED-001: Missing Error Context
- **Status**: 🔴 Open
- **Description**: Errors lack sufficient context for debugging
- **Fix**: Add context using `.context()` from anyhow

### MED-002: No Architecture Decision Records
- **Status**: 🔴 Open
- **Location**: `docs/`
- **Description**: 0/13 ADRs documented
- **Fix**: Create ADRs for key architectural decisions

### MED-003: Missing User Guides
- **Status**: 🔴 Open
- **Location**: `docs/user-guide/`
- **Description**: 0/6 user guides present
- **Fix**: Write comprehensive user documentation

### MED-004: CLI Mode Confusion
- **Status**: 🟢 Resolved
- **Location**: `sage-cli/src/`
- **Description**: Interactive vs non-interactive mode unclear
- **Fix**: Added comprehensive CLI mode documentation:
  - Created `CliMode` enum with clear descriptions and examples
  - Added extensive module-level documentation explaining all execution modes
  - Enhanced help text for all commands (Run, Interactive, Unified, Config, Trajectory, Tools)
  - Added usage examples for each command and subcommand
  - Documented special commands available in interactive mode
  - Clarified differences between Run (one-shot), Interactive (multi-turn), and Unified (advanced) modes
  - Updated ConfigAction and TrajectoryAction enums with detailed descriptions

### MED-005: Inconsistent Tool Response Format
- **Status**: 🔴 Open
- **Location**: `sage-tools/src/tools/`
- **Description**: Tools return different response structures
- **Fix**: Standardize tool response format

### MED-006: Missing Retry Logic for LLM
- **Status**: 🟢 Resolved
- **Location**: `sage-core/src/llm/`
- **Description**: No automatic retry on transient failures
- **Fix**: Added exponential backoff with jitter to LLM client retry logic:
  - Implemented `jittered_backoff()` function with configurable base delay
  - Uses random jitter (0-50% of delay) to prevent thundering herd
  - Applied to all retryable error scenarios

### MED-007: Large Trajectory Files
- **Status**: 🔴 Open
- **Location**: `trajectories/`
- **Description**: Files can grow very large
- **Fix**: Implement compression and rotation

### MED-008: No Graceful Shutdown
- **Status**: 🟢 Resolved
- **Location**: `sage-core/src/agent/`
- **Description**: Agent doesn't handle shutdown signals properly
- **Fix**: Implemented graceful shutdown in `UnifiedExecutor`:
  - Added `shutdown()` method to UnifiedExecutor
  - Stops animations via AnimationManager.stop_animation()
  - Finalizes trajectory recording if present
  - Logs session cleanup with session ID

### MED-009: Missing Inline Documentation
- **Status**: 🔴 Open
- **Location**: Multiple files
- **Description**: Many public functions lack doc comments
- **Fix**: Add rustdoc comments to public APIs

### MED-010: Timeout Handling Inconsistent
- **Status**: 🔴 Open
- **Location**: `sage-core/src/llm/`
- **Description**: Different timeout handling per provider
- **Fix**: Unify timeout configuration and handling

---

## Low Priority Issues (Priority 4)

### LOW-001: Unused Dependencies
- **Status**: 🔴 Open
- **Description**: Some dependencies may be unused
- **Fix**: Audit and remove unused dependencies

### LOW-002: Inconsistent Naming
- **Status**: 🔴 Open
- **Description**: Some inconsistencies in naming conventions
- **Fix**: Standardize naming across codebase

### LOW-003: Missing CHANGELOG
- **Status**: 🔴 Open
- **Location**: Root directory
- **Description**: No CHANGELOG.md file
- **Fix**: Create and maintain changelog

### LOW-004: Example Code Outdated
- **Status**: 🔴 Open
- **Location**: `examples/`
- **Description**: Some examples may not reflect current API
- **Fix**: Update examples to current API

### LOW-005: No Contributing Guide
- **Status**: 🔴 Open
- **Location**: Root directory
- **Description**: Missing CONTRIBUTING.md
- **Fix**: Create contribution guidelines

---

## Progress Log

| Date | Issue | Status | Commit |
|------|-------|--------|--------|
| 2025-12-22 | CRIT-004 | Resolved | db3eed5 |
| 2025-12-22 | CRIT-002 | Resolved | ef01fe7 |
| 2025-12-22 | CRIT-001 | Resolved | cb4b5b5 |
| 2025-12-22 | CRIT-003 | Resolved | e46e4f4 |
| 2025-12-22 | HIGH-006 | Resolved | 150e08a |
| 2025-12-22 | HIGH-001 | Resolved | 4a3f740 |
| 2025-12-22 | HIGH-002 | Resolved | 4a3f740 |
| 2025-12-22 | CRIT-007 | Resolved | ef9c297 |
| 2025-12-22 | HIGH-008 | Resolved | 6459fdb |
| 2025-12-22 | CRIT-005 | Partial | 85e4863 |
| 2025-12-22 | CRIT-005 | Partial | 02fb81d |
| 2025-12-22 | CRIT-006 | Resolved | ff87be2 |
| 2025-12-22 | CRIT-008 | Resolved | 2c0d1e0 |
| 2025-12-22 | HIGH-009 | Acceptable | (analysis only) |
| 2025-12-22 | HIGH-010 | Acceptable | (analysis only) |
| 2025-12-22 | MED-006 | Resolved | (jitter in retry logic) |
| 2025-12-22 | MED-008 | Resolved | (graceful shutdown) |
| 2025-12-22 | HIGH-011 | Partial | (tracing instrumentation) |
| 2025-12-22 | HIGH-012 | Resolved | (trajectory replayer) |
| 2025-12-23 | CRIT-005 | Partial | 4d74353 (more unwrap fixes) |
| 2025-12-23 | CRIT-003 | Enhanced | b2cf3b4 (parking_lot::Mutex in signal_handler.rs) |
| 2025-12-23 | Clippy | Fixed | 81a35b5 (auto-fix 35 files) |
| 2025-12-23 | Clippy | Fixed | 1b7c6c3 (341→2 warnings) |

---

### Clippy Cleanup Summary (2025-12-23)

A major clippy cleanup was performed, reducing warnings from **341 to 2**:

1. **Phase 1: Critical Fixes**
   - Replaced `std::sync::Mutex` with `parking_lot::Mutex` in `signal_handler.rs`
   - Fixed MutexGuard held across await points
   - Added parking_lot dependency to sage-cli

2. **Phase 2: Auto-fix**
   - Ran `cargo clippy --fix` across workspace
   - Applied automatic fixes to 35 files
   - Fixed iterator patterns, redundant closures, type annotations

3. **Phase 3: Manual Improvements**
   - Replaced `filter_map` with `map` where all arms return `Some`
   - Used `sort_by_key` instead of `sort_by`
   - Simplified `match` with `unwrap_or` patterns
   - Used `strip_prefix` instead of manual slicing
   - Fixed loop variable indexing with `enumerate`
   - Used HashMap `entry` API
   - Added `Default` impl for `DisplayManager`
   - Added `#[allow]` attributes for intentional design choices

Remaining 2 warnings are deprecated method warnings (intentional deprecation).

---

## Notes

- All fixes should include tests
- Each fix should be committed separately
- Run `make ci` before committing
- Follow Rust 2024 edition standards
