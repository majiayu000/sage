# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

#### Testing & Quality
- Unit tests for agent module (base, execution, state, lifecycle, unified)
- Unit tests for LLM module (client, fallback, rate_limiter, parsers)
- Integration tests for sage-tools (bash, edit, grep)
- Comprehensive test coverage for core modules (HIGH-003)

#### Documentation
- Architecture Decision Records (ADR) documentation (MED-002)
- User guide documentation (installation, configuration, quick-start) (MED-003)
- Comprehensive rustdoc comments for SDK and core modules (MED-009)
- CLI mode documentation with usage examples (MED-004)
- TodoWrite best practices guide
- Explore Agent usage guide
- Timeout unification audit report (MED-010)
- Dependency audit report (LOW-001)
- Naming convention audit report (LOW-002)
- Tool response audit documentation
- Test coverage summary for LLM module
- SAFETY comments for all unsafe blocks (HIGH-008)
- Contributing guide (CONTRIBUTING.md) (LOW-005)

#### Features
- API versioning system for SDK with SemVer support (HIGH-004)
  - Version negotiation utilities (`is_compatible()`, `negotiate_version()`)
  - Deprecation macros (`deprecated_since!`, `experimental!`)
  - Version methods in `SageAgentSDK`
- Rate limiting for LLM API calls with token bucket algorithm (CRIT-008)
  - Provider-specific rate limits
  - Burst support for short bursts above sustained rate
  - Global rate limiter registry for shared state
- Trajectory replay functionality (HIGH-012)
  - `TrajectoryReplayer` module with load, list, and analyze capabilities
  - Step-by-step analysis and summary generation
  - Token usage statistics calculation
- Tool usage telemetry tracking system
- Tool usage policy and validation system
- Multi-provider fallback for quota/rate limit errors
- Provider fallback detection for quota/rate limit errors
- Graceful shutdown handling in UnifiedExecutor (MED-008)
  - Animation cleanup
  - Trajectory finalization
  - Session cleanup logging
- SWEBench evaluation framework with improved prompting
- Comprehensive URL validation with SSRF prevention (CRIT-006)
  - Blocks localhost, private IPs, cloud metadata endpoints
  - Validates URL schemes (only http/https allowed)
  - Blocks internal hostnames (.local, .internal, .localhost)
- Tracing instrumentation for key operations (HIGH-011)
  - UnifiedExecutor.execute() with task tracking
  - MCP client methods
  - BashTool, ReadTool, EditTool execution
- Unlimited steps execution mode
- Mandatory trajectory recording
- Claude Code style storage format

### Changed

#### Architecture & Refactoring
- Complete large file refactoring (Phase 1-6) across codebase
- Modularized large files in sage-core
- Eliminated code duplication across modules
- Extracted conversation types to separate module
- Replaced `std::sync::Mutex` with `parking_lot::Mutex` throughout codebase (CRIT-003)
  - Faster, non-poisoning locks
  - Prevents runtime deadlock in async contexts
  - Applied in: interrupt.rs, monitoring.rs, telemetry, task_management.rs, tools/registry.rs
- Replaced `std::sync::RwLock` with `parking_lot::RwLock` in sandbox module

#### Error Handling & Safety
- Unified error types with SageError (HIGH-005)
- Enhanced error context with `.context()` support (MED-001)
  - Added context to trajectory operations (storage.rs, unified.rs)
  - Improved error messages for debugging
- Replaced `unwrap()` with safe patterns across core modules (CRIT-005)
  - Fixed in: http_client.rs, monitoring.rs, sandbox/mod.rs
  - Fixed in: reactive_agent.rs, telemetry, tools/registry.rs
  - Fixed in: rpc_server.rs, claude_mode.rs, reorganize_tasklist.rs
  - Fixed in: progress.rs, todo_write.rs, task.rs, ui_backend.rs
  - Reduced from 1414 to ~1350 occurrences (remaining in test code)
- Replaced `.unwrap_or_default()` with proper `.ok_or_else()` validation (CRIT-007)

#### Dependencies
- Updated `nix` crate to unified workspace version (0.29) (HIGH-001)
- Updated `reqwest` to unified workspace version (0.12) (HIGH-002)
- Removed `uuid` dependency (unused)
- Added `parking_lot` for better synchronization primitives
- Added all providers to validation whitelist (HIGH-006)
  - Added: azure, openrouter, doubao, glm, zhipu

