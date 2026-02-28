# Command Execution Security

**Last Updated:** 2026-01-21
**Status:** Audited

This document describes the security measures in place for command execution in Sage Agent.

## Overview

The codebase uses `std::process::Command` (and `tokio::process::Command`) for executing external commands. There are two primary patterns:

1. **Shell command execution** - Commands passed through a shell interpreter (`bash -c` or `sh -c`)
2. **Direct command execution** - Specific executables called with argument arrays

## Security Risk Analysis

### High Risk: Shell Command Execution

When commands are executed via `bash -c "user_input"`, there's potential for command injection if `user_input` is not properly validated.

**Mitigated in:**
- `sage-tools/src/tools/process/bash/execution.rs` - Uses comprehensive security validation

**Security Controls:**
1. `is_command_allowed()` - CommandTool trait whitelisting
2. `validate_command_comprehensive()` - Multi-layer validation including:
   - Fork bomb detection
   - System destruction command detection
   - Heredoc injection detection
   - Shell metacharacter abuse detection
   - Variable injection detection
   - Dangerous pattern detection
   - Critical path removal prevention

### Low Risk: Direct Command Execution

When executables are called directly with arguments as separate array elements, shell injection is not possible.

**Pattern:**
```rust
Command::new("git")
    .args(["status", "--porcelain"])
    .output()
```

This is safe because:
- The executable name is hardcoded
- Arguments are passed as separate strings, not interpreted by a shell
- No shell metacharacters can escape the argument boundaries

## Command Execution Inventory

### Bash Tool (High Risk - Mitigated)
- **File:** `crates/sage-tools/src/tools/process/bash/execution.rs`
- **Risk:** High (executes arbitrary user commands)
- **Mitigation:** Full security validation via `validate_command_comprehensive()`

### Git Tools (Low Risk)
- **Files:**
  - `crates/sage-tools/src/tools/vcs/git/executor.rs`
  - `crates/sage-tools/src/tools/vcs/git_simple.rs`
- **Risk:** Low (hardcoded `git` command, args passed separately)
- **Pattern:** `Command::new("git").args(&["rev-parse", "--show-toplevel"])`

### Docker Tool (Low Risk)
- **File:** `crates/sage-tools/src/tools/container/docker/commands.rs`
- **Risk:** Low (hardcoded `docker` command, args passed separately)
- **Pattern:** `Command::new("docker").args(&["ps", "-a"])`
- **Note:** Some args come from user input (container names, image names) but are passed as separate arguments

### Kubernetes Tool (Low Risk)
- **File:** `crates/sage-tools/src/tools/infrastructure/kubernetes.rs`
- **Risk:** Low (hardcoded `kubectl` command, args passed separately)
- **Pattern:** `Command::new("kubectl").args(&["get", "pods"])`

### Terraform Tool (Low Risk)
- **File:** `crates/sage-tools/src/tools/infrastructure/terraform.rs`
- **Risk:** Low (hardcoded `terraform` command, args passed separately)
- **Pattern:** `Command::new("terraform").args(&["init"])`

### Sandbox Executor (Medium Risk - Mitigated)
- **File:** `crates/sage-core/src/sandbox/executor/executor.rs`
- **Risk:** Medium (lower-level executor)
- **Mitigation:** Resource limits applied, used by higher-level validated tools

### Hook Executor (Medium Risk - User Configured)
- **File:** `crates/sage-core/src/hooks/executor.rs`
- **Risk:** Medium (executes user-configured hooks)
- **Mitigation:**
  - Hooks are explicitly configured by the user
  - Timeout enforcement
  - Cancellation support
  - Sandboxed via shell execution

### Session Recorder (Low Risk)
- **File:** `crates/sage-core/src/trajectory/session.rs`
- **Risk:** Low (reads git branch only)
- **Pattern:** `Command::new("git").args(["rev-parse", "--abbrev-ref", "HEAD"])`

### Diagnostics (Low Risk)
- **File:** `crates/sage-cli/src/commands/diagnostics/checks.rs`
- **Risk:** Low (internal diagnostics, hardcoded commands)

## Argument Validation Best Practices

For tools that accept user-provided arguments (container names, resource names, etc.):

### 1. Allowlist Pattern Matching
```rust
fn validate_resource_name(name: &str) -> Result<(), ToolError> {
    // Only allow alphanumeric, hyphens, and underscores
    if !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err(ToolError::InvalidArguments(
            "Resource name contains invalid characters".into()
        ));
    }
    Ok(())
}
```

### 2. Length Limits
```rust
fn validate_argument_length(arg: &str, max_len: usize) -> Result<(), ToolError> {
    if arg.len() > max_len {
        return Err(ToolError::InvalidArguments(
            format!("Argument too long (max {} chars)", max_len)
        ));
    }
    Ok(())
}
```

### 3. Shell Metacharacter Rejection
```rust
fn reject_shell_chars(arg: &str) -> Result<(), ToolError> {
    const DANGEROUS: &[char] = &['$', '`', '|', ';', '&', '>', '<', '\n', '\r'];
    if arg.chars().any(|c| DANGEROUS.contains(&c)) {
        return Err(ToolError::InvalidArguments(
            "Argument contains forbidden characters".into()
        ));
    }
    Ok(())
}
```

## Security Validation Module

The `sage-core/src/sandbox/validation/` module provides comprehensive checks:

| Check | Purpose | File |
|-------|---------|------|
| `check_heredoc_safety` | Detect heredoc injection | `heredoc_check.rs` |
| `check_shell_metacharacters` | Detect shell metachar abuse | `metacharacter_check.rs` |
| `check_dangerous_variables` | Detect variable injection | `variable_check.rs` |
| `check_dangerous_patterns` | Detect system destruction | `pattern_check.rs` |
| `check_dangerous_removal` | Detect critical path removal | `removal_check.rs` |

## Recommendations

### Implemented
- [x] Comprehensive validation for bash tool
- [x] CommandTool trait with whitelisting
- [x] Sandbox executor with resource limits
- [x] Input sanitizer for JSON inputs
- [x] Violation tracking and audit logging

### Future Enhancements
- [ ] Add argument validation helpers for infrastructure tools
- [ ] Consider namespace/project scoping for K8s/cloud tools
- [ ] Add audit logging for all command executions
- [ ] Implement rate limiting for command execution

## Conclusion

The command execution security in Sage Agent follows defense-in-depth principles:

1. **Shell commands** (high risk) have comprehensive validation
2. **Direct commands** (low risk) use safe argument separation
3. **User-configured hooks** are expected to be under user control
4. **Sandbox executor** applies resource limits

The codebase is considered secure for command execution when used as intended.
