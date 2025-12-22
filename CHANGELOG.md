# Changelog

All notable changes to the Sage Agent project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

#### LLM Provider Support
- GLM (Zhipu AI) provider support with proper tool conversion
- OpenRouter provider with improved routing
- Azure provider support
- Doubao provider support
- Token bucket rate limiter for all LLM API calls to prevent service disruption (CRIT-008)
- Retry mechanism with exponential backoff and jitter for transient failures
- Prompt caching support for Anthropic API with cache token statistics

#### SWEBench Evaluation
- Complete SWEBench evaluation framework with improved prompting
- Evaluation results tracking after refactoring

#### CI/CD and Development Tools
- Complete CI/CD pipeline with GitHub Actions
- Comprehensive documentation update based on actual features
- Example configuration file `sage_config.json.example`

#### Trajectory and Session Recording
- Unlimited execution steps with mandatory trajectory recording
- Claude Code-style session recording and file tracking
- JSONL storage for enhanced messages
- Claude Code-inspired enhanced message types
- Trajectory replayer module with analysis, statistics, and summarization capabilities
- Fixed trajectory storage `list()` method to actually scan for trajectory files

#### Interactive Features
- Slash command support in chat mode: `/cost`, `/context`, `/status`, `/resume`, `/plan`, `/undo`
- Interactive `/resume` command for session continuation
- Slash command processing in run command
- Improved `/undo` prompt for better directory isolation

#### MCP (Model Context Protocol) Integration
- Learning Mode with MCP Server integration
- MCP notification handling system
- MCP resource cache with TTL and statistics
- MCP schema translator for bidirectional tool schema conversion
- HTTP transport layer for MCP with SSE support
- Enhanced MCP module with concurrent requests and server discovery

#### Claude Code Compatibility
- Claude Code compatible TodoWrite and Task tools
- Claude Code style prompt system integrated into agent execution
- Subagent system for decomposed task execution
- Help/Feedback and Documentation Lookup tools
- Version tracking system
- Hooks system for extensibility
- Session notes capability
- Output truncation for large results

#### Tools and Utilities
- Comprehensive tools for Kubernetes operations
- Monitoring and observability tools
- Terraform infrastructure tools
- Git version control tools
- Deno and RPC clients for Sage Agent communication

#### Observability
- Tracing instrumentation with `#[instrument]` spans:
  - UnifiedExecutor.execute() with task tracking
  - MCP client methods (initialize, list_tools, call_tool)
  - BashTool.execute_command with command preview
  - ReadTool.read_file with path tracking
  - EditTool.execute with call ID tracking

### Changed

#### Architecture and Refactoring
- Major architecture refactoring with new subsystems
- Refactored sage-core to modularize large files and eliminate code duplication
- Unified execution loop following Claude Code style
- Replaced `std::sync::Mutex` with `parking_lot::Mutex` for better async performance (CRIT-003)
- Replaced `std::sync::RwLock` with `parking_lot::RwLock` in sandbox module
- Improved code-first execution with modular prompt system

#### User Experience
- Updated system prompt to prioritize action over questions
- Enhanced user message handling with prompt caching
- Improved Read tool error messages for directory paths
- Enhanced console welcome banner with adaptive width
- Optimized interactive mode exit logic and Ctrl+C signal handling
- Restored interactive UI while using unified executor

#### Tools
- Redesigned Edit tool to match Claude Code's design
- Improved tool registration in UnifiedExecutor
- Corrected tool name variables to match actual Sage tools

#### Configuration
- Streamlined signal handling and cache module imports
- Applied custom base_url from config to ProviderConfig
- Added total_token_budget for session-wide token limits

### Fixed

#### Core Functionality
- Agent execution now pauses correctly when ask_user_question tool is called
- Prevented sending both temperature and top_p to Anthropic API (API constraint)
- Fixed cache_control on empty text blocks and conversation history
- Limited cache_control to last 2 messages (Anthropic max 4 blocks limit)
- Included tool descriptions in system prompt
- Fixed default tool registration in UnifiedExecutor
- Included cache_creation_input_tokens and cache-read tokens in total token count
- Recorded LLM response and token usage in trajectory
- Recorded input messages (llm_messages) in trajectory