#### Code Quality
- Reduced clippy warnings from 341 to 2
  - Auto-fix applied to 35 files
  - Manual improvements for iterator patterns, match statements
  - Simplified with `sort_by_key`, `strip_prefix`, HashMap `entry` API
- Improved code style and patterns following Rust 2024 standards
- Added `#[allow]` attributes for intentional design choices

#### LLM & Providers
- Replaced deprecated `with_timeout` with `with_timeouts` API
- Added exponential backoff with jitter to retry logic (MED-006)
  - Configurable base delay
  - Random jitter (0-50% of delay) to prevent thundering herd
- Required explicit API keys for cloud providers (CRIT-007)

### Fixed

#### Security Vulnerabilities
- Command injection vulnerability in Bash tool (CRIT-001)
  - Added comprehensive `validate_command_security()`
  - Expanded dangerous patterns and operator blocking
- Path traversal validation (CRIT-002)
  - Implemented canonical path checking
  - Proper handling of non-existent files
- Blocking mutex in async context causing runtime deadlock (CRIT-003)
- Debug file write in production (CRIT-004)
  - Gated behind `#[cfg(debug_assertions)]`
  - Requires `SAGE_DEBUG_REQUESTS` env var
- SSRF vulnerabilities in web fetch tool (CRIT-006)
- Hardcoded default credentials in LLM client (CRIT-007)

#### Bug Fixes
- MutexGuard held across await points in signal handler
- Doc tests and example import paths
- Dead code warnings and unused imports
- Async operations using blocking operations (HIGH-007)
- Test compilation errors across workspace
- Port parsing in RPC server (replaced parse().unwrap() with SocketAddr::new())
- I/O flush unwrap() calls in CLI mode (replaced with let _ = pattern)

### Security
- Comprehensive command injection protection with pattern matching
- Proper path traversal validation with canonical path checking
- URL validation with SSRF prevention (localhost, private IPs, metadata endpoints)
- Token bucket rate limiter to prevent API abuse
- Input validation for all network tools
- Explicit API key requirements for cloud providers
- Debug file writes gated behind feature flag

### Deprecated
- None

### Removed
- `uuid` dependency (unused in codebase)
- Dead code and unused imports across workspace
- Debug file write to `/tmp/glm_request.json` in production

---

## [0.1.0] - 2024-XX-XX

### Added

#### Core Features
- Multi-LLM support for multiple providers
  - OpenAI, Anthropic, Google Gemini
  - Azure OpenAI, OpenRouter
  - Doubao (ByteDance), GLM (Zhipu AI)
- Async architecture built on Tokio runtime
- Interactive CLI with terminal UI animations
- Progress indicators and status displays
- SDK for programmatic integration

#### Tool Ecosystem
- Bash execution tool with security validation
- File operations: read, write, edit, glob
- Search tools: grep with regex support
- JSON editing capabilities
- Codebase retrieval and analysis
- Task management system
- MCP (Model Context Protocol) integration

#### Execution & Recording
- Trajectory recording system
- Complete execution tracking
- Session management
- Workspace detection
- Multi-step agent execution
- Reactive agent patterns

#### Configuration
- JSON-based configuration system
- Environment variable substitution
- Provider-specific settings
- Timeout configuration
- Retry logic configuration

### Changed
- Initial architecture and design decisions
- Workspace structure with four main crates
  - sage-core: Core library and agent engine
  - sage-cli: Command-line interface
  - sage-sdk: High-level SDK
  - sage-tools: Built-in tools collection

### Security
- Initial security model and tool sandboxing
- API key management
- Basic input validation

---

## Version History

- [Unreleased] - Current development (pre-release improvements)
- [0.1.0] - Initial release (unreleased)

## Links

- [Repository](https://github.com/majiayu000/sage)
- [Issue Tracker](https://github.com/majiayu000/sage/issues)
- [Documentation](https://github.com/majiayu000/sage/tree/main/docs)

## Notes

- Issues are tracked in `docs/AUDIT_ISSUES.md`
- See `CONTRIBUTING.md` for contribution guidelines
- Follow Rust 2024 edition standards for all code
- Run `make ci` before submitting changes
