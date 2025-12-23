# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Unit tests for agent module (base, execution, state, lifecycle, unified)
- Unit tests for LLM module (client, fallback, rate_limiter, parsers)
- Integration tests for sage-tools (bash, edit, grep)
- Architecture Decision Records (ADR) documentation
- User guide documentation (installation, configuration, quick-start)
- Comprehensive rustdoc comments for SDK and core modules
- Timeout unification audit report
- Dependency audit report
- Rate limiting for LLM API calls
- Trajectory replay functionality
- API versioning system for SDK
- Graceful shutdown handling

### Changed
- Unified error types with SageError
- Enhanced error context with .context() support
- Reduced clippy warnings from 341 to 2

### Fixed
- Command injection vulnerability in Bash tool
- Path traversal validation
- Blocking mutex in async context
- Debug file write in production
- SSRF vulnerabilities in web fetch tool
- Hardcoded default credentials

### Security
- Added comprehensive URL validation with SSRF prevention
- Implemented token bucket rate limiter for API calls
- Added input validation for all tools

## [0.1.0] - 2024-XX-XX (Initial Release)

### Added
- Multi-LLM support (OpenAI, Anthropic, Google, Azure, etc.)
- Rich tool ecosystem (bash, edit, read, write, grep, glob, etc.)
- Interactive CLI with animations
- Trajectory recording and replay
- SDK for programmatic integration
- Async architecture built on Tokio