#### Provider-Specific Fixes
- Fixed GLM temperature precision issue and added GLM-specific tool conversion
- Fixed OpenRouter tool_calls format and provider routing
- Used Google provider for OpenRouter to avoid Bedrock tool_call format issues
- Fixed proper Anthropic tool_result format with is_error support
- Added tool_calls to assistant message in UnifiedExecutor

#### Code Quality
- Resolved 29 failing tests and cleaned up warnings
- Prevented potential deadlock by avoiding holding multiple locks
- Prevented panic from HashMap indexing and poisoned locks
- Removed critical unwrap() calls in sage-tools and multiple modules (CRIT-005, partial)
- Added expect() with safety comments where unwrap() was necessary

#### Dependencies
- Unified dependency versions across workspace (HIGH-001, HIGH-002):
  - Updated `nix` crate from 0.27 to 0.29
  - Updated `reqwest` from 0.11 to 0.12

#### Documentation
- Added SAFETY comments to all unsafe blocks (HIGH-008)
- Updated AUDIT_ISSUES.md with progress tracking
- Updated Rust version requirement to 1.85+ in README files
- Updated copyright year to 2025
- Updated repository link to new GitHub address

#### Other Fixes
- Added warning for empty tool input from proxy servers
- Fixed graceful shutdown in UnifiedExecutor with animation cleanup and trajectory finalization
- Added provider whitelist validation for all providers (HIGH-006)

### Security

#### Critical Security Fixes
- **CRIT-001**: Comprehensive command injection protection in Bash tool
  - Added `validate_command_security()` with expanded dangerous patterns
  - Blocked shell operators and command chaining
  - Prevented execution of dangerous commands

- **CRIT-002**: Proper path traversal validation
  - Implemented canonical path checking
  - Proper handling of non-existent files
  - Prevented arbitrary file access outside allowed directories

- **CRIT-003**: Fixed blocking mutex in async context
  - Replaced `std::sync::Mutex` with `parking_lot::Mutex`
  - Eliminated runtime deadlock risk
  - Improved async performance

- **CRIT-004**: Gated debug file writes behind feature flag
  - Debug writes to `/tmp/glm_request.json` now require `#[cfg(debug_assertions)]`
  - Added `SAGE_DEBUG_REQUESTS` environment variable guard
  - Prevented information leakage in production

- **CRIT-006**: URL validation to prevent SSRF attacks
  - Created comprehensive URL validation module
  - Blocked localhost, private IPs, and cloud metadata endpoints
  - Validated URL schemes (only http/https allowed)
  - Blocked internal hostnames (.local, .internal, .localhost)
  - Integrated into web_fetch tool with 7 comprehensive test cases

- **CRIT-007**: Required explicit API keys for cloud providers
  - Replaced `.unwrap_or_default()` with proper `.ok_or_else()` validation
  - Eliminated hardcoded default credentials
  - Enforced explicit configuration for Azure, OpenRouter, and Doubao

- **CRIT-008**: Implemented rate limiting for LLM calls
  - Created token bucket rate limiter with provider-specific limits
  - Global rate limiter registry for shared state
  - Burst support for short spikes above sustained rate
  - Non-blocking `try_acquire()` and blocking `acquire()` methods
  - Integrated in both `chat()` and `chat_stream()` methods
  - Added 8 comprehensive test cases

### Removed
- Removed redundant root-level tools directory
- Deleted language selection interface and related documentation to simplify project structure

## [0.1.0] - Initial Development

This version represents the initial development phase of Sage Agent, a Rust-based LLM agent system for software engineering tasks.

### Initial Features
- Multi-crate Rust workspace architecture (sage-core, sage-cli, sage-sdk, sage-tools)
- Async architecture built on Tokio
- Support for multiple LLM providers (OpenAI, Anthropic, Google)
- Rich tool ecosystem for software engineering tasks
- Interactive CLI with terminal UI
- Configuration system with JSON files
- Comprehensive examples and documentation

---

## Version History

- **[Unreleased]**: Current development version with security fixes and new features
- **0.1.0**: Initial development release

For detailed commit history, see the [Git log](https://github.com/majiayu000/sage/commits/).
